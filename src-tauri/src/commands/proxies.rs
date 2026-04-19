use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::discord::api::ping_via_proxy;
use crate::discord::http_client::ProxyDef;
use crate::discord::identity::Identity;
use crate::state::{require_key, AppState};
use crate::storage::crypto::{decrypt_field, encrypt_field};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxyRow {
    pub id: String,
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub has_auth: bool,
    pub shared_slots: u32,
    pub alive: Option<bool>,
    pub latency_ms: Option<u64>,
    pub last_check_at: Option<i64>,
    pub assigned_count: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct AddResult {
    pub added: Vec<ProxyRow>,
    pub skipped: Vec<String>,
}

#[tauri::command]
pub async fn proxies_list(state: State<'_, AppState>) -> Result<Vec<ProxyRow>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || list_rows(&db))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

fn list_rows(db: &crate::storage::db::Db) -> rusqlite::Result<Vec<ProxyRow>> {
    db.with(|c| {
        let mut s = c.prepare(
            "select p.id, p.scheme, p.host, p.port,
                    p.user_enc is not null, p.shared_slots, p.alive, p.latency_ms, p.last_check_at,
                    (select count(*) from accounts a where a.proxy_id = p.id) as ac
             from proxies p
             order by p.created_at asc",
        )?;
        let rows = s.query_map([], |r| {
            let alive_i: Option<i64> = r.get(6)?;
            Ok(ProxyRow {
                id: r.get(0)?,
                scheme: r.get(1)?,
                host: r.get(2)?,
                port: r.get::<_, i64>(3)? as u16,
                has_auth: r.get(4)?,
                shared_slots: r.get::<_, i64>(5)? as u32,
                alive: alive_i.map(|v| v != 0),
                latency_ms: r.get(7)?,
                last_check_at: r.get(8)?,
                assigned_count: r.get::<_, i64>(9)? as u32,
            })
        })?;
        rows.collect()
    })
}

#[tauri::command]
pub async fn proxies_add(raw: String, state: State<'_, AppState>) -> Result<AddResult, String> {
    let key = require_key(&state)?;
    let db = state.db.clone();
    let parsed: Vec<Result<ProxyDef, String>> = raw
        .split(|c: char| c == '\n' || c == '\r')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(parse_proxy_line)
        .collect();

    let mut added = Vec::new();
    let mut skipped = Vec::new();

    for p in parsed {
        match p {
            Ok(def) => {
                let id = Uuid::new_v4().to_string();
                let (user_enc, user_nonce) = def
                    .user
                    .as_ref()
                    .map(|u| encrypt_field(&key, u.as_bytes()))
                    .map(|(a, b)| (Some(a), Some(b)))
                    .unwrap_or((None, None));
                let (pass_enc, pass_nonce) = def
                    .pass
                    .as_ref()
                    .map(|u| encrypt_field(&key, u.as_bytes()))
                    .map(|(a, b)| (Some(a), Some(b)))
                    .unwrap_or((None, None));
                let id2 = id.clone();
                let def2 = def.clone();
                let db2 = db.clone();
                tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
                    db2.with(|c| {
                        c.execute(
                            "insert into proxies(id,scheme,host,port,user_enc,user_nonce,pass_enc,pass_nonce,shared_slots)
                             values(?1,?2,?3,?4,?5,?6,?7,?8,1)",
                            params![id2, def2.scheme, def2.host, def2.port as i64, user_enc, user_nonce, pass_enc, pass_nonce],
                        )?;
                        Ok(())
                    })
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;

                added.push(ProxyRow {
                    id,
                    scheme: def.scheme.clone(),
                    host: def.host.clone(),
                    port: def.port,
                    has_auth: def.user.is_some(),
                    shared_slots: 1,
                    alive: None,
                    latency_ms: None,
                    last_check_at: None,
                    assigned_count: 0,
                });
            }
            Err(e) => skipped.push(e),
        }
    }

    Ok(AddResult { added, skipped })
}

#[tauri::command]
pub async fn proxies_remove(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute(
                "update accounts set proxy_id = null where proxy_id = ?1",
                params![id],
            )?;
            c.execute("delete from proxies where id = ?1", params![id])?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn proxies_set_slots(
    id: String,
    slots: u32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let slots = slots.clamp(1, 5);
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute(
                "update proxies set shared_slots = ?1 where id = ?2",
                params![slots as i64, id],
            )?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn proxies_test(id: String, state: State<'_, AppState>) -> Result<ProxyRow, String> {
    let def = load_proxy_def(&state, &id).await?.ok_or("proxy not found")?;
    let identity = Identity::generate();
    let now = unix_now();
    let (alive, latency) = match ping_via_proxy(&def, &identity).await {
        Ok(ms) => (true, Some(ms)),
        Err(_) => (false, None),
    };
    let db = state.db.clone();
    let id_cp = id.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute(
                "update proxies set alive = ?1, latency_ms = ?2, last_check_at = ?3 where id = ?4",
                params![alive as i64, latency.map(|v| v as i64), now, id_cp],
            )?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let db2 = state.db.clone();
    let rows = tokio::task::spawn_blocking(move || list_rows(&db2))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    rows.into_iter().find(|r| r.id == id).ok_or_else(|| "gone".into())
}

#[tauri::command]
pub async fn proxies_assign_auto(state: State<'_, AppState>) -> Result<u32, String> {
    let db = state.db.clone();
    let assigned = tokio::task::spawn_blocking(move || -> rusqlite::Result<u32> {
        db.with(|c| {
            let mut q = c.prepare(
                "select id, shared_slots,
                        (select count(*) from accounts a where a.proxy_id = p.id) as used
                 from proxies p
                 where alive is null or alive = 1
                 order by used asc, host asc",
            )?;
            let mut pool: Vec<(String, i64, i64)> = q
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))?
                .collect::<Result<_, _>>()?;

            let mut n = 0u32;
            let mut acc = c.prepare("select id from accounts where proxy_id is null")?;
            let account_ids: Vec<String> = acc
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<_, _>>()?;

            for aid in account_ids {
                pool.sort_by_key(|x| x.2);
                if let Some(slot) = pool.iter_mut().find(|p| p.2 < p.1) {
                    c.execute(
                        "update accounts set proxy_id = ?1 where id = ?2",
                        params![slot.0, aid],
                    )?;
                    slot.2 += 1;
                    n += 1;
                }
            }
            Ok(n)
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    Ok(assigned)
}

#[tauri::command]
pub async fn proxies_assign(
    account_id: String,
    proxy_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute(
                "update accounts set proxy_id = ?1 where id = ?2",
                params![proxy_id, account_id],
            )?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

pub async fn load_proxy_def(
    state: &State<'_, AppState>,
    id: &str,
) -> Result<Option<ProxyDef>, String> {
    let key = require_key(state)?;
    let db = state.db.clone();
    let id = id.to_string();
    let row = tokio::task::spawn_blocking(move || -> rusqlite::Result<Option<(String, String, i64, Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>)>> {
        db.with(|c| {
            let mut s = c.prepare(
                "select scheme, host, port, user_enc, user_nonce, pass_enc, pass_nonce from proxies where id = ?1",
            )?;
            let row = s.query_row(params![id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<Vec<u8>>>(3)?,
                    r.get::<_, Option<Vec<u8>>>(4)?,
                    r.get::<_, Option<Vec<u8>>>(5)?,
                    r.get::<_, Option<Vec<u8>>>(6)?,
                ))
            }).ok();
            Ok(row)
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let Some((scheme, host, port, ue, un, pe, pn)) = row else { return Ok(None) };
    let user = match (ue, un) {
        (Some(ct), Some(n)) => Some(
            String::from_utf8(
                decrypt_field(&key, &ct, &n).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?,
        ),
        _ => None,
    };
    let pass = match (pe, pn) {
        (Some(ct), Some(n)) => Some(
            String::from_utf8(
                decrypt_field(&key, &ct, &n).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?,
        ),
        _ => None,
    };
    Ok(Some(ProxyDef {
        scheme,
        host,
        port: port as u16,
        user,
        pass,
    }))
}

fn parse_proxy_line(line: &str) -> Result<ProxyDef, String> {
    if let Some(_) = line.find("://") {
        return parse_url_form(line);
    }
    parse_colon_form(line)
}

fn parse_url_form(line: &str) -> Result<ProxyDef, String> {
    let (scheme, rest) = line.split_once("://").ok_or_else(|| line.to_string())?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" && scheme != "socks5" && scheme != "socks5h" {
        return Err(format!("bad scheme {scheme}"));
    }
    let (auth, hostport) = match rest.rsplit_once('@') {
        Some((a, hp)) => (Some(a), hp),
        None => (None, rest),
    };
    let (host, port) = hostport
        .split_once(':')
        .ok_or_else(|| format!("bad host:port {hostport}"))?;
    let port: u16 = port.parse().map_err(|_| format!("bad port {port}"))?;
    let (user, pass) = match auth {
        Some(a) => match a.split_once(':') {
            Some((u, p)) => (Some(u.to_string()), Some(p.to_string())),
            None => (Some(a.to_string()), None),
        },
        None => (None, None),
    };
    Ok(ProxyDef {
        scheme: if scheme == "https" { "http".into() } else { scheme },
        host: host.to_string(),
        port,
        user,
        pass,
    })
}

fn parse_colon_form(line: &str) -> Result<ProxyDef, String> {
    let parts: Vec<&str> = line.split(':').collect();
    match parts.len() {
        2 => {
            let port: u16 = parts[1].parse().map_err(|_| format!("bad port {}", parts[1]))?;
            Ok(ProxyDef {
                scheme: "http".into(),
                host: parts[0].to_string(),
                port,
                user: None,
                pass: None,
            })
        }
        4 => {
            let port: u16 = parts[1].parse().map_err(|_| format!("bad port {}", parts[1]))?;
            Ok(ProxyDef {
                scheme: "http".into(),
                host: parts[0].to_string(),
                port,
                user: Some(parts[2].to_string()),
                pass: Some(parts[3].to_string()),
            })
        }
        _ => Err(format!("can't parse {line}")),
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
