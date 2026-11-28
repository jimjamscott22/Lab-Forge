//! LabForge: local-first training platform for authorized, isolated cybersecurity labs.
//!
//! Layering (dependencies point downwards only):
//! `commands` (Tauri IPC) → `services` (rules, validation) → `repositories` (SQL)
//! → `database` (pool, migrations). `models` are shared data types; `content` holds
//! bundled challenge definitions; `lab_runtime` is reserved for Milestone 3.

pub mod commands;
pub mod content;
pub mod database;
pub mod error;
pub mod lab_runtime;
pub mod models;
pub mod repositories;
pub mod services;

use std::path::PathBuf;

use tauri::Manager;

use commands::AppState;

/// Environment variable that overrides where the database lives. Used for the
/// restart check and for tests; normal launches use the platform app data directory.
pub const DATA_DIR_ENV: &str = "LABFORGE_DATA_DIR";

fn resolve_data_dir(app: &tauri::App) -> Result<PathBuf, Box<dyn std::error::Error>> {
    match std::env::var_os(DATA_DIR_ENV) {
        Some(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
        _ => Ok(app.path().app_data_dir()?),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = resolve_data_dir(app)?;
            let pool = tauri::async_runtime::block_on(async {
                let pool = database::open_in_dir(&data_dir).await?;
                let seeded = services::challenges::seed_bundled(&pool).await?;
                eprintln!(
                    "labforge: data dir {}, seeded {seeded} new challenge(s)",
                    data_dir.display()
                );
                Ok::<_, error::AppError>(pool)
            })?;
            app.manage(AppState { pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_challenges,
            commands::get_challenge,
            commands::create_challenge,
            commands::update_challenge,
            commands::delete_challenge,
            commands::start_attempt,
            commands::get_attempt,
            commands::complete_step,
            commands::save_note,
            commands::get_dashboard,
            commands::list_attempts,
            commands::open_external_link,
        ])
        .run(tauri::generate_context!())
        .expect("error while running LabForge");
}
