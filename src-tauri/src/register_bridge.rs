use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex as PlMutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

pub struct TokenBridge {
    pub port: u16,
}

pub async fn start_token_bridge() -> Result<(TokenBridge, oneshot::Receiver<String>), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let (tx, rx) = oneshot::channel();
    let done = Arc::new(PlMutex::new(Some(tx)));

    tokio::spawn(async move {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1800);
        loop {
            let remain = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remain.is_zero() {
                break;
            }
            let accept = tokio::time::timeout(remain, listener.accept()).await;
            let stream = match accept {
                Ok(Ok((s, _))) => s,
                _ => break,
            };
            let done_ref = done.clone();
            tokio::spawn(async move {
                let _ = handle_conn(stream, done_ref).await;
            });
            if done.lock().is_none() {
                break;
            }
        }
    });

    Ok((TokenBridge { port }, rx))
}

async fn handle_conn(
    mut stream: tokio::net::TcpStream,
    done: Arc<PlMutex<Option<oneshot::Sender<String>>>>,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; 32 * 1024];
    let mut used = 0usize;
    loop {
        let n = stream.read(&mut buf[used..]).await?;
        if n == 0 {
            break;
        }
        used += n;
        if used >= 4 && buf[..used].windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if used >= buf.len() {
            break;
        }
    }
    let req = String::from_utf8_lossy(&buf[..used]).to_string();

    let cors_ok = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: *\r\nAccess-Control-Allow-Methods: POST, OPTIONS, GET\r\nConnection: close\r\n\r\n";

    if req.starts_with("OPTIONS ") {
        stream.write_all(cors_ok.as_bytes()).await?;
        stream.shutdown().await.ok();
        return Ok(());
    }

    if req.starts_with("POST /token") {
        let mut body = req.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        let content_length = parse_content_length(&req).unwrap_or(0);
        while body.len() < content_length {
            let n = stream.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            body.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        let token = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("token").and_then(|t| t.as_str()).map(String::from))
            .unwrap_or_default();
        if !token.is_empty() {
            if let Some(tx) = done.lock().take() {
                let _ = tx.send(token);
            }
        }
        let resp = "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
        stream.write_all(resp.as_bytes()).await?;
        stream.shutdown().await.ok();
        return Ok(());
    }

    let html = r#"<!doctype html><title>botcord</title><style>body{background:#000;color:#aaa;font-family:monospace;padding:40px;text-align:center}</style><body>bridge alive. use chromium with the autofill extension.</body>"#;
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    );
    stream.write_all(resp.as_bytes()).await?;
    stream.shutdown().await.ok();
    Ok(())
}

fn parse_content_length(req: &str) -> Option<usize> {
    for line in req.lines() {
        let l = line.to_ascii_lowercase();
        if let Some(rest) = l.strip_prefix("content-length:") {
            return rest.trim().parse().ok();
        }
    }
    None
}
