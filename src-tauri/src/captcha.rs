use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex as PlMutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

pub const DISCORD_HCAPTCHA_SITE_KEY: &str = "4c672d35-0701-42b2-88c3-78380b0db560";

pub struct CaptchaSession {
    pub port: u16,
    pub sitekey: String,
    pub service: String,
    pub rqdata: Option<String>,
}

pub async fn start_bridge(
    title: String,
    sitekey: String,
    service: String,
    rqdata: Option<String>,
) -> Result<(CaptchaSession, oneshot::Receiver<String>), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener
        .local_addr()
        .map_err(|e| e.to_string())?
        .port();

    let (tx, rx) = oneshot::channel();
    let done = Arc::new(PlMutex::new(Some(tx)));
    let sitekey_out = sitekey.clone();
    let service_out = service.clone();
    let rqdata_out = rqdata.clone();

    tokio::spawn(async move {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(900);
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
            let title_ref = title.clone();
            let sk_ref = sitekey.clone();
            let svc_ref = service.clone();
            let rq_ref = rqdata.clone();
            tokio::spawn(async move {
                let _ = handle_conn(stream, done_ref, title_ref, sk_ref, svc_ref, rq_ref).await;
            });
            if done.lock().is_none() {
                break;
            }
        }
    });

    Ok((
        CaptchaSession {
            port,
            sitekey: sitekey_out,
            service: service_out,
            rqdata: rqdata_out,
        },
        rx,
    ))
}

async fn handle_conn(
    mut stream: tokio::net::TcpStream,
    done: Arc<PlMutex<Option<oneshot::Sender<String>>>>,
    title: String,
    sitekey: String,
    service: String,
    rqdata: Option<String>,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; 64 * 1024];
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
    let first = req.lines().next().unwrap_or("").to_string();

    if first.starts_with("GET /done") {
        respond(&mut stream, "text/html; charset=utf-8", ok_page()).await?;
    } else if first.starts_with("POST /solve") {
        let mut body = req
            .split("\r\n\r\n")
            .nth(1)
            .unwrap_or("")
            .to_string();
        let content_length = parse_content_length(&req).unwrap_or(0);
        while body.len() < content_length {
            let n = stream.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            body.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        let token = extract_token(&body).unwrap_or_default();
        if !token.is_empty() {
            if let Some(tx) = done.lock().take() {
                let _ = tx.send(token);
            }
        }
        respond(&mut stream, "text/plain", "ok".to_string()).await?;
    } else {
        respond(
            &mut stream,
            "text/html; charset=utf-8",
            captcha_page(&title, &sitekey, &service, rqdata.as_deref()),
        )
        .await?;
    }
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

fn extract_token(body: &str) -> Option<String> {
    if let Some(stripped) = body.strip_prefix("captcha_key=") {
        return Some(urldecode(stripped));
    }
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("captcha_key").and_then(|k| k.as_str()).map(String::from))
}

fn urldecode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(' '),
            b'%' if i + 2 < bytes.len() => {
                let hi = from_hex(bytes[i + 1]);
                let lo = from_hex(bytes[i + 2]);
                if let (Some(a), Some(b)) = (hi, lo) {
                    out.push(((a << 4) | b) as char);
                    i += 2;
                } else {
                    out.push('%');
                }
            }
            c => out.push(c as char),
        }
        i += 1;
    }
    out
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

async fn respond(
    stream: &mut tokio::net::TcpStream,
    ct: &str,
    body: String,
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        ct,
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    stream.shutdown().await.ok();
    Ok(())
}

fn captcha_page(title: &str, sitekey: &str, service: &str, rqdata: Option<&str>) -> String {
    let effective_sitekey = if sitekey.is_empty() { DISCORD_HCAPTCHA_SITE_KEY } else { sitekey };
    if service == "recaptcha" {
        return recaptcha_page(title, effective_sitekey);
    }
    let rqdata_js = rqdata
        .filter(|s| !s.is_empty())
        .map(|s| format!(r#""{}""#, s.replace('\\', "\\\\").replace('"', "\\\"")))
        .unwrap_or_else(|| "null".into());

    format!(
        r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8"/>
  <title>botcord · solve captcha</title>
  <script src="https://js.hcaptcha.com/1/api.js?onload=onHcapLoaded&render=explicit" async defer></script>
  <style>
    :root {{ color-scheme: dark; }}
    body {{ background:#000; color:#f5f5f5; font-family: "JetBrains Mono", ui-monospace, monospace; margin:0; display:grid; place-items:center; height:100vh; letter-spacing:-0.005em; }}
    .card {{ border:1px solid #2a2a2a; background:#0a0a0a; padding:24px; min-width:320px; max-width:90vw; display:flex; flex-direction:column; gap:16px; align-items:center; }}
    .hint {{ font-size:10px; text-transform:uppercase; letter-spacing:0.22em; color:#5e5e5e; }}
    .title {{ font-size:14px; font-weight:600; align-self:flex-start; }}
    .ok {{ color:#7aff7a; font-size:11px; text-transform:uppercase; letter-spacing:0.22em; }}
    .err {{ color:#ff5555; font-size:11px; text-transform:uppercase; letter-spacing:0.22em; }}
  </style>
</head>
<body>
  <div class="card">
    <div class="title">botcord · {title}</div>
    <div class="hint">hcaptcha · enterprise: {enterprise}</div>
    <div id="cap"></div>
    <div id="status" class="hint">loading hcaptcha...</div>
  </div>
  <script>
    const SITEKEY = "{sitekey}";
    const RQDATA = {rqdata_js};
    let widgetId = null;

    function setStatus(msg, cls) {{
      const el = document.getElementById('status');
      el.textContent = msg;
      el.className = cls || 'hint';
    }}

    window.onHcapLoaded = function() {{
      widgetId = hcaptcha.render('cap', {{
        sitekey: SITEKEY,
        size: 'invisible',
        callback: 'onSolved',
        'error-callback': 'onErr',
        'expired-callback': 'onErr',
        'chalexpired-callback': 'onErr',
      }});
      setStatus('requesting challenge...');
      try {{
        if (RQDATA) {{
          hcaptcha.execute(widgetId, {{ rqdata: RQDATA, sentry: true, async: false }});
        }} else {{
          hcaptcha.execute(widgetId);
        }}
      }} catch (e) {{
        setStatus('execute err: ' + e, 'err');
      }}
    }};

    window.onSolved = function(token) {{
      setStatus('submitting...');
      fetch('/solve', {{
        method: 'POST',
        headers: {{ 'content-type': 'application/x-www-form-urlencoded' }},
        body: 'captcha_key=' + encodeURIComponent(token),
      }}).then(r => r.text()).then(() => {{
        setStatus('got it. go back to botcord.', 'ok');
      }}).catch(e => {{
        setStatus('post err: ' + e, 'err');
      }});
    }};

    window.onErr = function(err) {{
      setStatus('captcha err: ' + (err || 'retry'), 'err');
    }};
  </script>
</body>
</html>"#,
        title = title,
        sitekey = effective_sitekey,
        enterprise = if rqdata.is_some() { "yes" } else { "no" },
        rqdata_js = rqdata_js,
    )
}

fn recaptcha_page(title: &str, sitekey: &str) -> String {
    format!(
        r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8"/>
  <title>botcord · solve recaptcha</title>
  <script src="https://www.google.com/recaptcha/api.js" async defer></script>
  <style>
    :root {{ color-scheme: dark; }}
    body {{ background:#000; color:#f5f5f5; font-family: "JetBrains Mono", monospace; margin:0; display:grid; place-items:center; height:100vh; }}
    .card {{ border:1px solid #2a2a2a; background:#0a0a0a; padding:24px; display:flex; flex-direction:column; gap:16px; align-items:center; }}
    .hint {{ font-size:10px; text-transform:uppercase; letter-spacing:0.22em; color:#5e5e5e; }}
    .title {{ font-size:14px; font-weight:600; align-self:flex-start; }}
  </style>
</head>
<body>
  <div class="card">
    <div class="title">botcord · {title}</div>
    <div class="g-recaptcha" data-sitekey="{sitekey}" data-callback="onSolved"></div>
    <div id="status" class="hint">waiting for solve...</div>
  </div>
  <script>
    window.onSolved = function(token) {{
      fetch('/solve', {{
        method: 'POST',
        headers: {{ 'content-type': 'application/x-www-form-urlencoded' }},
        body: 'captcha_key=' + encodeURIComponent(token),
      }}).then(() => {{
        document.getElementById('status').textContent = 'got it. go back to botcord.';
      }});
    }};
  </script>
</body>
</html>"#,
        title = title,
        sitekey = sitekey,
    )
}

fn ok_page() -> String {
    r#"<!doctype html><title>ok</title><body style="background:#000;color:#7aff7a;font-family:monospace;text-align:center;padding-top:40vh">ok, return to botcord</body>"#.to_string()
}
