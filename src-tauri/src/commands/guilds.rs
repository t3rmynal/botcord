use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::proxies::load_proxy_def;
use crate::discord::api::{fetch_channels, fetch_guilds};
use crate::discord::identity::Identity;
use crate::state::{require_key, AppState};
use crate::storage::crypto::decrypt_field;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuildRow {
    pub guild_id: String,
    pub name: Option<String>,
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VoiceChannelRow {
    pub channel_id: String,
    pub guild_id: String,
    pub name: Option<String>,
    pub favorite: bool,
}

#[tauri::command]
pub async fn guilds_list(state: State<'_, AppState>) -> Result<Vec<GuildRow>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<Vec<GuildRow>> {
        db.with(|c| {
            let mut s = c.prepare("select guild_id, name, icon from guilds order by name asc")?;
            let rows = s.query_map([], |r| {
                Ok(GuildRow {
                    guild_id: r.get(0)?,
                    name: r.get(1)?,
                    icon: r.get(2)?,
                })
            })?;
            rows.collect()
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn voice_channels_list(state: State<'_, AppState>) -> Result<Vec<VoiceChannelRow>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<Vec<VoiceChannelRow>> {
        db.with(|c| {
            let mut s = c.prepare(
                "select channel_id, guild_id, name, favorite from voice_channels order by favorite desc, name asc",
            )?;
            let rows = s.query_map([], |r| {
                Ok(VoiceChannelRow {
                    channel_id: r.get(0)?,
                    guild_id: r.get(1)?,
                    name: r.get(2)?,
                    favorite: r.get::<_, i64>(3)? != 0,
                })
            })?;
            rows.collect()
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn voice_channels_add_manual(
    channel_id: String,
    guild_id: Option<String>,
    name: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if channel_id.trim().is_empty() {
        return Err("channel id required".into());
    }
    let gid = guild_id.unwrap_or_default();
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute(
                "insert into voice_channels(channel_id, guild_id, name, favorite)
                 values(?1, ?2, ?3, 0)
                 on conflict(channel_id) do update set guild_id=excluded.guild_id, name=excluded.name",
                params![channel_id, gid, name],
            )?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn voice_channels_remove(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute(
                "delete from voice_channels where channel_id = ?1",
                params![channel_id],
            )?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn voice_channels_set_favorite(
    channel_id: String,
    favorite: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute(
                "update voice_channels set favorite = ?1 where channel_id = ?2",
                params![favorite as i64, channel_id],
            )?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn guilds_import_from_account(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let key = require_key(&state)?;
    let db = state.db.clone();

    let row = tokio::task::spawn_blocking({
        let db = db.clone();
        let aid = account_id.clone();
        move || -> rusqlite::Result<Option<(Vec<u8>, Vec<u8>, Option<String>)>> {
            db.with(|c| {
                let mut s = c.prepare(
                    "select token_enc, token_nonce, proxy_id from accounts where id = ?1",
                )?;
                let r = s
                    .query_row(params![aid], |r| {
                        Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, Option<String>>(2)?))
                    })
                    .ok();
                Ok(r)
            })
        }
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

    let identity = Identity::generate();
    let guilds = fetch_guilds(&token, &identity, proxy.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut total_channels: u32 = 0;
    for g in guilds {
        let gid = g.id.clone();
        let name = g.name.clone();
        let icon = g.icon.clone();
        let db2 = db.clone();
        tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
            db2.with(|c| {
                c.execute(
                    "insert into guilds(guild_id, name, icon) values(?1,?2,?3)
                     on conflict(guild_id) do update set name=excluded.name, icon=excluded.icon",
                    params![gid, name, icon],
                )?;
                Ok(())
            })
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let chans = match fetch_channels(&token, &identity, proxy.as_ref(), &g.id).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let voices: Vec<_> = chans
            .into_iter()
            .filter(|c| c.kind == 2 || c.kind == 13)
            .collect();
        for ch in voices {
            let cid = ch.id.clone();
            let gid2 = g.id.clone();
            let cname = ch.name.clone();
            let db3 = db.clone();
            tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
                db3.with(|c| {
                    c.execute(
                        "insert into voice_channels(channel_id, guild_id, name, favorite)
                         values(?1, ?2, ?3, 0)
                         on conflict(channel_id) do update set guild_id=excluded.guild_id, name=excluded.name",
                        params![cid, gid2, cname],
                    )?;
                    Ok(())
                })
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
            total_channels += 1;
        }
    }

    Ok(total_channels)
}
