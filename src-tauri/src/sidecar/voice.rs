use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use parking_lot::Mutex as PlMutex;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, Mutex as AsyncMutex};

pub type Pending = Arc<PlMutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

pub struct VoiceSidecar {
    child: AsyncMutex<Option<Child>>,
    stdin: AsyncMutex<Option<ChildStdin>>,
    next_id: AtomicCounter,
    pending: Pending,
    script_path: PathBuf,
    cwd: PathBuf,
    app: AppHandle,
}

pub struct AtomicCounter(std::sync::atomic::AtomicU64);

impl AtomicCounter {
    fn new() -> Self {
        Self(std::sync::atomic::AtomicU64::new(1))
    }
    fn next(&self) -> u64 {
        self.0
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

impl VoiceSidecar {
    pub fn new(app: AppHandle) -> Self {
        let cwd = resolve_sidecar_dir(&app);
        let script_path = cwd.join("index.js");
        Self {
            child: AsyncMutex::new(None),
            stdin: AsyncMutex::new(None),
            next_id: AtomicCounter::new(),
            pending: Arc::new(PlMutex::new(HashMap::new())),
            script_path,
            cwd,
            app,
        }
    }

    pub async fn ensure_running(&self) -> Result<(), String> {
        {
            let c = self.child.lock().await;
            if c.is_some() {
                return Ok(());
            }
        }
        self.spawn().await
    }

    async fn spawn(&self) -> Result<(), String> {
        let mut child = Command::new("node")
            .arg(&self.script_path)
            .current_dir(&self.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn node: {e}"))?;

        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let stderr = child.stderr.take().ok_or("no stderr")?;

        let pending = self.pending.clone();
        let app = self.app.clone();

        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                handle_line(&line, &pending, &app);
            }
        });

        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(target: "voice-sidecar", "{}", line);
            }
        });

        *self.child.lock().await = Some(child);
        *self.stdin.lock().await = Some(stdin);
        Ok(())
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        self.ensure_running().await?;
        let id = self.next_id.next();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, tx);

        let msg = json!({ "id": id, "method": method, "params": params });
        let payload = format!("{}\n", msg);

        {
            let mut guard = self.stdin.lock().await;
            let stdin = guard.as_mut().ok_or("no stdin")?;
            stdin
                .write_all(payload.as_bytes())
                .await
                .map_err(|e| e.to_string())?;
        }

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(res)) => res,
            Ok(Err(_)) => Err("sidecar dropped".into()),
            Err(_) => {
                self.pending.lock().remove(&id);
                Err("sidecar timeout".into())
            }
        }
    }
}

fn handle_line(line: &str, pending: &Pending, app: &AppHandle) {
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return,
    };
    if let Some(id) = v.get("id").and_then(|x| x.as_u64()) {
        let mut p = pending.lock();
        if let Some(tx) = p.remove(&id) {
            if let Some(err) = v.get("error") {
                let _ = tx.send(Err(err.as_str().unwrap_or("error").to_string()));
            } else {
                let _ = tx.send(Ok(v.get("result").cloned().unwrap_or(Value::Null)));
            }
        }
        return;
    }
    if let Some(evt) = v.get("event").and_then(|x| x.as_str()) {
        let payload = v.get("payload").cloned().unwrap_or(Value::Null);
        let ch = format!("voice:{evt}");
        let _ = app.emit(&ch, payload);
    }
}

fn resolve_sidecar_dir(app: &AppHandle) -> PathBuf {
    if let Ok(dir) = app.path().resource_dir() {
        let p = dir.join("sidecars").join("voice");
        if p.join("index.js").exists() {
            return p;
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("sidecars").join("voice");
        if p.join("index.js").exists() {
            return p;
        }
    }
    let mut here = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..4 {
        here.pop();
        let p = here.join("src-tauri").join("sidecars").join("voice");
        if p.join("index.js").exists() {
            return p;
        }
    }
    PathBuf::from("sidecars/voice")
}
