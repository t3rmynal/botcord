use std::time::Duration;

use rand::Rng;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::commands::proxies::load_proxy_def;
use crate::discord::api::{
    join_invite, send_friend_request, CaptchaChallenge, RegisterError,
};
use crate::discord::http_client::{build_client, ProxyDef};
use crate::discord::identity::Identity;
use crate::state::{require_key, AppState};
use crate::storage::crypto::decrypt_field;

#[derive(Deserialize, Debug)]
pub struct BulkInviteArgs {
    pub account_ids: Vec<String>,
    pub invite: String,
    pub min_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Deserialize, Debug)]
pub struct BulkFriendArgs {
    pub account_ids: Vec<String>,
    pub username: String,
    pub discriminator: Option<String>,
    pub min_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct SocialStep {
    pub account_id: String,
    pub state: String,
    pub message: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct SocialResult {
    pub ok: u32,
    pub failed: u32,
    pub captcha_pending: u32,
}

fn extract_invite_code(s: &str) -> String {
    let s = s.trim().trim_end_matches('/');
    if let Some(idx) = s.rfind('/') {
        return s[idx + 1..].to_string();
    }
    s.to_string()
}

fn emit(app: &tauri::AppHandle, evt: &str, payload: SocialStep) {
    let _ = app.emit(evt, payload);
}

async fn load_account(
    state: &State<'_, AppState>,
    account_id: &str,
) -> Result<(String, Identity, Option<ProxyDef>), String> {
    let key = require_key(state)?;
    let db = state.db.clone();
    let aid = account_id.to_string();
    let row = tokio::task::spawn_blocking(
        move || -> rusqlite::Result<Option<(Vec<u8>, Vec<u8>, Option<String>, Option<String>)>> {
            db.with(|c| {
                let mut s = c.prepare(
                    "select token_enc, token_nonce, proxy_id, meta_json from accounts where id = ?1",
                )?;
                let r = s
                    .query_row(params![aid], |r| {
                        Ok((
                            r.get::<_, Vec<u8>>(0)?,
                            r.get::<_, Vec<u8>>(1)?,
                            r.get::<_, Option<String>>(2)?,
                            r.get::<_, Option<String>>(3)?,
                        ))
                    })
                    .ok();
                Ok(r)
            })
        },
    )
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let (ct, nonce, proxy_id, meta) = row.ok_or("account not found")?;
    let token_bytes = decrypt_field(&key, &ct, &nonce).map_err(|e| e.to_string())?;
    let token = String::from_utf8(token_bytes).map_err(|e| e.to_string())?;

    let identity = meta
        .as_deref()
        .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
        .and_then(|v| v.get("identity").cloned())
        .and_then(|v| serde_json::from_value::<Identity>(v).ok())
        .unwrap_or_else(Identity::generate);

    let proxy = match proxy_id {
        Some(pid) => load_proxy_def(state, &pid).await?,
        None => None,
    };
    Ok((token, identity, proxy))
}

async fn wait_captcha(
    app: &tauri::AppHandle,
    account_id: &str,
    challenge: &CaptchaChallenge,
    event: &str,
) -> Result<String, String> {
    let (session, rx) = crate::captcha::start_bridge(
        format!("{} {}", event, account_id),
        challenge.sitekey.clone(),
        challenge.service.clone(),
        challenge.rqdata.clone(),
    )
    .await?;
    let url = format!("http://127.0.0.1:{}/captcha", session.port);
    let _ = app.emit(
        "captcha:url",
        serde_json::json!({
            "url": url.clone(),
            "sitekey": challenge.sitekey,
            "service": challenge.service,
            "account_id": account_id,
            "flow": event,
        }),
    );
    emit(
        app,
        event,
        SocialStep {
            account_id: account_id.into(),
            state: "captcha".into(),
            message: Some(url.clone()),
        },
    );
    open_in_browser(&url);
    tokio::time::timeout(Duration::from_secs(600), rx)
        .await
        .map_err(|_| "captcha timeout".to_string())?
        .map_err(|_| "captcha bridge dropped".to_string())
}

fn open_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
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

#[tauri::command]
pub async fn bulk_join_invite(
    app: tauri::AppHandle,
    args: BulkInviteArgs,
    state: State<'_, AppState>,
) -> Result<SocialResult, String> {
    let code = extract_invite_code(&args.invite);
    if code.is_empty() {
        return Err("invalid invite".into());
    }
    let min_d = args.min_delay_ms.max(800);
    let max_d = args.max_delay_ms.max(min_d + 1);

    let mut out = SocialResult::default();
    for aid in &args.account_ids {
        let jitter = rand::thread_rng().gen_range(min_d..=max_d);
        tokio::time::sleep(Duration::from_millis(jitter)).await;

        let (token, identity, proxy) = match load_account(&state, aid).await {
            Ok(v) => v,
            Err(e) => {
                out.failed += 1;
                emit(&app, "invite:progress", SocialStep { account_id: aid.clone(), state: "error".into(), message: Some(e) });
                continue;
            }
        };
        let client = match build_client(proxy.as_ref(), &identity) {
            Ok(c) => c,
            Err(e) => {
                out.failed += 1;
                emit(&app, "invite:progress", SocialStep { account_id: aid.clone(), state: "error".into(), message: Some(e.to_string()) });
                continue;
            }
        };

        let res = join_invite(&client, &token, &code, None, None).await;
        match res {
            Ok(()) => {
                out.ok += 1;
                emit(&app, "invite:progress", SocialStep { account_id: aid.clone(), state: "joined".into(), message: None });
            }
            Err(RegisterError::Captcha(c)) => {
                out.captcha_pending += 1;
                let cap = match wait_captcha(&app, aid, &c, "invite:progress").await {
                    Ok(k) => k,
                    Err(e) => {
                        out.failed += 1;
                        out.captcha_pending -= 1;
                        emit(&app, "invite:progress", SocialStep { account_id: aid.clone(), state: "error".into(), message: Some(e) });
                        continue;
                    }
                };
                let retry = join_invite(&client, &token, &code, Some(&cap), c.rqtoken.as_deref()).await;
                out.captcha_pending = out.captcha_pending.saturating_sub(1);
                match retry {
                    Ok(()) => {
                        out.ok += 1;
                        emit(&app, "invite:progress", SocialStep { account_id: aid.clone(), state: "joined".into(), message: None });
                    }
                    Err(e) => {
                        out.failed += 1;
                        emit(&app, "invite:progress", SocialStep { account_id: aid.clone(), state: "error".into(), message: Some(format_err(&e)) });
                    }
                }
            }
            Err(e) => {
                out.failed += 1;
                emit(&app, "invite:progress", SocialStep { account_id: aid.clone(), state: "error".into(), message: Some(format_err(&e)) });
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn bulk_friend_request(
    app: tauri::AppHandle,
    args: BulkFriendArgs,
    state: State<'_, AppState>,
) -> Result<SocialResult, String> {
    if args.username.trim().is_empty() {
        return Err("username required".into());
    }
    let min_d = args.min_delay_ms.max(600);
    let max_d = args.max_delay_ms.max(min_d + 1);

    let mut out = SocialResult::default();
    for aid in &args.account_ids {
        let jitter = rand::thread_rng().gen_range(min_d..=max_d);
        tokio::time::sleep(Duration::from_millis(jitter)).await;

        let (token, identity, proxy) = match load_account(&state, aid).await {
            Ok(v) => v,
            Err(e) => {
                out.failed += 1;
                emit(&app, "friend:progress", SocialStep { account_id: aid.clone(), state: "error".into(), message: Some(e) });
                continue;
            }
        };
        let client = match build_client(proxy.as_ref(), &identity) {
            Ok(c) => c,
            Err(e) => {
                out.failed += 1;
                emit(&app, "friend:progress", SocialStep { account_id: aid.clone(), state: "error".into(), message: Some(e.to_string()) });
                continue;
            }
        };

        let res = send_friend_request(
            &client,
            &token,
            &args.username,
            args.discriminator.as_deref(),
            None,
            None,
        )
        .await;
        match res {
            Ok(()) => {
                out.ok += 1;
                emit(&app, "friend:progress", SocialStep { account_id: aid.clone(), state: "sent".into(), message: None });
            }
            Err(RegisterError::Captcha(c)) => {
                out.captcha_pending += 1;
                let cap = match wait_captcha(&app, aid, &c, "friend:progress").await {
                    Ok(k) => k,
                    Err(e) => {
                        out.failed += 1;
                        out.captcha_pending -= 1;
                        emit(&app, "friend:progress", SocialStep { account_id: aid.clone(), state: "error".into(), message: Some(e) });
                        continue;
                    }
                };
                let retry = send_friend_request(
                    &client,
                    &token,
                    &args.username,
                    args.discriminator.as_deref(),
                    Some(&cap),
                    c.rqtoken.as_deref(),
                )
                .await;
                out.captcha_pending = out.captcha_pending.saturating_sub(1);
                match retry {
                    Ok(()) => {
                        out.ok += 1;
                        emit(&app, "friend:progress", SocialStep { account_id: aid.clone(), state: "sent".into(), message: None });
                    }
                    Err(e) => {
                        out.failed += 1;
                        emit(&app, "friend:progress", SocialStep { account_id: aid.clone(), state: "error".into(), message: Some(format_err(&e)) });
                    }
                }
            }
            Err(e) => {
                out.failed += 1;
                emit(&app, "friend:progress", SocialStep { account_id: aid.clone(), state: "error".into(), message: Some(format_err(&e)) });
            }
        }
    }
    Ok(out)
}

fn format_err(e: &RegisterError) -> String {
    match e {
        RegisterError::Captcha(_) => "captcha required".into(),
        RegisterError::Field(s) => s.clone(),
        RegisterError::Net(s) => format!("network: {s}"),
        RegisterError::Status(c, b) => format!("{c}: {b}"),
    }
}
