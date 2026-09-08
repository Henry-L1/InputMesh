mod commands;
pub mod config;
pub mod crypto;
mod discovery;
mod input;
pub mod model;
mod network;
pub mod protocol;
mod runtime;
pub mod topology;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "inputmesh=info,warn".into()),
        )
        .with_target(false)
        .compact()
        .init();

    tauri::Builder::default()
        .setup(runtime::setup)
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::set_sharing_enabled,
            commands::set_screen_enabled,
            commands::update_screen_position,
            commands::update_settings,
            commands::pair_peer,
            commands::reject_peer,
            commands::open_permission_settings,
            commands::refresh_discovery,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run InputMesh");
}
