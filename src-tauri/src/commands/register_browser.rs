use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use tokio::process::Command;
use uuid::Uuid;

use crate::commands::accounts::AccountRow;
use crate::discord::api::fetch_me;
use crate::discord::identity::Identity;
use crate::state::{require_key, AppState};
use crate::storage::crypto::encrypt_field;

#[derive(Deserialize, Debug)]
pub struct BrowserRegisterArgs {
    pub email: String,
    pub password: String,
    pub username: String,
    pub date_of_birth: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub use_proxy: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct BrowserRegisterResult {
    pub ok: bool,
    pub account: Option<AccountRow>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn accounts_register_via_browser(
    app: tauri::AppHandle,
    args: BrowserRegisterArgs,
    state: State<'_, AppState>,
) -> Result<BrowserRegisterResult, String> {
    let _ = require_key(&state)?;

    let (bridge, rx) = crate::register_bridge::start_token_bridge().await?;
    let port = bridge.port;

    let profile = make_profile_dir(&app)?;
    let ext = write_autofill_extension(&profile, &args, port)?;

    let chromium = resolve_chromium(&app).map_err(|e| e)?;
    let pb = find_privacy_badger(&app);

    let mut load_exts: Vec<String> = Vec::new();
    if let Some(p) = pb {
        load_exts.push(p.to_string_lossy().to_string());
    }
    load_exts.push(ext.to_string_lossy().to_string());

    let mut cmd = Command::new(&chromium);
    cmd.arg(format!("--user-data-dir={}", profile.display()));
    cmd.arg(format!("--load-extension={}", load_exts.join(",")));
    cmd.arg("--no-first-run");
    cmd.arg("--no-default-browser-check");
    cmd.arg("--disable-sync");
    cmd.arg("--disable-background-networking");
    cmd.arg("--metrics-recording-only");
    cmd.arg("--no-pings");
    cmd.arg("--disable-breakpad");
    cmd.arg("https://discord.com/register");
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let child = cmd.spawn().map_err(|e| format!("spawn chromium: {e}"))?;

    let token = match tokio::time::timeout(std::time::Duration::from_secs(1500), rx).await {
        Ok(Ok(t)) => t,
        Ok(Err(_)) => {
            cleanup(child, &profile);
            return Ok(BrowserRegisterResult {
                ok: false,
                account: None,
                error: Some("bridge dropped before token".into()),
            });
        }
        Err(_) => {
            cleanup(child, &profile);
            return Ok(BrowserRegisterResult {
                ok: false,
                account: None,
                error: Some("timeout waiting for register (25 min)".into()),
            });
        }
    };

    cleanup(child, &profile);

    let identity = Identity::generate();
    let proxy = if args.use_proxy {
        crate::commands::accounts::pick_alive_proxy(&state).await?
    } else {
        None
    };

    let me = fetch_me(&token, &identity, proxy.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let key = require_key(&state)?;
    let id = Uuid::new_v4().to_string();
    let (ct, nonce) = encrypt_field(&key, token.as_bytes());
    let meta = serde_json::json!({
        "global_name": me.global_name,
        "avatar": me.avatar,
        "premium_type": me.premium_type,
        "identity": identity,
        "registered": true,
        "email": args.email,
        "registered_via": "browser",
    });
    let meta_s = meta.to_string();
    let now = unix_now();
    let discord_id = me.id.clone();
    let label = me.username.clone();
    let db = state.db.clone();
    let id_cp = id.clone();
    let discord_id_cp = discord_id.clone();
    let label_cp = label.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute(
                "insert into accounts(id,discord_id,label,token_enc,token_nonce,valid,last_check_at,meta_json)
                 values(?1,?2,?3,?4,?5,1,?6,?7)",
                params![id_cp, discord_id_cp, label_cp, ct, nonce, now, meta_s],
            )?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    if args.use_proxy {
        let _ = crate::commands::proxies::proxies_assign_auto(state.clone()).await;
    }

    Ok(BrowserRegisterResult {
        ok: true,
        account: Some(AccountRow {
            id,
            discord_id: Some(discord_id),
            label: Some(label),
            global_name: me.global_name,
            avatar: me.avatar,
            premium_type: me.premium_type,
            proxy_id: None,
            valid: Some(true),
            last_check_at: Some(now),
        }),
        error: None,
    })
}

fn cleanup(mut child: tokio::process::Child, profile: &Path) {
    let _ = child.start_kill();
    let p = profile.to_path_buf();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let _ = std::fs::remove_dir_all(&p);
    });
}

fn make_profile_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("register-profiles").join(Uuid::new_v4().to_string());
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn write_autofill_extension(
    profile: &Path,
    args: &BrowserRegisterArgs,
    port: u16,
) -> Result<PathBuf, String> {
    let dir = profile.join("ext-autofill");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let manifest = serde_json::json!({
        "manifest_version": 3,
        "name": "botcord register autofill",
        "version": "1.0.0",
        "description": "fills discord register form + captures token",
        "host_permissions": [
            "https://discord.com/*",
            "http://127.0.0.1/*"
        ],
        "content_scripts": [{
            "matches": ["https://discord.com/*", "https://*.discord.com/*"],
            "js": ["content.js"],
            "run_at": "document_start",
            "world": "MAIN",
            "all_frames": false
        }]
    });
    fs::write(dir.join("manifest.json"), manifest.to_string()).map_err(|e| e.to_string())?;

    let email = js_escape(&args.email);
    let password = js_escape(&args.password);
    let username = js_escape(&args.username);
    let display_name = js_escape(
        args.display_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("1337"),
    );

    let content = format!(
        r#"(function(){{
  var CREDS = {{
    email: "{email}",
    password: "{password}",
    username: "{username}",
    display_name: "{display_name}",
  }};
  var BRIDGE = "http://127.0.0.1:{port}";
  var sent = false;

  function log() {{ try {{ console.log.apply(console, ["[botcord]"].concat([].slice.call(arguments))); }} catch (e) {{}} }}

  function postToken(t) {{
    if (sent || !t) return;
    sent = true;
    var clean = t;
    if (clean.length > 1 && clean[0] === '"' && clean[clean.length-1] === '"') clean = clean.slice(1, -1);
    log("posting token len=" + clean.length);
    fetch(BRIDGE + "/token", {{
      method: "POST",
      mode: "cors",
      headers: {{ "content-type": "application/json" }},
      body: JSON.stringify({{ token: clean }}),
    }}).then(function(){{ log("token posted"); }}).catch(function(e){{ log("post err", String(e)); sent = false; }});
  }}

  try {{
    var origFetch = window.fetch;
    window.fetch = function() {{
      var p = origFetch.apply(this, arguments);
      try {{
        var u = arguments[0];
        var url = typeof u === "string" ? u : (u && u.url) || "";
        if (url.indexOf("/auth/register") !== -1 || url.indexOf("/auth/login") !== -1) {{
          p.then(function(r){{
            try {{ r.clone().json().then(function(b){{ if (b && b.token) postToken(b.token); }}); }} catch(e) {{}}
          }}, function(){{}});
        }}
      }} catch (e) {{}}
      return p;
    }};

    var XHR_OPEN = XMLHttpRequest.prototype.open;
    var XHR_SEND = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function(m, u) {{
      this.__botcord_url = u;
      return XHR_OPEN.apply(this, arguments);
    }};
    XMLHttpRequest.prototype.send = function() {{
      var self = this;
      var u = self.__botcord_url || "";
      if (u.indexOf("/auth/register") !== -1 || u.indexOf("/auth/login") !== -1) {{
        self.addEventListener("load", function(){{
          try {{ var b = JSON.parse(self.responseText || "null"); if (b && b.token) postToken(b.token); }} catch (e) {{}}
        }});
      }}
      return XHR_SEND.apply(this, arguments);
    }};
  }} catch (e) {{ log("hook err", String(e)); }}

  function setReactValue(el, value) {{
    try {{
      var proto = Object.getPrototypeOf(el);
      var desc = Object.getOwnPropertyDescriptor(proto, "value");
      if (desc && desc.set) desc.set.call(el, value); else el.value = value;
      el.dispatchEvent(new Event("input", {{ bubbles: true }}));
      el.dispatchEvent(new Event("change", {{ bubbles: true }}));
      el.dispatchEvent(new Event("blur", {{ bubbles: true }}));
    }} catch (e) {{}}
  }}

  function clickConsent() {{
    try {{
      var cb = document.querySelector('input[type="checkbox"]');
      if (cb && !cb.checked) {{
        var label = cb.closest('label');
        if (label) label.click();
        else cb.click();
      }}
    }} catch (e) {{}}
  }}

  function attempt() {{
    try {{
      var t = localStorage.getItem("token");
      if (t) postToken(t);
    }} catch (e) {{}}

    var email = document.querySelector('input[name="email"]');
    var user  = document.querySelector('input[name="username"]');
    var pwd   = document.querySelector('input[name="password"]');
    var gn    = document.querySelector('input[name="global_name"]');
    if (email && !email.value) setReactValue(email, CREDS.email);
    if (user && !user.value)   setReactValue(user, CREDS.username);
    if (pwd && !pwd.value)     setReactValue(pwd, CREDS.password);
    if (gn && !gn.value)       setReactValue(gn, CREDS.display_name);
    clickConsent();
  }}

  var i = setInterval(attempt, 500);
  setTimeout(function(){{ clearInterval(i); }}, 60000 * 25);
  attempt();
  log("content loaded world=" + (typeof chrome === "undefined" ? "main" : "hybrid"));
}})();
"#
    );
    fs::write(dir.join("content.js"), content).map_err(|e| e.to_string())?;

    Ok(dir)
}

fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn resolve_chromium(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    crate::commands::browser::resolve_chromium_public(app)
}

fn find_privacy_badger(app: &tauri::AppHandle) -> Option<PathBuf> {
    crate::commands::browser::find_privacy_badger_public(app)
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
