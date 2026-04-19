mod captcha;
mod commands;
mod discord;
mod register_bridge;
mod sidecar;
mod state;
mod storage;




use std::path::PathBuf;

use state::AppState;
use tauri::Manager;

fn init_logging() {
    let log_path: PathBuf = std::env::temp_dir().join("botcord-app.log");
    let _ = std::fs::write(
        &log_path,
        format!("--- botcord boot {} ---\n", chrono_now()),
    );
    let file_appender = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path);
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "botcord=info,botcord_lib=info,warn".into());
    match file_appender {
        Ok(file) => {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_writer(std::sync::Mutex::new(file))
                .with_ansi(false)
                .try_init();
        }
        Err(_) => {
            let _ = tracing_subscriber::fmt().with_env_filter(env_filter).try_init();
        }
    }
    tracing::info!("log file: {}", log_path.display());
    if let Ok(exe) = std::env::current_exe() {
        tracing::info!("exe: {}", exe.display());
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => format!("{}s", d.as_secs()),
        Err(_) => "?".into(),
    }
}

pub fn run() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::init(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault::vault_status,
            commands::vault::vault_setup,
            commands::vault::vault_unlock,
            commands::vault::vault_lock,
            commands::accounts::accounts_list,
            commands::accounts::accounts_check,
            commands::accounts::accounts_add,
            commands::accounts::accounts_remove,
            commands::accounts::accounts_recheck,
            commands::accounts::accounts_register_prepare,
            commands::accounts::accounts_register_with_captcha,
            commands::register_browser::accounts_register_via_browser,
            commands::accounts::accounts_export_tokens,
            commands::accounts::save_text_file,
            commands::proxies::proxies_list,
            commands::proxies::proxies_add,
            commands::proxies::proxies_remove,
            commands::proxies::proxies_set_slots,
            commands::proxies::proxies_test,
            commands::proxies::proxies_assign_auto,
            commands::proxies::proxies_assign,
            commands::guilds::guilds_list,
            commands::guilds::voice_channels_list,
            commands::guilds::voice_channels_add_manual,
            commands::guilds::voice_channels_remove,
            commands::guilds::voice_channels_set_favorite,
            commands::guilds::guilds_import_from_account,
            commands::guilds::servers_clear_all,
            commands::voice::voice_join,
            commands::voice::voice_bulk_join,
            commands::voice::voice_leave,
            commands::voice::voice_leave_all,
            commands::browser::browser_open,
            commands::browser::browser_wipe,
            commands::browser::inbox_open,
            commands::browser::inbox_wipe,
            commands::inboxes::inboxes_list,
            commands::inboxes::inboxes_add,
            commands::inboxes::inboxes_remove,
            commands::inboxes::inboxes_open,
            commands::broadcast::broadcast_dms,
            commands::broadcast::broadcast_guilds,
            commands::profile::bulk_set_profile,
            commands::social::bulk_join_invite,
            commands::social::bulk_friend_request,
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}
