use std::path::Path;
use std::time::Duration;

use rand::Rng;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::commands::proxies::load_proxy_def;
use crate::discord::api::{
    fetch_channels, fetch_dms, fetch_guilds, send_dm_attachment, send_dm_text, ApiError,
};
use crate::discord::http_client::ProxyDef;
use crate::discord::identity::Identity;
use crate::state::{require_key, AppState};
use crate::storage::crypto::decrypt_field;

#[derive(Deserialize, Debug)]
pub struct BroadcastArgs {
    pub account_ids: Vec<String>,
    pub text: String,
    pub image_path: Option<String>,
    pub skip_groups: bool,
    pub min_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct BroadcastProgress {
    pub account_id: String,
    pub channel_id: String,
    pub recipient: Option<String>,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct BroadcastResult {
    pub delivered: u32,
    pub failed: u32,
    pub skipped: u32,
}

#[tauri::command]
pub async fn broadcast_dms(
    app: tauri::AppHandle,
    args: BroadcastArgs,
    state: State<'_, AppState>,
) -> Result<BroadcastResult, String> {
    if args.text.trim().is_empty() && args.image_path.is_none() {
        return Err("need text or image".into());
    }
    let min_d = args.min_delay_ms.max(400);
    let max_d = args.max_delay_ms.max(min_d + 1);

    let image = match args.image_path.as_deref() {
        Some(p) if !p.is_empty() => {
            let bytes = tokio::fs::read(p).await.map_err(|e| e.to_string())?;
            let path = Path::new(p);
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("image.bin")
                .to_string();
            let mime = mime_from_ext(&file_name);
            Some((file_name, bytes, mime))
        }
        _ => None,
    };

    let mut res = BroadcastResult::default();

    for aid in &args.account_ids {
        let loaded = load_account(&state, aid).await;
        let (token, identity, proxy) = match loaded {
            Ok(v) => v,
            Err(e) => {
                emit_fail(&app, aid, "", None, &e);
                res.failed += 1;
                continue;
            }
        };

        let dms = match fetch_dms(&token, &identity, proxy.as_ref()).await {
            Ok(v) => v,
            Err(e) => {
                emit_fail(&app, aid, "", None, &api_err(&e));
                res.failed += 1;
                continue;
            }
        };

        for ch in dms {
            if args.skip_groups && ch.kind == 3 {
                res.skipped += 1;
                continue;
            }
            if ch.kind != 1 && ch.kind != 3 {
                res.skipped += 1;
                continue;
            }

            let recipient = ch
                .recipients
                .first()
                .and_then(|r| r.global_name.clone().or(r.username.clone()))
                .or(ch.name.clone());

            let jitter = rand::thread_rng().gen_range(min_d..=max_d);
            tokio::time::sleep(Duration::from_millis(jitter)).await;

            let send_res = match &image {
                Some((name, bytes, mime)) => {
                    send_dm_attachment(
                        &token,
                        &identity,
                        proxy.as_ref(),
                        &ch.id,
                        &args.text,
                        name,
                        bytes,
                        mime,
                    )
                    .await
                }
                None => {
                    send_dm_text(&token, &identity, proxy.as_ref(), &ch.id, &args.text).await
                }
            };

            match send_res {
                Ok(()) => {
                    res.delivered += 1;
                    let _ = app.emit(
                        "broadcast:progress",
                        BroadcastProgress {
                            account_id: aid.clone(),
                            channel_id: ch.id.clone(),
                            recipient,
                            ok: true,
                            error: None,
                        },
                    );
                }
                Err(ApiError::RateLimited(ra)) => {
                    tokio::time::sleep(Duration::from_secs_f32(ra + 0.5)).await;
                    let retry = match &image {
                        Some((name, bytes, mime)) => {
                            send_dm_attachment(
                                &token,
                                &identity,
                                proxy.as_ref(),
                                &ch.id,
                                &args.text,
                                name,
                                bytes,
                                mime,
                            )
                            .await
                        }
                        None => {
                            send_dm_text(&token, &identity, proxy.as_ref(), &ch.id, &args.text)
                                .await
                        }
                    };
                    if retry.is_ok() {
                        res.delivered += 1;
                    } else {
                        res.failed += 1;
                    }
                    let _ = app.emit(
                        "broadcast:progress",
                        BroadcastProgress {
                            account_id: aid.clone(),
                            channel_id: ch.id.clone(),
                            recipient,
                            ok: retry.is_ok(),
                            error: retry.err().map(|e| api_err(&e)),
                        },
                    );
                }
                Err(e) => {
                    res.failed += 1;
                    emit_fail(&app, aid, &ch.id, recipient, &api_err(&e));
                }
            }
        }
    }

    Ok(res)
}

fn emit_fail(
    app: &tauri::AppHandle,
    account_id: &str,
    channel_id: &str,
    recipient: Option<String>,
    error: &str,
) {
    let _ = app.emit(
        "broadcast:progress",
        BroadcastProgress {
            account_id: account_id.into(),
            channel_id: channel_id.into(),
            recipient,
            ok: false,
            error: Some(error.into()),
        },
    );
}

#[derive(Deserialize, Debug)]
pub struct GuildBroadcastArgs {
    pub account_ids: Vec<String>,
    pub text: String,
    pub image_path: Option<String>,
    pub per_guild_limit: u32,
    pub skip_announcements: bool,
    pub min_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[tauri::command]
pub async fn broadcast_guilds(
    app: tauri::AppHandle,
    args: GuildBroadcastArgs,
    state: State<'_, AppState>,
) -> Result<BroadcastResult, String> {
    if args.text.trim().is_empty() && args.image_path.is_none() {
        return Err("need text or image".into());
    }
    let min_d = args.min_delay_ms.max(1500);
    let max_d = args.max_delay_ms.max(min_d + 1);
    let per_guild = args.per_guild_limit.max(1);

    let image = match args.image_path.as_deref() {
        Some(p) if !p.is_empty() => {
            let bytes = tokio::fs::read(p).await.map_err(|e| e.to_string())?;
            let path = Path::new(p);
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("image.bin")
                .to_string();
            let mime = mime_from_ext(&file_name);
            Some((file_name, bytes, mime))
        }
        _ => None,
    };

    let mut res = BroadcastResult::default();

    for aid in &args.account_ids {
        let loaded = load_account(&state, aid).await;
        let (token, identity, proxy) = match loaded {
            Ok(v) => v,
            Err(e) => {
                emit_fail(&app, aid, "", None, &e);
                res.failed += 1;
                continue;
            }
        };

        let guilds = match fetch_guilds(&token, &identity, proxy.as_ref()).await {
            Ok(v) => v,
            Err(e) => {
                emit_fail(&app, aid, "", None, &api_err(&e));
                res.failed += 1;
                continue;
            }
        };

        for g in guilds {
            let chans = match fetch_channels(&token, &identity, proxy.as_ref(), &g.id).await {
                Ok(v) => v,
                Err(_) => {
                    res.skipped += 1;
                    continue;
                }
            };

            let mut sent_here = 0u32;
            for ch in chans {
                if sent_here >= per_guild {
                    break;
                }
                let usable = ch.kind == 0 || (!args.skip_announcements && ch.kind == 5);
                if !usable {
                    continue;
                }

                let jitter = rand::thread_rng().gen_range(min_d..=max_d);
                tokio::time::sleep(Duration::from_millis(jitter)).await;

                let label = Some(format!(
                    "{} / {}",
                    g.name,
                    ch.name.clone().unwrap_or_else(|| ch.id.clone())
                ));

                let send_res = match &image {
                    Some((name, bytes, mime)) => {
                        send_dm_attachment(
                            &token,
                            &identity,
                            proxy.as_ref(),
                            &ch.id,
                            &args.text,
                            name,
                            bytes,
                            mime,
                        )
                        .await
                    }
                    None => {
                        send_dm_text(&token, &identity, proxy.as_ref(), &ch.id, &args.text).await
                    }
                };

                match send_res {
                    Ok(()) => {
                        res.delivered += 1;
                        sent_here += 1;
                        let _ = app.emit(
                            "broadcast:progress",
                            BroadcastProgress {
                                account_id: aid.clone(),
                                channel_id: ch.id.clone(),
                                recipient: label,
                                ok: true,
                                error: None,
                            },
                        );
                    }
                    Err(ApiError::Status(403, _)) => {
                        res.skipped += 1;
                    }
                    Err(ApiError::RateLimited(ra)) => {
                        tokio::time::sleep(Duration::from_secs_f32(ra + 0.5)).await;
                        res.failed += 1;
                        emit_fail(&app, aid, &ch.id, label, "rate limited");
                    }
                    Err(e) => {
                        res.failed += 1;
                        emit_fail(&app, aid, &ch.id, label, &api_err(&e));
                    }
                }
            }
        }
    }

    Ok(res)
}

fn api_err(e: &ApiError) -> String {
    e.to_string()
}

fn mime_from_ext(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
    .into()
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
