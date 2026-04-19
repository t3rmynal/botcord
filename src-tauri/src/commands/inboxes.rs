use std::fs;
use std::path::PathBuf;
use std::process::Stdio;

use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InboxRow {
    pub id: String,
    pub name: String,
    pub url: String,
    pub domain: Option<String>,
    pub created_at: i64,
}

#[tauri::command]
pub async fn inboxes_list(state: State<'_, AppState>) -> Result<Vec<InboxRow>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<Vec<InboxRow>> {
        db.with(|c| {
            let mut s = c.prepare(
                "select id, name, url, domain, created_at from inboxes order by created_at asc",
            )?;
            let rows = s.query_map([], |r| {
                Ok(InboxRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    url: r.get(2)?,
                    domain: r.get(3)?,
                    created_at: r.get(4)?,
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
pub async fn inboxes_add(
    name: String,
    url: String,
    domain: Option<String>,
    state: State<'_, AppState>,
) -> Result<InboxRow, String> {
    if name.trim().is_empty() {
        return Err("name required".into());
    }
    let url = if url.trim().is_empty() {
        "https://mail.proton.me/".into()
    } else {
        url.trim().to_string()
    };
    let domain = domain
        .as_deref()
        .map(|s| s.trim().trim_start_matches('@').to_string())
        .filter(|s| !s.is_empty());
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let db = state.db.clone();
    let id_cp = id.clone();
    let name_cp = name.clone();
    let url_cp = url.clone();
    let domain_cp = domain.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute(
                "insert into inboxes(id, name, url, domain, created_at) values(?1,?2,?3,?4,?5)",
                params![id_cp, name_cp, url_cp, domain_cp, now],
            )?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(InboxRow {
        id,
        name,
        url,
        domain,
        created_at: now,
    })
}

#[tauri::command]
pub async fn inboxes_remove(
    app: tauri::AppHandle,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.clone();
    let id_cp = id.clone();
    tokio::task::spawn_blocking(move || -> rusqlite::Result<()> {
        db.with(|c| {
            c.execute("delete from inboxes where id = ?1", params![id_cp])?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let profile = profile_dir(&app, &id)?;
    if profile.exists() {
        let _ = fs::remove_dir_all(&profile);
    }
    Ok(())
}

#[tauri::command]
pub async fn inboxes_open(
    app: tauri::AppHandle,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.clone();
    let id_cp = id.clone();
    let url = tokio::task::spawn_blocking(move || -> rusqlite::Result<Option<String>> {
        db.with(|c| {
            let mut s = c.prepare("select url from inboxes where id = ?1")?;
            Ok(s.query_row(params![id_cp], |r| r.get::<_, String>(0)).ok())
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?
    .ok_or("inbox not found")?;

    let profile = profile_dir(&app, &id)?;
    fs::create_dir_all(&profile).map_err(|e| e.to_string())?;

    let chromium = crate::commands::browser::resolve_chromium_public(&app)?;
    let pb = crate::commands::browser::find_privacy_badger_public(&app);

    let mut cmd = tokio::process::Command::new(&chromium);
    cmd.arg(format!("--user-data-dir={}", profile.display()));
    if let Some(pb_path) = pb {
        cmd.arg(format!("--load-extension={}", pb_path.display()));
    }
    cmd.arg("--no-first-run");
    cmd.arg("--no-default-browser-check");
    cmd.arg("--disable-sync");
    cmd.arg("--disable-background-networking");
    cmd.arg("--metrics-recording-only");
    cmd.arg("--no-pings");
    cmd.arg("--disable-breakpad");
    cmd.arg(url);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    cmd.spawn().map_err(|e| format!("spawn chromium: {e}"))?;

    Ok(())
}

fn profile_dir(app: &tauri::AppHandle, id: &str) -> Result<PathBuf, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(base.join("inbox-profiles").join(id))
}
