mod commands;
mod discord;
mod sidecar;
mod state;
mod storage;

use state::AppState;
use tauri::Manager;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "botcord=info,warn".into()),
        )
        .init();

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
            commands::voice::voice_join,
            commands::voice::voice_bulk_join,
            commands::voice::voice_leave,
            commands::voice::voice_leave_all,
            commands::browser::browser_open,
            commands::browser::browser_wipe,
            commands::broadcast::broadcast_dms,
            commands::broadcast::broadcast_guilds,
            commands::profile::bulk_set_profile,
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}
