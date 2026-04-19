use std::sync::Arc;

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, Wry};

use crate::sidecar::voice::VoiceSidecar;
use crate::storage::crypto::DataKey;
use crate::storage::db::Db;

pub struct AppState {
    pub db: Arc<Db>,
    pub key: Arc<Mutex<Option<DataKey>>>,
    pub voice: Arc<VoiceSidecar>,
}

impl AppState {
    pub fn init(app: &AppHandle<Wry>) -> anyhow::Result<Self> {
        tracing::warn!("resource_dir: {:?}", app.path().resource_dir());
        tracing::warn!("app_data_dir: {:?}", app.path().app_data_dir());
        let data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("botcord.sqlite");
        let db = Db::open(&db_path)?;
        db.migrate()?;
        let voice = Arc::new(VoiceSidecar::new(app.clone()));
        tracing::warn!(
            "voice sidecar resolved: cwd={:?} script={:?} cwd_exists={} script_exists={}",
            voice.cwd(),
            voice.script_path(),
            voice.cwd().exists(),
            voice.script_path().exists(),
        );
        Ok(Self {
            db: Arc::new(db),
            key: Arc::new(Mutex::new(None)),
            voice,
        })
    }

    pub fn is_unlocked(&self) -> bool {
        self.key.lock().is_some()
    }

    pub fn lock_now(&self) {
        *self.key.lock() = None;
    }

    pub fn key_clone(&self) -> Option<DataKey> {
        self.key.lock().clone()
    }
}

pub fn require_key(state: &AppState) -> Result<DataKey, String> {
    state.key_clone().ok_or_else(|| "vault locked".to_string())
}
