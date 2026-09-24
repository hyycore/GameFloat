use std::collections::HashMap;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::model::{Settings, ShortcutIssue};
use crate::overlay;

#[derive(Clone, Debug)]
pub enum Binding {
    Toggle(String),
    Action(String, String),
}

pub struct Registry {
    pub bindings: Mutex<HashMap<Shortcut, Binding>>,
    pub issues: Mutex<Vec<ShortcutIssue>>,
}

pub fn apply(app: &AppHandle, settings: &Settings) {
    let _ = app.global_shortcut().unregister_all();

    let mut map: HashMap<Shortcut, Binding> = HashMap::new();
    let mut issues: Vec<ShortcutIssue> = Vec::new();

    let mut bind = |hotkey: &str, binding: Binding| {
        let hotkey = hotkey.trim();
        match hotkey.parse::<Shortcut>() {
            Ok(shortcut) => {
                if map.contains_key(&shortcut) {
                    issues.push(ShortcutIssue {
                        hotkey: hotkey.to_string(),
                        reason: "与其他快捷键重复".to_string(),
                    });
                } else {
                    map.insert(shortcut, binding);
                }
            }
            Err(error) => issues.push(ShortcutIssue {
                hotkey: hotkey.to_string(),
                reason: error.to_string(),
            }),
        }
    };

    for item in &settings.windows {
        if !item.toggle_hotkey.trim().is_empty() {
            bind(&item.toggle_hotkey, Binding::Toggle(item.id.clone()));
        }
        for action in &item.hotkeys {
            if !action.hotkey.trim().is_empty() {
                bind(
                    &action.hotkey,
                    Binding::Action(item.id.clone(), action.id.clone()),
                );
            }
        }
    }

    for shortcut in map.keys() {
        if let Err(error) = app.global_shortcut().register(shortcut.clone()) {
            issues.push(ShortcutIssue {
                hotkey: shortcut.to_string(),
                reason: format!("注册失败：{error}"),
            });
        }
    }

    // 让已打开的设置窗口立即看到问题；未打开时设置界面会在加载时主动查询。
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.emit("shortcuts:issues", &issues);
    }

    let state = app.state::<Registry>();
    *state.bindings.lock().unwrap() = map;
    *state.issues.lock().unwrap() = issues;
}

pub fn issues(app: &AppHandle) -> Vec<ShortcutIssue> {
    app.state::<Registry>().issues.lock().unwrap().clone()
}

pub fn handle(app: &AppHandle, shortcut: &Shortcut) {
    let binding = app
        .state::<Registry>()
        .bindings
        .lock()
        .unwrap()
        .get(shortcut)
        .cloned();

    match binding {
        Some(Binding::Toggle(id)) => {
            overlay::toggle_window(app, &id);
        }
        Some(Binding::Action(window_id, action_id)) => {
            let _ = overlay::run_action(app, &window_id, &action_id);
        }
        None => {}
    }
}
