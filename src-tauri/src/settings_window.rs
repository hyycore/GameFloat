use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn open(app: &AppHandle) {
    crate::overlay::run_on_main_deferred(app, |handle| {
        if let Some(window) = handle.get_webview_window("settings") {
            let _ = window.show();
            let _ = window.set_focus();
            return;
        }

        let _ = WebviewWindowBuilder::new(
            &handle,
            "settings",
            WebviewUrl::App("settings/index.html".into()),
        )
        .title("GameFloat 设置")
        .inner_size(880.0, 760.0)
        .min_inner_size(680.0, 520.0)
        .build();
    });
}
