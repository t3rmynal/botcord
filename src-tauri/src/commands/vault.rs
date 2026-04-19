use tauri::State;

use crate::state::AppState;
use crate::storage::crypto::{create_vault, unlock_vault, VaultMeta};

const KEY: &str = "vault_meta";

#[tauri::command]
pub fn vault_status(state: State<'_, AppState>) -> Result<&'static str, String> {
    if state.is_unlocked() {
        return Ok("unlocked");
    }
    let has = state
        .db
        .get_setting(KEY)
        .map_err(|e| e.to_string())?
        .is_some();
    Ok(if has { "locked" } else { "setup" })
}

#[tauri::command]
pub fn vault_setup(password: String, state: State<'_, AppState>) -> Result<(), String> {
    if password.len() < 4 {
        return Err("password too short".into());
    }
    if state.db.get_setting(KEY).map_err(|e| e.to_string())?.is_some() {
        return Err("vault already exists".into());
    }
    let (meta, dek) = create_vault(&password).map_err(|e| e.to_string())?;
    let j = serde_json::to_string(&meta).map_err(|e| e.to_string())?;
    state.db.set_setting(KEY, &j).map_err(|e| e.to_string())?;
    *state.key.lock() = Some(dek);
    Ok(())
}

#[tauri::command]
pub fn vault_unlock(password: String, state: State<'_, AppState>) -> Result<(), String> {
    let raw = state
        .db
        .get_setting(KEY)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no vault".to_string())?;
    let meta: VaultMeta = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let dek = unlock_vault(&meta, &password).map_err(|e| e.to_string())?;
    *state.key.lock() = Some(dek);
    Ok(())
}

#[tauri::command]
pub fn vault_lock(state: State<'_, AppState>) {
    state.lock_now();
}
