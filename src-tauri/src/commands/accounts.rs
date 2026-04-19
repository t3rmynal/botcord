use std::time::Duration;

use rand::Rng;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
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

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
