use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

use crate::overlay;
use crate::settings_window;

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let settings = overlay::get_settings(app);
    let menu = Menu::new(app)?;

    for item in &settings.windows {
        let name = if item.name.is_empty() {
            "小窗".to_string()
        } else {
            item.name.clone()
        };
        let submenu = Submenu::new(app, &name, true)?;

        let toggle_label = if overlay::is_visible(app, &item.id) {
            "隐藏小窗"
        } else {
            "显示小窗"
        };
        submenu.append(&MenuItem::with_id(
            app,
            format!("toggle:{}", item.id),
            toggle_label,
            true,
            None::<&str>,
        )?)?;

        for action in &item.hotkeys {
            let label = if action.hotkey.is_empty() {
                action.name.clone()
            } else {
                format!("{}  ({})", action.name, action.hotkey)
            };
            submenu.append(&MenuItem::with_id(
                app,
                format!("action:{}\u{1f}{}", item.id, action.id),
                label,
                true,
                None::<&str>,
            )?)?;
        }

        menu.append(&submenu)?;
    }

    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "settings",
        "设置...",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        "退出 GameFloat",
        true,
        None::<&str>,
    )?)?;

    Ok(menu)
}

pub fn rebuild(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("GameFloat (游窗速播)")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref().to_string();
            if id == "settings" {
                settings_window::open(app);
            } else if id == "quit" {
                overlay::set_quitting();
                // 退出前同步落盘，避免后台合并中的 bounds 更新丢失。
                let settings = overlay::get_settings(app);
                crate::store::save(app, &settings);
                app.exit(0);
            } else if let Some(window_id) = id.strip_prefix("toggle:") {
                overlay::toggle_window(app, window_id);
                rebuild(app);
            } else if let Some(rest) = id.strip_prefix("action:") {
                if let Some((window_id, action_id)) = rest.split_once('\u{1f}') {
                    let _ = overlay::run_action(app, window_id, action_id);
                }
            }
        })
        .build(app)?;
    Ok(())
}
