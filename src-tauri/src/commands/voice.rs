use std::time::Duration;

use rand::Rng;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;

use crate::commands::proxies::load_proxy_def;
use crate::discord::http_client::ProxyDef;
use crate::discord::identity::Identity;
use crate::state::{require_key, AppState};
use crate::storage::crypto::decrypt_field;

#[derive(Deserialize, Debug)]
pub struct JoinArgs {
    pub account_id: String,
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub mode: String,
}

#[derive(Deserialize, Debug)]
pub struct BulkJoinArgs {
    pub account_ids: Vec<String>,
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub mode: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct JoinResult {
    pub account_id: String,
    pub ok: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn voice_join(args: JoinArgs, state: State<'_, AppState>) -> Result<JoinResult, String> {
    let guild_id = resolve_guild(&state, &args.channel_id, args.guild_id.clone()).await?;
    let res = join_single(&state, &args.account_id, &guild_id, &args.channel_id, &args.mode).await;
    Ok(match res {
        Ok(_) => JoinResult {
            account_id: args.account_id,
            ok: true,
            error: None,
        },
        Err(e) => JoinResult {
            account_id: args.account_id,
            ok: false,
            error: Some(e),
        },
    })
}

#[tauri::command]
pub async fn voice_bulk_join(
    args: BulkJoinArgs,
    state: State<'_, AppState>,
) -> Result<Vec<JoinResult>, String> {
    let guild_id = resolve_guild(&state, &args.channel_id, args.guild_id.clone()).await?;
    let mut out = Vec::with_capacity(args.account_ids.len());
    for aid in args.account_ids {
        let jitter = rand::thread_rng().gen_range(800..=2200);
        tokio::time::sleep(Duration::from_millis(jitter)).await;
        let r = join_single(&state, &aid, &guild_id, &args.channel_id, &args.mode).await;
        out.push(match r {
            Ok(_) => JoinResult {
                account_id: aid,
                ok: true,
                error: None,
            },
            Err(e) => JoinResult {
                account_id: aid,
                ok: false,
                error: Some(e),
            },
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn voice_leave(account_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let sc = state.voice.clone();
    sc.call("leave", json!({ "account_id": account_id })).await?;
    Ok(())
}

#[tauri::command]
pub async fn voice_leave_all(
    account_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    for aid in account_ids {
        let sc = state.voice.clone();
        let _ = sc.call("leave", json!({ "account_id": aid })).await;
    }
    Ok(())
}

async fn resolve_guild(
    state: &State<'_, AppState>,
    channel_id: &str,
    explicit: Option<String>,
) -> Result<String, String> {
    if let Some(g) = explicit.filter(|s| !s.is_empty()) {
        return Ok(g);
    }
    let db = state.db.clone();
    let cid = channel_id.to_string();
    let r = tokio::task::spawn_blocking(move || -> rusqlite::Result<Option<String>> {
        db.with(|c| {
            let mut s = c.prepare("select guild_id from voice_channels where channel_id = ?1")?;
            let g = s.query_row(params![cid], |r| r.get::<_, String>(0)).ok();
            Ok(g.filter(|s| !s.is_empty()))
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    r.ok_or_else(|| "guild id unknown, add channel via import or set it manually".to_string())
}

async fn join_single(
    state: &State<'_, AppState>,
    account_id: &str,
    guild_id: &str,
    channel_id: &str,
    mode: &str,
) -> Result<(), String> {
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

    let proxy: Option<ProxyDef> = match proxy_id {
        Some(pid) => load_proxy_def(state, &pid).await?,
        None => None,
    };

    let sc = state.voice.clone();
    sc.call(
        "join",
        json!({
            "account_id": account_id,
            "token": token,
            "identity": identity,
            "proxy": proxy.map(|p| json!({
                "scheme": p.scheme,
                "host": p.host,
                "port": p.port,
                "user": p.user,
                "pass": p.pass,
            })),
            "guild_id": guild_id,
            "channel_id": channel_id,
            "mode": mode,
        }),
    )
    .await?;
    Ok(())
}
