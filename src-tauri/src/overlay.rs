use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};

use tauri::webview::NewWindowResponse;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Rect, Url, Webview, WebviewBuilder,
    WebviewUrl, Window, WindowBuilder, WindowEvent,
};

use crate::model::{Bounds, FloatWindow, JsResult, OverlayState, Settings};
use crate::store::AppState;

/// 顶部「三个点」悬浮按钮的尺寸与距顶留白（逻辑像素）。
/// 按钮 webview 就按这个尺寸创建并定位，CSS 端用 `inset: 0` 填满整个 webview，
/// 所以尺寸只在 Rust 这一处定义。
pub const UI_PILL_WIDTH: f64 = 64.0;
pub const UI_PILL_HEIGHT: f64 = 26.0;
pub const UI_PILL_TOP: f64 = 8.0;

static QUITTING: AtomicBool = AtomicBool::new(false);

pub fn set_quitting() {
    QUITTING.store(true, Ordering::SeqCst);
}

/// 记录哪些小窗的下拉栏当前处于打开状态。
/// 下拉栏由独立的、铺满窗口的 webview 承载，默认隐藏，打开时 `show`、关闭时 `hide`。
static MENU_OPEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn menu_open_set() -> &'static Mutex<HashSet<String>> {
    MENU_OPEN.get_or_init(|| Mutex::new(HashSet::new()))
}

pub fn is_menu_open(id: &str) -> bool {
    menu_open_set().lock().unwrap().contains(id)
}

fn mark_menu_open(id: &str, open: bool) {
    let mut set = menu_open_set().lock().unwrap();
    if open {
        set.insert(id.to_string());
    } else {
        set.remove(id);
    }
}

/// 悬浮按钮 webview 的几何：固定在窗口顶部居中。
/// 尺寸不随下拉栏开合变化，所以点击时不会触发任何重排。
fn pill_geometry(width: f64) -> (LogicalPosition<f64>, LogicalSize<f64>) {
    let x = ((width - UI_PILL_WIDTH) / 2.0).max(0.0);
    (
        LogicalPosition::new(x, UI_PILL_TOP),
        LogicalSize::new(UI_PILL_WIDTH, UI_PILL_HEIGHT),
    )
}

/// 把按钮尺寸注入菜单层（`window.__GF_PILL__`），供展开动画从按钮位置/尺寸起步。
/// 这样按钮几何只在 Rust 定义一次，CSS 不再写死 72/30/8。
fn pill_metrics_script() -> String {
    format!(
        "window.__GF_PILL__={{width:{w},height:{h},top:{t}}};",
        w = UI_PILL_WIDTH,
        h = UI_PILL_HEIGHT,
        t = UI_PILL_TOP,
    )
}

/// 窗口客户区的逻辑尺寸。
fn logical_size(window: &Window) -> Option<(f64, f64)> {
    let size = window.inner_size().ok()?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let logical = size.to_logical::<f64>(scale);
    Some((logical.width, logical.height))
}

type MainJob = Box<dyn FnOnce(AppHandle) + Send>;

static MAIN_DISPATCHER: OnceLock<Sender<(AppHandle, MainJob)>> = OnceLock::new();

/// 始终把任务投递到主线程事件循环执行（即使当前已在主线程）。
/// 直接调用 `run_on_main_thread` 在主线程上会内联执行，导致在事件回调/命令中
/// 创建窗口（`add_child`）时重入死锁；这里统一交给一个常驻转发线程，由它把任务
/// 投递到主线程，避免每次调用都新建一个线程。
pub fn run_on_main_deferred<F: FnOnce(AppHandle) + Send + 'static>(app: &AppHandle, f: F) {
    let dispatcher = MAIN_DISPATCHER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<(AppHandle, MainJob)>();
        std::thread::spawn(move || {
            for (handle, job) in rx {
                let inner = handle.clone();
                let _ = handle.run_on_main_thread(move || job(inner));
            }
        });
        tx
    });
    let _ = dispatcher.send((app.clone(), Box::new(f)));
}

pub fn window_label(id: &str) -> String {
    format!("w-{id}")
}

pub fn ui_label(id: &str) -> String {
    format!("w-{id}:ui")
}

/// 下拉栏（菜单）webview 的 label。
pub fn menu_label(id: &str) -> String {
    format!("w-{id}:menu")
}

pub fn content_label(id: &str) -> String {
    format!("w-{id}:content")
}

pub fn id_from_label(label: &str) -> Option<String> {
    let rest = label.strip_prefix("w-")?;
    // 只剥离已知后缀，避免小窗 id 本身含 ':' 时被错误截断。
    let id = rest
        .strip_suffix(":ui")
        .or_else(|| rest.strip_suffix(":menu"))
        .or_else(|| rest.strip_suffix(":content"))
        .unwrap_or(rest);
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

pub fn get_settings(app: &AppHandle) -> Settings {
    let state = app.state::<AppState>();
    let guard = state.settings.lock().unwrap();
    guard.clone()
}

pub fn set_settings(app: &AppHandle, settings: Settings) {
    *app.state::<AppState>().settings.lock().unwrap() = settings;
}

pub fn default_bounds(app: &AppHandle) -> Bounds {
    get_settings(app).default_bounds
}

/// 确保小窗的三个子 WebView 都存在：内容页、下拉栏（菜单）、悬浮按钮。
/// 内容页在 URL 无效时也会创建（回退到 about:blank），这样之后改成有效 URL 时
/// 仍能通过 `location.href` 正常跳转，不会留下永远空白的窗口。
///
/// 创建顺序很关键：内容 → 按钮。后添加的子 webview 位于更上层，所以按钮在网页之上。
/// 菜单 webview 不常驻，改为打开下拉栏时临时创建（见 `open_menu_webview`）。
fn ensure_children(app: &AppHandle, window: &Window, item: &FloatWindow) {
    let Some((width, height)) = logical_size(window) else {
        return;
    };

    if app.get_webview(&content_label(&item.id)).is_none() {
        // 要求完整网址（带 scheme）；无效时先建一个 about:blank，之后仍可再改。
        let target = item.url.trim();
        let parsed = match target.parse::<Url>() {
            Ok(parsed) => parsed,
            Err(_) => "about:blank"
                .parse()
                .expect("about:blank should always be a valid URL"),
        };

        let nav_handle = app.clone();
        let nav_id = item.id.clone();
        let content = WebviewBuilder::new(content_label(&item.id), WebviewUrl::External(parsed))
            .on_navigation(|_| true)
            .on_new_window(move |url, _features| {
                let handle = nav_handle.clone();
                let id = nav_id.clone();
                let href = url.to_string();
                run_on_main_deferred(&handle, move |handle| {
                    navigate_window(&handle, &id, &href);
                });
                NewWindowResponse::Deny
            });
        let _ = window.add_child(
            content,
            LogicalPosition::new(0.0, 0.0),
            LogicalSize::new(width, height),
        );
    }

    // 悬浮按钮 webview：尺寸固定、始终显示。
    if app.get_webview(&ui_label(&item.id)).is_none() {
        let ui = WebviewBuilder::new(
            ui_label(&item.id),
            WebviewUrl::App("overlay/index.html".into()),
        )
        .transparent(true);
        let (position, size) = pill_geometry(width);
        let _ = window.add_child(ui, position, size);
    }
}

/// 新建下拉栏 webview（铺满窗口、透明）。
/// 每次打开都新建一个，关闭时直接销毁——所以不需要「关闭时缩成 1×1 保持渲染」
/// 来规避旧帧，关闭后也会真正释放内存。
fn open_menu_webview(app: &AppHandle, id: &str) -> Option<Webview> {
    if let Some(menu) = app.get_webview(&menu_label(id)) {
        return Some(menu);
    }
    let window = app.get_window(&window_label(id))?;
    let (width, height) = logical_size(&window)?;
    let builder = WebviewBuilder::new(
        menu_label(id),
        WebviewUrl::App("overlay/index.html".into()),
    )
    .transparent(true)
    .initialization_script(pill_metrics_script());
    window
        .add_child(
            builder,
            LogicalPosition::new(0.0, 0.0),
            LogicalSize::new(width, height),
        )
        .ok()
}

/// 销毁下拉栏 webview（不存在时是空操作）。
fn close_menu_webview(app: &AppHandle, id: &str) {
    if let Some(menu) = app.get_webview(&menu_label(id)) {
        let _ = menu.close();
    }
}

fn ensure_window(app: &AppHandle, item: &FloatWindow) -> Option<Window> {
    let label = window_label(&item.id);
    let bounds = item.bounds.clone().unwrap_or_else(|| default_bounds(app));

    let window = match app.get_window(&label) {
        Some(window) => window,
        None => {
            let mut builder = WindowBuilder::new(app, label.clone())
                .title(if item.name.is_empty() {
                    "GameFloat".to_string()
                } else {
                    item.name.clone()
                })
                .inner_size(bounds.width as f64, bounds.height as f64)
                .decorations(false)
                .resizable(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false);

            if let (Some(x), Some(y)) = (bounds.x, bounds.y) {
                builder = builder.position(x as f64, y as f64);
            }

            let window = builder.build().ok()?;

            let app_handle = app.clone();
            let id = item.id.clone();
            window.on_window_event(move |event| match event {
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    reposition(&app_handle, &id);
                    persist_bounds(&app_handle, &id);
                }
                WindowEvent::Moved(_) => {
                    persist_bounds(&app_handle, &id);
                }
                WindowEvent::CloseRequested { api, .. } => {
                    persist_bounds(&app_handle, &id);
                    if QUITTING.load(Ordering::SeqCst) {
                        return;
                    }
                    api.prevent_close();
                    hide_window(&app_handle, &id);
                }
                _ => {}
            });

            window
        }
    };

    ensure_children(app, &window, item);
    reposition(app, &item.id);
    Some(window)
}

fn ensure_for_id(app: &AppHandle, id: &str) -> Option<Window> {
    let item = get_settings(app).find_window(id).cloned()?;
    ensure_window(app, &item)
}

pub fn reposition(app: &AppHandle, id: &str) {
    let Some(window) = app.get_window(&window_label(id)) else {
        return;
    };
    let Some((width, height)) = logical_size(&window) else {
        return;
    };

    if let Some(content) = app.get_webview(&content_label(id)) {
        let _ = content.set_bounds(Rect {
            position: LogicalPosition::new(0.0, 0.0).into(),
            size: LogicalSize::new(width, height).into(),
        });
    }
    if let Some(menu) = app.get_webview(&menu_label(id)) {
        let _ = menu.set_bounds(Rect {
            position: LogicalPosition::new(0.0, 0.0).into(),
            size: LogicalSize::new(width, height).into(),
        });
    }
    if let Some(ui) = app.get_webview(&ui_label(id)) {
        let (position, ui_size) = pill_geometry(width);
        // 按钮 webview 尺寸固定，只在窗口缩放时重新居中；点击开合不改变它。
        let _ = ui.set_bounds(Rect {
            position: position.into(),
            size: ui_size.into(),
        });
    }
}

/// 切换下拉栏开合，返回切换后的状态。
pub fn toggle_menu(app: &AppHandle, id: &str) -> bool {
    let open = !is_menu_open(id);
    set_menu_open(app, id, open);
    open
}

/// 关闭下拉栏（已关闭时是空操作）。
pub fn close_menu(app: &AppHandle, id: &str) {
    if is_menu_open(id) {
        set_menu_open(app, id, false);
    }
}

/// 打开 / 关闭下拉栏：打开时新建菜单 webview，关闭时销毁它，并广播状态给按钮。
fn set_menu_open(app: &AppHandle, id: &str, open: bool) {
    mark_menu_open(id, open);
    let id = id.to_string();
    run_on_main_deferred(app, move |handle| {
        if open {
            if let Some(menu) = open_menu_webview(&handle, &id) {
                let _ = menu.set_focus();
            }
        } else {
            close_menu_webview(&handle, &id);
        }
        // 菜单层每次都是新建的，挂载时会自己播放展开动画；这里只需通知按钮更新高亮。
        if let Some(pill) = handle.get_webview(&ui_label(&id)) {
            let _ = pill.emit("overlay:menu-state", open);
        }
    });
}

/// 只把最新的窗口位置/尺寸写入内存设置，不在此处落盘。
/// 退出时会由 `save` 统一写入配置文件。
pub fn persist_bounds(app: &AppHandle, id: &str) {
    let Some(window) = app.get_window(&window_label(id)) else {
        return;
    };
    let Ok(pos) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.inner_size() else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let logical = size.to_logical::<u32>(scale);

    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    if let Some(item) = settings.windows.iter_mut().find(|w| w.id == id) {
        item.bounds = Some(Bounds {
            x: Some(pos.x),
            y: Some(pos.y),
            width: logical.width,
            height: logical.height,
        });
    }
}

pub fn show_window(app: &AppHandle, id: &str) {
    let id = id.to_string();
    run_on_main_deferred(app, move |handle| {
        if let Some(window) = ensure_for_id(&handle, &id) {
            let _ = window.show();
            let _ = window.set_focus();
        }
        broadcast_visibility(&handle);
        // 显示是延迟执行的，必须在真正显示之后再重建托盘菜单，
        // 否则菜单里的「显示/隐藏」标签会与实际状态相反。
        crate::tray::rebuild(&handle);
    });
}

pub fn hide_window(app: &AppHandle, id: &str) {
    persist_bounds(app, id);
    // 关闭下拉栏并广播状态：下次显示时不会还挂着菜单，胶囊高亮也会同步复位。
    set_menu_open(app, id, false);
    if let Some(window) = app.get_window(&window_label(id)) {
        let _ = window.hide();
    }
    broadcast_visibility(app);
    crate::tray::rebuild(app);
}

/// 切换显隐，并返回切换后的目标可见状态（显示为延迟执行，返回值让调用方
/// 无需等待事件即可得到正确状态）。
pub fn toggle_window(app: &AppHandle, id: &str) -> bool {
    if is_visible(app, id) {
        hide_window(app, id);
        false
    } else {
        show_window(app, id);
        true
    }
}

pub fn is_visible(app: &AppHandle, id: &str) -> bool {
    app.get_window(&window_label(id))
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

pub fn visibility_map(app: &AppHandle) -> HashMap<String, bool> {
    get_settings(app)
        .windows
        .iter()
        .map(|item| (item.id.clone(), is_visible(app, &item.id)))
        .collect()
}

pub fn broadcast_visibility(app: &AppHandle) {
    let map = visibility_map(app);
    if let Some(settings) = app.get_webview_window("settings") {
        let _ = settings.emit("windows:visibility-changed", map);
    }
}

pub fn build_state(id: &str, app: &AppHandle) -> Option<OverlayState> {
    let item = get_settings(app).find_window(id)?.clone();
    Some(OverlayState { item })
}

pub fn destroy_window(app: &AppHandle, id: &str) {
    mark_menu_open(id, false);
    let id = id.to_string();
    run_on_main_deferred(app, move |handle| {
        for label in [ui_label(&id), menu_label(&id), content_label(&id)] {
            if let Some(webview) = handle.get_webview(&label) {
                let _ = webview.close();
            }
        }
        if let Some(window) = handle.get_window(&window_label(&id)) {
            let _ = window.destroy();
        }
    });
}

fn wrap_script(code: &str) -> String {
    format!(
        "(async () => {{\n  try {{\n{code}\n  }} catch (__error) {{\n    console.error('[GameFloat] JS 执行失败:', __error);\n  }}\n}})()"
    )
}

pub fn eval_in_content(app: &AppHandle, id: &str, code: &str) -> Result<(), String> {
    let webview = app
        .get_webview(&content_label(id))
        .ok_or_else(|| "小窗内容页不存在".to_string())?;
    webview.eval(code).map_err(|error| error.to_string())
}

pub fn execute_js_in_window(app: &AppHandle, window_id: &str, code: &str) -> JsResult {
    if code.trim().is_empty() {
        return JsResult { ok: true, error: None };
    }

    let window_id = window_id.to_string();
    let script = wrap_script(code);
    run_on_main_deferred(app, move |handle| {
        if ensure_for_id(&handle, &window_id).is_none() {
            return;
        }
        let _ = eval_in_content(&handle, &window_id, &script);
    });

    JsResult { ok: true, error: None }
}

pub fn run_action(app: &AppHandle, window_id: &str, action_id: &str) -> JsResult {
    let action = get_settings(app)
        .find_window(window_id)
        .and_then(|item| item.hotkeys.iter().find(|a| a.id == action_id).cloned());

    let Some(action) = action else {
        return JsResult {
            ok: false,
            error: Some("快捷键动作不存在".to_string()),
        };
    };
    execute_js_in_window(app, window_id, &action.code)
}

pub fn reload_window(app: &AppHandle, id: &str) {
    let _ = eval_in_content(app, id, "location.reload()");
}

pub fn go_back(app: &AppHandle, id: &str) {
    let _ = eval_in_content(app, id, "history.back()");
}

pub fn go_forward(app: &AppHandle, id: &str) {
    let _ = eval_in_content(app, id, "history.forward()");
}

pub fn navigate_window(app: &AppHandle, id: &str, url: &str) {
    let target = url.trim();
    // 只接受完整网址（带 scheme），避免裸域名被当成相对路径拼到当前地址后面。
    if target.parse::<Url>().is_err() {
        return;
    }
    let script = format!("location.href = {}", json_string(target));
    let _ = eval_in_content(app, id, &script);
}

pub fn sync_windows(app: &AppHandle, previous: &Settings) {
    let current = get_settings(app);

    for label in app.windows().keys() {
        if let Some(id) = id_from_label(label) {
            if current.find_window(&id).is_none() {
                destroy_window(app, &id);
            }
        }
    }

    for item in &current.windows {
        if app.get_window(&window_label(&item.id)).is_none() {
            continue;
        }
        // 复用 ensure_window 以补建可能缺失的 UI / 内容 WebView。
        let Some(window) = ensure_window(app, item) else {
            continue;
        };
        let _ = window.set_title(if item.name.is_empty() {
            "GameFloat"
        } else {
            &item.name
        });

        let old_url = previous
            .find_window(&item.id)
            .map(|w| w.url.trim().to_string())
            .unwrap_or_default();
        let new_url = item.url.trim();
        if old_url != new_url && !new_url.is_empty() {
            navigate_window(app, &item.id, new_url);
        }
    }
}
