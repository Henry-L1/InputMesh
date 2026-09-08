use std::sync::Arc;

use tauri::State;
use uuid::Uuid;

use crate::{
    model::{AppSnapshot, SharingSettings},
    runtime::AppCore,
};

#[tauri::command]
pub fn get_snapshot(core: State<'_, Arc<AppCore>>) -> AppSnapshot {
    core.snapshot()
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_sharing_enabled(
    core: State<'_, Arc<AppCore>>,
    enabled: bool,
) -> Result<AppSnapshot, String> {
    core.inner().set_sharing_enabled(enabled)
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_screen_enabled(
    core: State<'_, Arc<AppCore>>,
    screen_id: String,
    enabled: bool,
) -> Result<AppSnapshot, String> {
    core.set_screen_enabled(&screen_id, enabled)
}

#[tauri::command(rename_all = "camelCase")]
pub fn update_screen_position(
    core: State<'_, Arc<AppCore>>,
    screen_id: String,
    x: i32,
    y: i32,
) -> Result<AppSnapshot, String> {
    core.update_screen_position(&screen_id, x, y)
}

#[tauri::command]
pub fn update_settings(
    core: State<'_, Arc<AppCore>>,
    settings: SharingSettings,
) -> Result<AppSnapshot, String> {
    core.update_settings(settings)
}

#[tauri::command(rename_all = "camelCase")]
pub fn pair_peer(core: State<'_, Arc<AppCore>>, peer_id: String) -> Result<AppSnapshot, String> {
    let peer_id = Uuid::parse_str(&peer_id).map_err(|_| "无效的设备 ID".to_string())?;
    core.approve_peer(peer_id)
}

#[tauri::command(rename_all = "camelCase")]
pub fn reject_peer(core: State<'_, Arc<AppCore>>, peer_id: String) -> Result<AppSnapshot, String> {
    let peer_id = Uuid::parse_str(&peer_id).map_err(|_| "无效的设备 ID".to_string())?;
    core.reject_peer(peer_id)
}

#[tauri::command]
pub fn open_permission_settings(core: State<'_, Arc<AppCore>>) -> Result<AppSnapshot, String> {
    core.open_permission_settings()
}

#[tauri::command]
pub fn refresh_discovery(core: State<'_, Arc<AppCore>>) -> AppSnapshot {
    core.refresh_discovery()
}
