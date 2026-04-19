use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use rusqlite::params;
use serde::Serialize;
use tauri::{Manager, State};
use tokio::process::Command;

use crate::commands::proxies::load_proxy_def;
use crate::discord::http_client::ProxyDef;
use crate::state::{require_key, AppState};
use crate::storage::crypto::decrypt_field;

#[derive(Serialize, Clone, Debug)]
pub struct BrowserOpenResult {
    pub profile_dir: String,
    pub chromium_path: String,
    pub privacy_badger: bool,
}

#[tauri::command]
pub async fn browser_open(
    app: tauri::AppHandle,
    account_id: String,
    state: State<'_, AppState>,
) -> Result<BrowserOpenResult, String> {
    let key = require_key(&state)?;
    let db = state.db.clone();
    let aid = account_id.clone();
    let row = tokio::task::spawn_blocking(move || -> rusqlite::Result<Option<(Vec<u8>, Vec<u8>, Option<String>)>> {
        db.with(|c| {
            let mut s = c.prepare(
                "select token_enc, token_nonce, proxy_id from accounts where id = ?1",
            )?;
            let r = s.query_row(params![aid], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, Option<String>>(2)?))
            }).ok();
            Ok(r)
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let (ct, nonce, proxy_id) = row.ok_or("account not found")?;
    let token_bytes = decrypt_field(&key, &ct, &nonce).map_err(|e| e.to_string())?;
    let token = String::from_utf8(token_bytes).map_err(|e| e.to_string())?;

    let proxy = match proxy_id {
        Some(pid) => load_proxy_def(&state, &pid).await?,
        None => None,
    };

    let profile = profile_dir(&app, &account_id)?;
    fs::create_dir_all(&profile).map_err(|e| e.to_string())?;

    let bootstrap = write_bootstrap_ext(&profile, &token)?;
    let proxy_auth = if let Some(p) = &proxy {
        if p.user.is_some() && p.pass.is_some() {
            Some(write_proxy_auth_ext(&profile, p)?)
        } else {
            None
        }
    } else {
        None
    };

    let chromium = resolve_chromium(&app)?;
    let privacy_badger = find_privacy_badger(&app);

    let mut ext_paths: Vec<PathBuf> = Vec::new();
    if let Some(pb) = &privacy_badger {
        ext_paths.push(pb.clone());
    }
    ext_paths.push(bootstrap);
    if let Some(pa) = proxy_auth {
        ext_paths.push(pa);
    }

    let load_ext = ext_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(",");

    let mut cmd = Command::new(&chromium);
    cmd.arg(format!("--user-data-dir={}", profile.display()));
    cmd.arg(format!("--load-extension={load_ext}"));
    cmd.arg("--no-first-run");
    cmd.arg("--no-default-browser-check");
    cmd.arg("--disable-sync");
    cmd.arg("--disable-background-networking");
    cmd.arg("--metrics-recording-only");
    cmd.arg("--no-pings");
    cmd.arg("--disable-breakpad");
    if let Some(p) = &proxy {
        cmd.arg(format!("--proxy-server={}://{}:{}", p.scheme, p.host, p.port));
    }
    cmd.arg("https://discord.com/app");
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());

    cmd.spawn().map_err(|e| format!("spawn chromium: {e}"))?;

    Ok(BrowserOpenResult {
        profile_dir: profile.to_string_lossy().to_string(),
        chromium_path: chromium.to_string_lossy().to_string(),
        privacy_badger: privacy_badger.is_some(),
    })
}

#[tauri::command]
pub async fn browser_wipe(
    app: tauri::AppHandle,
    account_id: String,
) -> Result<(), String> {
    let dir = profile_dir(&app, &account_id)?;
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn profile_dir(app: &tauri::AppHandle, account_id: &str) -> Result<PathBuf, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(base.join("profiles").join(account_id))
}

fn write_bootstrap_ext(profile: &Path, token: &str) -> Result<PathBuf, String> {
    let dir = profile.join("ext-bootstrap")   ;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let manifest = serde_json::json!({
        "manifest_version": 3,
        "name": "botcord bootstrap",
        "version": "1.0.0",
        "description": "auto-login helper",
        "content_scripts": [{
            "matches": ["https://discord.com/*", "https://*.discord.com/*"],
            "js": ["bootstrap.js"],
            "run_at": "document_start",
            "world": "MAIN"
        }]
    });
    fs::write(dir.join("manifest.json"), manifest.to_string()).map_err(|e| e.to_string())?;

    let escaped = token.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        r#"(function() {{
  try {{
    const t = "{escaped}";
    if (!localStorage.getItem('token')) {{
      const old = window.localStorage;
      const v = 'token';
      try {{
        Object.defineProperty(window, 'localStorage', {{ value: old, configurable: true }});
      }} catch (e) {{}}
      old.setItem(v, '"' + t + '"');
      setTimeout(() => location.reload(), 100);
    }}
  }} catch (e) {{ console.error('bootstrap', e); }}
}})();
"#
    );
    fs::write(dir.join("bootstrap.js"), script).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn write_proxy_auth_ext(profile: &Path, proxy: &ProxyDef) -> Result<PathBuf, String> {
    let dir = profile.join("ext-proxy-auth");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let manifest = serde_json::json!({
        "manifest_version": 3,
        "name": "botcord proxy auth",
        "version": "1.0.0",
        "background": { "service_worker": "bg.js", "type": "module" },
        "permissions": ["webRequest", "webRequestAuthProvider"],
        "host_permissions": ["<all_urls>"]
    });
    fs::write(dir.join("manifest.json"), manifest.to_string()).map_err(|e| e.to_string())?;

    let user = proxy.user.clone().unwrap_or_default().replace('\\', "\\\\").replace('"', "\\\"");
    let pass = proxy.pass.clone().unwrap_or_default().replace('\\', "\\\\").replace('"', "\\\"");
    let bg = format!(
        r#"chrome.webRequest.onAuthRequired.addListener(
  () => ({{ authCredentials: {{ username: "{user}", password: "{pass}" }} }}),
  {{ urls: ["<all_urls>"] }},
  ["blocking"]
);
"#
    );
    fs::write(dir.join("bg.js"), bg).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn resolve_chromium(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("BOTCORD_CHROMIUM") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Ok(pb);
        }
    }
    if let Ok(dir) = app.path().resource_dir() {
        let plat = platform_subdir();
        let exe = chromium_exe_name();
        let candidate = dir.join("chromium").join(plat).join(exe);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let plat = platform_subdir();
        let exe = chromium_exe_name();
        let candidate = PathBuf::from(manifest)
            .join("resources")
            .join("chromium")
            .join(plat)
            .join(exe);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    for fallback in system_chrome_candidates() {
        if fallback.exists() {
            return Ok(fallback);
        }
    }
    Err("chromium not found. Put it in src-tauri/resources/chromium/<platform>/ or set BOTCORD_CHROMIUM env var.".into())
}

fn find_privacy_badger(app: &tauri::AppHandle) -> Option<PathBuf> {
    let rel = Path::new("extensions").join("privacy-badger");
    if let Ok(dir) = app.path().resource_dir() {
        let p = dir.join(&rel);
        if p.join("manifest.json").exists() {
            return Some(p);
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("resources").join(&rel);
        if p.join("manifest.json").exists() {
            return Some(p);
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn platform_subdir() -> &'static str {
    "win-x64"
}
#[cfg(target_os = "macos")]
fn platform_subdir() -> &'static str {
    "mac"
}
#[cfg(target_os = "linux")]
fn platform_subdir() -> &'static str {
    "linux"
}

#[cfg(target_os = "windows")]
fn chromium_exe_name() -> &'static str {
    "chrome.exe"
}
#[cfg(not(target_os = "windows"))]
fn chromium_exe_name() -> &'static str {
    "chrome"
}

#[cfg(target_os = "windows")]
fn system_chrome_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(pf) = std::env::var("ProgramFiles") {
        v.push(PathBuf::from(pf).join("Google/Chrome/Application/chrome.exe"));
    }
    if let Ok(pf) = std::env::var("ProgramFiles(x86)") {
        v.push(PathBuf::from(pf).join("Google/Chrome/Application/chrome.exe"));
    }
    if let Ok(la) = std::env::var("LOCALAPPDATA") {
        v.push(PathBuf::from(la).join("Google/Chrome/Application/chrome.exe"));
    }
    v
}

#[cfg(target_os = "macos")]
fn system_chrome_candidates() -> Vec<PathBuf> {
    vec![PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")]
}

#[cfg(target_os = "linux")]
fn system_chrome_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/bin/google-chrome"),
        PathBuf::from("/usr/bin/chromium"),
    ]
}
