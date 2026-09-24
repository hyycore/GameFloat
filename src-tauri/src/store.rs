use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::model::Settings;

pub struct AppState {
    pub settings: Mutex<Settings>,
}

pub fn config_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    dir.join("gamefloat-config.json")
}

fn parse_settings(raw: &str) -> Option<Settings> {
    let text = raw.trim_start_matches('\u{feff}');
    let mut settings: Settings = serde_json::from_str(text).ok()?;
    settings.normalize();
    Some(settings)
}

fn read_settings(path: &Path) -> Option<Settings> {
    let raw = fs::read_to_string(path).ok()?;
    parse_settings(&raw)
}

pub fn load(app: &AppHandle) -> Settings {
    read_settings(&config_path(app)).unwrap_or_default()
}

/// 直接写盘。
///
/// 窗口移动 / 缩放等高频变化只更新内存中的设置，不在这里写盘；
/// 真正的落盘发生在用户显式保存、导入配置以及退出时（见 `commands::app_quit`
/// 与 `tray` 的退出菜单），这样既避免了拖动窗口时的频繁 I/O，也保证手动保存的
/// 配置在崩溃/强杀时不会丢失。
pub fn save(app: &AppHandle, settings: &Settings) {
    let path = config_path(app);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, raw);
    }
}
