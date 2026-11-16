//! ER:LC CAD desktop shell.

pub mod commands;

use erlc_cad_core::services::{settings, AppContext};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|err| {
                std::io::Error::other(format!(
                    "Could not resolve the application data folder: {err}"
                ))
            })?;
            let ctx = AppContext::new(&data_dir).map_err(|err| {
                std::io::Error::other(format!(
                    "Could not open the CAD database: {}",
                    err.message()
                ))
            })?;
            ctx.log(
                "info",
                &format!("ER:LC CAD starting (data dir: {})", data_dir.display()),
            );
            if let Err(err) = settings::auto_backup_if_due(&ctx) {
                ctx.log(
                    "warn",
                    &format!("Automatic backup skipped: {}", err.message()),
                );
            }
            app.manage(ctx);

            // Re-check the automatic backup schedule while the app stays open.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30 * 60));
                ticker.tick().await; // the startup check already ran
                loop {
                    ticker.tick().await;
                    let ctx = handle.state::<AppContext>();
                    if let Err(err) = settings::auto_backup_if_due(&ctx) {
                        ctx.log(
                            "warn",
                            &format!("Automatic backup skipped: {}", err.message()),
                        );
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Session and setup
            commands::auth::app_bootstrap,
            commands::auth::setup_owner,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::session_info,
            commands::auth::change_password,
            // Live server data
            commands::players::get_server_snapshot,
            commands::players::search_players,
            commands::players::list_known_players,
            commands::players::get_player_profile,
            commands::players::get_player_avatar,
            commands::players::lookup_roblox_user,
            commands::players::add_staff_note,
            commands::players::list_staff_notes,
            commands::players::delete_staff_note,
            // Moderation
            commands::moderation::kick_player,
            commands::moderation::ban_player,
            commands::moderation::unban_player,
            commands::moderation::create_internal_ban,
            commands::moderation::lift_internal_ban,
            commands::moderation::list_moderation,
            commands::moderation::list_player_moderation,
            commands::moderation::list_live_bans,
            commands::moderation::list_internal_bans,
            // Dispatch
            commands::dispatch::list_calls,
            commands::dispatch::create_call,
            commands::dispatch::update_call,
            commands::dispatch::set_call_status,
            commands::dispatch::attach_player_to_call,
            commands::dispatch::detach_player_from_call,
            commands::dispatch::list_units,
            commands::dispatch::create_unit,
            commands::dispatch::update_unit,
            commands::dispatch::assign_unit_to_call,
            commands::dispatch::unassign_unit,
            commands::dispatch::delete_unit,
            // Command console
            commands::console::execute_server_command,
            commands::console::list_command_history,
            commands::console::list_command_policies,
            commands::console::upsert_command_policy,
            commands::console::delete_command_policy,
            // Staff and audit
            commands::staff::list_staff,
            commands::staff::list_roles,
            commands::staff::list_permission_catalogue,
            commands::staff::create_staff,
            commands::staff::update_staff_role,
            commands::staff::set_staff_active,
            commands::staff::reset_staff_password,
            commands::staff::set_permission_override,
            commands::staff::create_role,
            commands::staff::update_role_definition,
            commands::staff::delete_role,
            commands::staff::list_audit_logs,
            commands::staff::list_audit_categories,
            // Settings and maintenance
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_api_config,
            commands::settings::save_api_key,
            commands::settings::clear_api_key,
            commands::settings::test_api_connection,
            commands::settings::backup_database,
            commands::settings::restore_database,
            commands::settings::database_stats
        ])
        .run(tauri::generate_context!())
        .expect("error while running ER:LC CAD");
}
