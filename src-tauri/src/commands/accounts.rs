use std::time::Duration;

use rand::Rng;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::discord::api::{fetch_me, ApiError};
use crate::discord::http_client::ProxyDef;
use crate::discord::identity::Identity;
use crate::state::{require_key, AppState};
use crate::storage::crypto::{decrypt_field, encrypt_field};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccountRow {
    pub id: String,
    pub discord_id: Option<String>,
    pub label: Option<String>,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub premium_type: Option<u64>,
    pub proxy_id: Option<String>,
    pub valid: Option<bool>,
    pub last_check_at: Option<i64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct CheckItem {
    pub token_tail: String,
    pub ok: bool,
    pub id: Option<String>,
    pub username: Option<String>,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub premium_type: Option<u64>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn accounts_list(state: State<'_, AppState>) -> Result<Vec<AccountRow>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || list_rows(&db))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

fn list_rows(db: &crate::storage::db::Db) -> rusqlite::Result<Vec<AccountRow>> {
    db.with(|c| {
        let mut s = c.prepare(
            "select id, discord_id, label, proxy_id, valid, last_check_at, meta_json
             from accounts order by created_at asc",
        )?;
        let rows = s.query_map([], |r| {
            let id: String = r.get(0)?;
            let discord_id: Option<String> = r.get(1)?;
            let label: Option<String> = r.get(2)?;
            let proxy_id: Option<String> = r.get(3)?;
            let valid_i: Option<i64> = r.get(4)?;
            let last_check_at: Option<i64> = r.get(5)?;
            let meta: Option<String> = r.get(6)?;
            let (global_name, avatar, premium_type) = extract_meta(meta.as_deref());
            Ok(AccountRow {
                id,
                discord_id,
                label,
                global_name,
                avatar,
                premium_type,
                proxy_id,
                valid: valid_i.map(|v| v != 0),
                last_check_at,
            })
        })?;
        rows.collect()
    })
}

fn extract_meta(meta: Option<&str>) -> (Option<String>, Option<String>, Option<u64>) {
    let Some(m) = meta else { return (None, None, None) };
    let v: serde_json::Value = serde_json::from_str(m).unwrap_or(serde_json::Value::Null);
    let gn = v.get("global_name").and_then(|x| x.as_str()).map(String::from);
    let av = v.get("avatar").and_then(|x| x.as_str()).map(String::from);
    let pt = v.get("premium_type").and_then(|x| x.as_u64());
    (gn, av, pt)
}

#[tauri::command]
pub async fn accounts_check(
    tokens: Vec<String>,
    _state: State<'_, AppState>,
) -> Result<Vec<CheckItem>, String> {
    Ok(check_tokens(tokens).await)
}

async fn check_tokens(tokens: Vec<String>) -> Vec<CheckItem> {
    let sem = std::sync::Arc::new(Semaphore::new(3));
    let mut handles = Vec::new();
    for raw in tokens {
        let t = raw.trim().to_string();
        if t.is_empty() {
            continue;
        }
        let sem = sem.clone();
        let identity = Identity::generate();
        handles.push(tokio::spawn(async move {
            let _p = sem.acquire_owned().await.unwrap();
            let delay_ms = rand::thread_rng().gen_range(120..=600);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            check_one(t, identity, None).await
        }));
    }
    let mut out = Vec::new();
    for h in handles {
        match h.await {
            Ok(item) => out.push(item),
            Err(e) => out.push(CheckItem {
                token_tail: "???".into(),
                ok: false,
                id: None,
                username: None,
                global_name: None,
                avatar: None,
                premium_type: None,
                error: Some(e.to_string()),
            }),
        }
    }
    out
}

async fn check_one(token: String, identity: Identity, proxy: Option<ProxyDef>) -> CheckItem {
    let tail = tail(&token);
    match fetch_me(&token, &identity, proxy.as_ref()).await {
        Ok(u) => CheckItem {
            token_tail: tail,
            ok: true,
            id: Some(u.id),
            username: Some(u.username),
            global_name: u.global_name,
            avatar: u.avatar,
            premium_type: u.premium_type,
            error: None,
        },
        Err(ApiError::Unauthorized) => CheckItem {
            token_tail: tail,
            ok: false,
            id: None,
            username: None,
            global_name: None,
            avatar: None,
            premium_type: None,
            error: Some("invalid token".into()),
        },
        Err(e) => CheckItem {
            token_tail: tail,
            ok: false,
            id: None,
            username: None,
            global_name: None,
            avatar: None,
            premium_type: None,
            error: Some(e.to_string()),
        },
    }
}

fn tail(token: &str) -> String {
    let n = token.len();
    if n <= 6 {
        return token.to_string();
    }
    format!("...{}", &token[n - 6..])
}

#[derive(Serialize, Clone, Debug)]
pub struct AddResult {
    pub added: Vec<AccountRow>,
    pub skipped: Vec<String>,
}

#[tauri::command]
pub async fn accounts_add(
    tokens: Vec<String>,
    state: State<'_, AppState>,
) -> Result<AddResult, String> {
    let key = require_key(&state)?;
    let db = state.db.clone();
    let items = check_tokens(tokens.clone()).await;

    let mut added = Vec::new();
    let mut skipped = Vec::new();

    for (tok, item) in tokens.iter().zip(items.iter()) {
        if !item.ok {
            skipped.push(item.error.clone().unwrap_or_else(|| "invalid".into()));
            continue;
        }
        let id = Uuid::new_v4().to_string();
        let (ct, nonce) = encrypt_field(&key, tok.trim().as_bytes());
        let discord_id = item.id.clone();
        let label = item.username.clone();
        let meta = serde_json::json!({
            "global_name": item.global_name,
            "avatar": item.avatar,
            "premium_type": item.premium_type,
            "identity": Identity::generate(),
        });
        let meta_s = meta.to_string();
        let now = unix_now();

        let row = AccountRow {
            id: id.clone(),
            discord_id: discord_id.clone(),
            label: label.clone(),
            global_name: item.global_name.clone(),
            avatar: item.avatar.clone(),
            premium_type: item.premium_type,
            proxy_id: None,
            valid: Some(true),
            last_check_at: Some(now),
        };

        let db2 = db.clone();
        let discord_id2 = discord_id.clone();
        let label2 = label.clone();
        tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
            db2.with(|c| {
                c.execute(
                    "insert into accounts(id,discord_id,label,token_enc,token_nonce,valid,last_check_at,meta_json)
                     values(?1,?2,?3,?4,?5,1,?6,?7)
                     on conflict(id) do nothing",
                    params![id, discord_id2, label2, ct, nonce, now, meta_s],
                )?;
                Ok(())
            })
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        added.push(row);
    }

    Ok(AddResult { added, skipped })
}

#[tauri::command]
pub async fn accounts_remove(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute("delete from accounts where id = ?1", params![id])?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn accounts_recheck(
    ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<AccountRow>, String> {
    let key = require_key(&state)?;
    let db = state.db.clone();

    let tokens: Vec<(String, Vec<u8>, Vec<u8>)> = tokio::task::spawn_blocking({
        let db = db.clone();
        let ids = ids.clone();
        move || -> rusqlite::Result<Vec<(String, Vec<u8>, Vec<u8>)>> {
            db.with(|c| {
                let mut out = Vec::new();
                for id in ids {
                    let mut s = c.prepare("select id, token_enc, token_nonce from accounts where id = ?1")?;
                    if let Ok((i, ct, n)) = s.query_row(params![id], |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, Vec<u8>>(2)?))
                    }) {
                        out.push((i, ct, n));
                    }
                }
                Ok(out)
            })
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    for (id, ct, n) in tokens {
        let tok = match decrypt_field(&key, &ct, &n) {
            Ok(b) => String::from_utf8_lossy(&b).to_string(),
            Err(_) => continue,
        };
        let identity = Identity::generate();
        let res = fetch_me(&tok, &identity, None).await;
        let now = unix_now();
        let db2 = db.clone();
        let (valid, meta_update) = match res {
            Ok(u) => (
                true,
                Some(serde_json::json!({
                    "global_name": u.global_name,
                    "avatar": u.avatar,
                    "premium_type": u.premium_type,
                })),
            ),
            Err(ApiError::Unauthorized) => (false, None),
            Err(_) => (true, None),
        };
        tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
            db2.with(|c| {
                if let Some(mu) = meta_update {
                    c.execute(
                        "update accounts
                         set valid=?1, last_check_at=?2,
                             meta_json = json_patch(coalesce(meta_json,'{}'), ?3)
                         where id=?4",
                        params![valid as i64, now, mu.to_string(), id],
                    )?;
                } else {
                    c.execute(
                        "update accounts set valid=?1, last_check_at=?2 where id=?3",
                        params![valid as i64, now, id],
                    )?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    }

    let db2 = state.db.clone();
    tokio::task::spawn_blocking(move || list_rows(&db2))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[derive(Deserialize, Debug, Default)]
pub struct RegisterArgs {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub date_of_birth: String,
    #[serde(default)]
    pub invite: Option<String>,
    #[serde(default)]
    pub use_proxy: bool,
    #[serde(default)]
    pub captcha_key: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct RegisterPrepared {
    pub email: String,
    pub password: String,
    pub username: String,
    pub date_of_birth: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct RegisterResult {
    pub ok: bool,
    pub account: Option<AccountRow>,
    pub error: Option<String>,
    pub prepared: RegisterPrepared,
}

#[tauri::command]
pub fn accounts_register_prepare(email: Option<String>) -> RegisterPrepared {
    RegisterPrepared {
        email: email.unwrap_or_default(),
        password: gen_password(),
        username: gen_username(),
        date_of_birth: gen_dob(),
    }
}

#[tauri::command]
pub async fn accounts_register_with_captcha(
    args: RegisterArgs,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<RegisterResult, String> {
    let email = args.email.trim().to_string();
    if email.is_empty() {
        return Err("email required".into());
    }
    let password = if args.password.is_empty() { gen_password() } else { args.password.clone() };
    let username = if args.username.is_empty() { gen_username() } else { args.username.clone() };
    let dob = if args.date_of_birth.is_empty() { gen_dob() } else { args.date_of_birth.clone() };

    let prepared = RegisterPrepared {
        email: email.clone(),
        password: password.clone(),
        username: username.clone(),
        date_of_birth: dob.clone(),
    };

    let key = require_key(&state)?;
    let _ = key;
    let identity = Identity::generate();
    let proxy = if args.use_proxy {
        pick_alive_proxy(&state).await?
    } else {
        None
    };

    let client = crate::discord::http_client::build_client(proxy.as_ref(), &identity)
        .map_err(|e| e.to_string())?;

    let fingerprint = crate::discord::api::fetch_fingerprint(&client).await.ok();

    let probe_body = crate::discord::api::RegisterBody {
        consent: true,
        date_of_birth: &dob,
        email: &email,
        fingerprint: fingerprint.as_deref(),
        invite: args.invite.as_deref(),
        password: &password,
        promotional_email_opt_in: false,
        username: &username,
        captcha_key: None,
        captcha_rqtoken: None,
    };

    let challenge = match crate::discord::api::register(&client, probe_body).await {
        Ok(ok) => {
            return Ok(RegisterResult {
                ok: true,
                account: Some(
                    finalize_account(&state, &identity, proxy.as_ref(), ok.token, &email, args.use_proxy)
                        .await
                        .map_err(|e| e)?,
                ),
                error: None,
                prepared,
            });
        }
        Err(crate::discord::api::RegisterError::Captcha(c)) => c,
        Err(crate::discord::api::RegisterError::Field(e)) => {
            return Ok(RegisterResult { ok: false, account: None, error: Some(e), prepared });
        }
        Err(crate::discord::api::RegisterError::Net(e)) => {
            return Ok(RegisterResult { ok: false, account: None, error: Some(format!("network: {e}")), prepared });
        }
        Err(crate::discord::api::RegisterError::Status(code, body)) => {
            return Ok(RegisterResult { ok: false, account: None, error: Some(format!("status {code}: {body}")), prepared });
        }
    };

    tracing::warn!(
        target: "register",
        "captcha challenge service={} sitekey={} rqtoken={}",
        challenge.service,
        challenge.sitekey,
        challenge.rqtoken.is_some()
    );

    let (session, rx) = crate::captcha::start_bridge(
        format!("register {}", username),
        challenge.sitekey.clone(),
        challenge.service.clone(),
        challenge.rqdata.clone(),
    )
    .await
    .map_err(|e| e.to_string())?;
    let url = format!("http://127.0.0.1:{}/captcha", session.port);
    let _ = app.emit(
        "register:captcha-url",
        serde_json::json!({ "url": url.clone(), "service": challenge.service, "sitekey": challenge.sitekey }),
    );
    open_in_browser(&url);

    let captcha_key = tokio::time::timeout(std::time::Duration::from_secs(600), rx)
        .await
        .map_err(|_| "captcha timeout (10 min)".to_string())?
        .map_err(|_| "captcha bridge dropped".to_string())?;

    let final_body = crate::discord::api::RegisterBody {
        consent: true,
        date_of_birth: &dob,
        email: &email,
        fingerprint: fingerprint.as_deref(),
        invite: args.invite.as_deref(),
        password: &password,
        promotional_email_opt_in: false,
        username: &username,
        captcha_key: Some(&captcha_key),
        captcha_rqtoken: challenge.rqtoken.as_deref(),
    };

    match crate::discord::api::register(&client, final_body).await {
        Ok(ok) => {
            let row = finalize_account(&state, &identity, proxy.as_ref(), ok.token, &email, args.use_proxy)
                .await
                .map_err(|e| e)?;
            Ok(RegisterResult { ok: true, account: Some(row), error: None, prepared })
        }
        Err(crate::discord::api::RegisterError::Captcha(_)) => Ok(RegisterResult {
            ok: false,
            account: None,
            error: Some("discord rejected the captcha solve. possible reasons: disposable email blocked, proxy flagged, region block. try a real email / different proxy.".into()),
            prepared,
        }),
        Err(crate::discord::api::RegisterError::Field(e)) => {
            Ok(RegisterResult { ok: false, account: None, error: Some(e), prepared })
        }
        Err(crate::discord::api::RegisterError::Net(e)) => {
            Ok(RegisterResult { ok: false, account: None, error: Some(format!("network: {e}")), prepared })
        }
        Err(crate::discord::api::RegisterError::Status(code, body)) => {
            Ok(RegisterResult { ok: false, account: None, error: Some(format!("status {code}: {body}")), prepared })
        }
    }
}

async fn finalize_account(
    state: &State<'_, AppState>,
    identity: &Identity,
    proxy: Option<&ProxyDef>,
    token: String,
    email: &str,
    auto_assign: bool,
) -> Result<AccountRow, String> {
    let key = require_key(state)?;
    let me = crate::discord::api::fetch_me(&token, identity, proxy)
        .await
        .map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let (ct, nonce) = encrypt_field(&key, token.as_bytes());
    let meta = serde_json::json!({
        "global_name": me.global_name,
        "avatar": me.avatar,
        "premium_type": me.premium_type,
        "identity": identity,
        "registered": true,
        "email": email,
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

    if auto_assign {
        let _ = crate::commands::proxies::proxies_assign_auto(state.clone()).await;
    }

    Ok(AccountRow {
        id,
        discord_id: Some(discord_id),
        label: Some(label),
        global_name: me.global_name,
        avatar: me.avatar,
        premium_type: me.premium_type,
        proxy_id: None,
        valid: Some(true),
        last_check_at: Some(now),
    })
}

fn open_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

fn gen_password() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%&?";
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}

fn gen_username() -> String {
    use rand::Rng;
    const WORDS: &[&str] = &[
        "red", "blue", "green", "fast", "slow", "night", "dawn", "void",
        "noise", "fog", "moss", "echo", "quiet", "loud", "bit", "byte",
        "ghost", "glass", "ion", "line", "max", "neon", "omen", "prism",
        "rune", "seam", "trip", "urbn", "vibe", "wax", "ysor", "zero",
    ];
    let mut rng = rand::thread_rng();
    let a = WORDS[rng.gen_range(0..WORDS.len())];
    let b = WORDS[rng.gen_range(0..WORDS.len())];
    let n: u32 = rng.gen_range(10..9999);
    format!("{a}{b}{n}")
}

fn gen_dob() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let year = rng.gen_range(1990..2003);
    let month = rng.gen_range(1..=12);
    let day = rng.gen_range(1..=28);
    format!("{:04}-{:02}-{:02}", year, month, day)
}


pub async fn pick_alive_proxy(state: &State<'_, AppState>) -> Result<Option<ProxyDef>, String> {
    let db = state.db.clone();
    let pid = tokio::task::spawn_blocking(move || -> rusqlite::Result<Option<String>> {
        db.with(|c| {
            let mut s = c.prepare(
                "select p.id from proxies p
                 where p.alive is null or p.alive = 1
                 order by (select count(*) from accounts a where a.proxy_id = p.id) asc
                 limit 1",
            )?;
            let r: Option<String> = s.query_row([], |r| r.get(0)).ok();
            Ok(r)
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    match pid {
        Some(id) => crate::commands::proxies::load_proxy_def(state, &id).await,
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn accounts_export_tokens(
    ids: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let key = require_key(&state)?;
    let db = state.db.clone();
    let rows = tokio::task::spawn_blocking(move || -> rusqlite::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        db.with(|c| match &ids {
            Some(list) if !list.is_empty() => {
                let mut out = Vec::new();
                for id in list {
                    let mut s = c.prepare(
                        "select token_enc, token_nonce from accounts where id = ?1",
                    )?;
                    if let Ok((ct, n)) = s.query_row(params![id], |r| {
                        Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
                    }) {
                        out.push((ct, n));
                    }
                }
                Ok(out)
            }
            _ => {
                let mut s = c.prepare(
                    "select token_enc, token_nonce from accounts order by created_at asc",
                )?;
                let rows = s.query_map([], |r| {
                    Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
                })?;
                let mut out: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                Ok(out)
            }
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(rows.len());
    for (ct, nonce) in rows {
        let bytes = decrypt_field(&key, &ct, &nonce).map_err(|e| e.to_string())?;
        let tok = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        out.push(tok);
    }
    Ok(out)
}

#[tauri::command]
pub async fn save_text_file(path: String, content: String) -> Result<(), String> {
    tokio::fs::write(path, content)
        .await
        .map_err(|e| e.to_string())
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
