use std::path::Path;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::Rng;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::commands::proxies::load_proxy_def;
use crate::discord::api::{fetch_guilds, patch_guild_nick, patch_self, ApiError};
use crate::discord::http_client::ProxyDef;
use crate::discord::identity::Identity;
use crate::state::{require_key, AppState};
use crate::storage::crypto::decrypt_field;

#[derive(Deserialize, Debug)]
pub struct BulkProfileArgs {
    pub account_ids: Vec<String>,
    pub global_name: Option<String>,
    pub nickname_per_guild: Option<String>,
    pub avatar_path: Option<String>,
    #[serde(default)]
    pub reset_avatar: bool,
    pub min_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct ProfileStep {
    pub account_id: String,
    pub action: String,
    pub target: Option<String>,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct ProfileResult {
    pub accounts_done: u32,
    pub guild_nicks_set: u32,
    pub failed: u32,
}

#[tauri::command]
pub async fn bulk_set_profile(
    app: tauri::AppHandle,
    args: BulkProfileArgs,
    state: State<'_, AppState>,
) -> Result<ProfileResult, String> {
    let min_d = args.min_delay_ms.max(500);
    let max_d = args.max_delay_ms.max(min_d + 1);

    let avatar_data_url = match args.avatar_path.as_deref() {
        Some(p) if !p.is_empty() => {
            let bytes = tokio::fs::read(p).await.map_err(|e| e.to_string())?;
            let name = Path::new(p)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let mime = if name.ends_with(".jpg") || name.ends_with(".jpeg") {
                "image/jpeg"
            } else if name.ends_with(".gif") {
                "image/gif"
            } else if name.ends_with(".webp") {
                "image/webp"
            } else {
                "image/png"
            };
            Some(format!("data:{};base64,{}", mime, B64.encode(&bytes)))
        }
        _ => None,
    };

    let mut res = ProfileResult::default();

    for aid in &args.account_ids {
        let (token, identity, proxy) = match load_account(&state, aid).await {
            Ok(v) => v,
            Err(e) => {
                emit(&app, aid, "load", None, false, Some(&e));
                res.failed += 1;
                continue;
            }
        };

        if args.global_name.is_some() || avatar_data_url.is_some() || args.reset_avatar {
            let mut body = serde_json::Map::new();
            if let Some(gn) = &args.global_name {
                if !gn.trim().is_empty() {
                    body.insert("global_name".into(), serde_json::Value::String(gn.clone()));
                }
            }
            if let Some(dat) = &avatar_data_url {
                body.insert("avatar".into(), serde_json::Value::String(dat.clone()));
            } else if args.reset_avatar {
                body.insert("avatar".into(), serde_json::Value::Null);
            }
            if !body.is_empty() {
                let jitter = rand::thread_rng().gen_range(min_d..=max_d);
                tokio::time::sleep(Duration::from_millis(jitter)).await;
                match patch_self(&token, &identity, proxy.as_ref(), serde_json::Value::Object(body))
                    .await
                {
                    Ok(()) => {
                        emit(&app, aid, "profile", None, true, None);
                    }
                    Err(e) => {
                        emit(&app, aid, "profile", None, false, Some(&e.to_string()));
                        res.failed += 1;
                    }
                }
            }
        }

        if let Some(nick) = args.nickname_per_guild.as_ref().filter(|s| !s.trim().is_empty()) {
            let guilds = match fetch_guilds(&token, &identity, proxy.as_ref()).await {
                Ok(v) => v,
                Err(e) => {
                    emit(&app, aid, "fetch_guilds", None, false, Some(&e.to_string()));
                    res.failed += 1;
                    continue;
                }
            };
            for g in guilds {
                let jitter = rand::thread_rng().gen_range(min_d..=max_d);
                tokio::time::sleep(Duration::from_millis(jitter)).await;
                let label = Some(g.name.clone());
                match patch_guild_nick(&token, &identity, proxy.as_ref(), &g.id, nick).await {
                    Ok(()) => {
                        res.guild_nicks_set += 1;
                        emit(&app, aid, "nick", label, true, None);
                    }
                    Err(ApiError::Status(403, _)) => {
                        emit(&app, aid, "nick", label, false, Some("no permission"));
                    }
                    Err(ApiError::RateLimited(ra)) => {
                        tokio::time::sleep(Duration::from_secs_f32(ra + 0.5)).await;
                        emit(&app, aid, "nick", label, false, Some("rate limited"));
                        res.failed += 1;
                    }
                    Err(e) => {
                        emit(&app, aid, "nick", label, false, Some(&e.to_string()));
                        res.failed += 1;
                    }
                }
            }
        }

        res.accounts_done += 1;
    }

    Ok(res)
}

fn emit(
    app: &tauri::AppHandle,
    account_id: &str,
    action: &str,
    target: Option<String>,
    ok: bool,
    error: Option<&str>,
) {
    let _ = app.emit(
        "profile:progress",
        ProfileStep {
            account_id: account_id.into(),
            action: action.into(),
            target,
            ok,
            error: error.map(|s| s.to_string()),
        },
    );
}

async fn load_account(
    state: &State<'_, AppState>,
    account_id: &str,
) -> Result<(String, Identity, Option<ProxyDef>), String> {
    let key = require_key(state)?;
    let db = state.db.clone();
    let aid = account_id.to_string();
    let row = tokio::task::spawn_blocking(move || -> rusqlite::Result<Option<(Vec<u8>, Vec<u8>, Option<String>, Option<String>)>> {
        db.with(|c| {
            let mut s = c.prepare(
                "select token_enc, token_nonce, proxy_id, meta_json from accounts where id = ?1",
            )?;
            let r = s.query_row(params![aid], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            }).ok();
            Ok(r)
        })
    })
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
