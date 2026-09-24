use std::collections::HashMap;

use tauri::{AppHandle, Webview};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::model::{ConfigResult, FloatWindow, JsResult, OverlayState, Settings, ShortcutIssue};
use crate::overlay;
use crate::settings_window;
use crate::shortcuts;
use crate::store::save;
use crate::tray;

/// 统一应用一份设置：写入内存与磁盘、重注册快捷键、同步窗口、刷新托盘与可见性。
/// `settings_save` 与 `config_import` 共用这条收尾链路。
fn apply_settings(app: &AppHandle, next: Settings) -> Settings {
    let previous = overlay::get_settings(app);
    let mut next = next;
    next.normalize();

    overlay::set_settings(app, next.clone());
    save(app, &next);
    shortcuts::apply(app, &next);
    overlay::sync_windows(app, &previous);
    tray::rebuild(app);
    overlay::broadcast_visibility(app);
    next
}

#[tauri::command]
pub async fn overlay_get_state(webview: Webview, app: AppHandle) -> Option<OverlayState> {
    let id = overlay::id_from_label(webview.label())?;
    overlay::build_state(&id, &app)
}

#[tauri::command]
pub async fn settings_get(app: AppHandle) -> Settings {
    overlay::get_settings(&app)
}

#[tauri::command]
pub async fn settings_save(app: AppHandle, patch: Settings) -> Settings {
    let previous = overlay::get_settings(&app);
    let mut next = patch;

    // 保留窗口的实际位置/尺寸，避免前端（可能过期的）bounds 覆盖运行时记录。
    for item in &mut next.windows {
        if let Some(old) = previous.find_window(&item.id) {
            if old.bounds.is_some() {
                item.bounds = old.bounds.clone();
            }
        }
    }

    apply_settings(&app, next)
}

#[tauri::command]
pub async fn windows_visibility(app: AppHandle) -> HashMap<String, bool> {
    overlay::visibility_map(&app)
}

#[tauri::command]
pub async fn windows_toggle(app: AppHandle, id: String) -> HashMap<String, bool> {
    let visible = overlay::toggle_window(&app, &id);
    let mut map = overlay::visibility_map(&app);
    // 显示是延迟执行的，用返回值覆盖，避免返回过期的可见状态。
    map.insert(id, visible);
    map
}

#[tauri::command]
pub async fn hotkeys_test(app: AppHandle, window_id: String, action_id: String) -> JsResult {
    overlay::run_action(&app, &window_id, &action_id)
}

#[tauri::command]
pub async fn shortcuts_status(app: AppHandle) -> Vec<ShortcutIssue> {
    shortcuts::issues(&app)
}

#[tauri::command]
pub async fn config_export(app: AppHandle, settings: Settings) -> ConfigResult {
    let data = serde_json::json!({
        "app": "GameFloat",
        "type": "config",
        "version": 1,
        "windows": settings.windows,
    });
    match app
        .clipboard()
        .write_text(serde_json::to_string(&data).unwrap_or_default())
    {
        Ok(()) => ConfigResult {
            ok: true,
            error: None,
            count: Some(settings.windows.len()),
            settings: None,
        },
        Err(error) => ConfigResult {
            ok: false,
            error: Some(error.to_string()),
            count: None,
            settings: None,
        },
    }
}

#[tauri::command]
pub async fn config_import(app: AppHandle) -> ConfigResult {
    let text = match app.clipboard().read_text() {
        Ok(text) => text,
        Err(error) => {
            return ConfigResult {
                ok: false,
                error: Some(error.to_string()),
                count: None,
                settings: None,
            }
        }
    };

    if text.trim().is_empty() {
        return ConfigResult {
            ok: false,
            error: Some("剪贴板为空".to_string()),
            count: None,
            settings: None,
        };
    }

    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => {
            return ConfigResult {
                ok: false,
                error: Some("剪贴板内容不是有效的 JSON".to_string()),
                count: None,
                settings: None,
            }
        }
    };

    let raw = if parsed.is_array() {
        parsed
    } else {
        parsed
            .get("windows")
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    };

    let windows: Vec<FloatWindow> = match serde_json::from_value(raw) {
        Ok(windows) => windows,
        Err(_) => {
            return ConfigResult {
                ok: false,
                error: Some("未找到小窗数据（缺少 windows 数组）".to_string()),
                count: None,
                settings: None,
            }
        }
    };

    let mut settings = overlay::get_settings(&app);
    settings.windows = windows;
    let settings = apply_settings(&app, settings);

    ConfigResult {
        ok: true,
        error: None,
        count: Some(settings.windows.len()),
        settings: Some(settings),
    }
}

#[tauri::command]
pub async fn overlay_hide(webview: Webview, app: AppHandle) {
    if let Some(id) = overlay::id_from_label(webview.label()) {
        overlay::hide_window(&app, &id);
    }
}

#[tauri::command]
pub async fn overlay_reload(webview: Webview, app: AppHandle) {
    if let Some(id) = overlay::id_from_label(webview.label()) {
        overlay::reload_window(&app, &id);
    }
}

#[tauri::command]
pub async fn overlay_back(webview: Webview, app: AppHandle) {
    if let Some(id) = overlay::id_from_label(webview.label()) {
        overlay::go_back(&app, &id);
    }
}

#[tauri::command]
pub async fn overlay_forward(webview: Webview, app: AppHandle) {
    if let Some(id) = overlay::id_from_label(webview.label()) {
        overlay::go_forward(&app, &id);
    }
}

#[tauri::command]
pub async fn overlay_navigate(webview: Webview, app: AppHandle, url: String) {
    if let Some(id) = overlay::id_from_label(webview.label()) {
        overlay::navigate_window(&app, &id, &url);
    }
}

#[tauri::command]
pub async fn overlay_open_settings(app: AppHandle) {
    settings_window::open(&app);
}

/// 切换小窗顶部下拉栏的开合，返回切换后的状态。
#[tauri::command]
pub async fn overlay_menu_toggle(webview: Webview, app: AppHandle) -> bool {
    match overlay::id_from_label(webview.label()) {
        Some(id) => overlay::toggle_menu(&app, &id),
        None => false,
    }
}

/// 关闭小窗顶部下拉栏（点击遮罩时调用）。
#[tauri::command]
pub async fn overlay_menu_close(webview: Webview, app: AppHandle) {
    if let Some(id) = overlay::id_from_label(webview.label()) {
        overlay::close_menu(&app, &id);
    }
}

#[tauri::command]
pub async fn app_quit(app: AppHandle) {
    overlay::set_quitting();
    // 退出前同步落盘，避免后台合并中的 bounds 更新丢失。
    let settings = overlay::get_settings(&app);
    save(&app, &settings);
    app.exit(0);
}
