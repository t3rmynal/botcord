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

    pub fn cwd(&self) -> &std::path::Path {
        &self.cwd
    }

    pub fn script_path(&self) -> &std::path::Path {
        &self.script_path
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
        tracing::warn!(
            target: "voice-sidecar",
            "spawn: cwd={:?} script={:?}",
            self.cwd,
            self.script_path
        );
        if !self.cwd.exists() {
            let msg = format!("voice sidecar dir not found: {}", self.cwd.display());
            tracing::warn!(target: "voice-sidecar", "{}", msg);
            return Err(msg);
        }
        if !self.script_path.exists() {
            let msg = format!("voice sidecar script missing: {}", self.script_path.display());
            tracing::warn!(target: "voice-sidecar", "{}", msg);
            return Err(msg);
        }
        let script_str = self.script_path.to_string_lossy().to_string();
        tracing::warn!(
            target: "voice-sidecar",
            "invoking: node {:?} (cwd {:?})",
            script_str,
            self.cwd
        );
        let mut child = Command::new("node")
            .arg(&script_str)
            .current_dir(&self.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                let msg = format!("spawn node: {e}. is node 20+ on PATH?");
                tracing::warn!(target: "voice-sidecar", "{}", msg);
                msg
            })?;
        tracing::warn!(target: "voice-sidecar", "child spawned, pid={:?}", child.id());

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

        let timeout_secs = match method {
            "join" => 90,
            "leave" => 15,
            _ => 30,
        };
        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await {
            Ok(Ok(res)) => res,
            Ok(Err(_)) => Err("sidecar dropped".into()),
            Err(_) => {
                self.pending.lock().remove(&id);
                Err(format!("sidecar timeout after {timeout_secs}s"))
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
    for cand in sidecar_candidates(app) {
        if cand.join("index.js").exists() {
            return cand;
        }
    }
    let all = sidecar_candidates(app);
    tracing::warn!(target: "voice-sidecar", "no index.js found, tried: {:?}", all);
    all.into_iter().next().unwrap_or_else(|| PathBuf::from("."))
}

fn sidecar_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(dir) = app.path().resource_dir() {
        let dir = dunce::simplified(&dir).to_path_buf();
        out.push(dir.join("resources").join("sidecars").join("voice"));
        out.push(dir.join("sidecars").join("voice"));
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe = dunce::simplified(&exe).to_path_buf();
        if let Some(parent) = exe.parent() {
            out.push(parent.join("resources").join("sidecars").join("voice"));
            out.push(parent.join("sidecars").join("voice"));
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        out.push(PathBuf::from(manifest).join("sidecars").join("voice"));
    }
    if let Ok(mut here) = std::env::current_exe() {
        here = dunce::simplified(&here).to_path_buf();
        for _ in 0..4 {
            here.pop();
            out.push(here.join("src-tauri").join("sidecars").join("voice"));
        }
    }
    out
}
