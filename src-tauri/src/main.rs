#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod model;
mod overlay;
mod settings_window;
mod shortcuts;
mod store;
mod tray;

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::Manager;

use store::AppState;

fn main() {
    tauri::Builder::default()
        // 单实例守卫：必须最先注册，让第二次启动在初始化其它插件/窗口之前就被拦截。
        // 重复启动时回调在主实例中执行，这里把设置窗口唤到前台，随后新实例自行退出。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            settings_window::open(app);
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        shortcuts::handle(app, shortcut);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let settings = store::load(&handle);

            app.manage(AppState {
                settings: Mutex::new(settings.clone()),
            });
            app.manage(shortcuts::Registry {
                bindings: Mutex::new(HashMap::new()),
                issues: Mutex::new(Vec::new()),
            });

            shortcuts::apply(&handle, &settings);
            tray::create(&handle)?;
            settings_window::open(&handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::overlay_get_state,
            commands::settings_get,
            commands::settings_save,
            commands::windows_visibility,
            commands::windows_toggle,
            commands::hotkeys_test,
            commands::shortcuts_status,
            commands::config_export,
            commands::config_import,
            commands::overlay_hide,
            commands::overlay_reload,
            commands::overlay_back,
            commands::overlay_forward,
            commands::overlay_navigate,
            commands::overlay_open_settings,
            commands::overlay_menu_toggle,
            commands::overlay_menu_close,
            commands::app_quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
