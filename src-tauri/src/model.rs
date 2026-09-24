use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 480,
            height: 720,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyAction {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub hotkey: String,
    #[serde(default)]
    pub code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloatWindow {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub toggle_hotkey: String,
    #[serde(default)]
    pub bounds: Option<Bounds>,
    #[serde(default)]
    pub hotkeys: Vec<HotkeyAction>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub windows: Vec<FloatWindow>,
    #[serde(default)]
    pub default_bounds: Bounds,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            windows: Vec::new(),
            default_bounds: Bounds::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayState {
    pub item: FloatWindow,
}

#[derive(Clone, Debug, Serialize)]
pub struct JsResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,
}

/// 某个快捷键未能生效的原因（格式非法、重复或注册失败）。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutIssue {
    pub hotkey: String,
    pub reason: String,
}

impl Settings {
    pub fn normalize(&mut self) {
        let mut seen_windows: std::collections::HashSet<String> = std::collections::HashSet::new();
        for window in &mut self.windows {
            if window.id.is_empty() || !seen_windows.insert(window.id.clone()) {
                window.id = format!("win-{}", uuid_like());
                seen_windows.insert(window.id.clone());
            }

            let mut seen_actions: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for action in &mut window.hotkeys {
                if action.id.is_empty() || !seen_actions.insert(action.id.clone()) {
                    action.id = format!("act-{}", uuid_like());
                    seen_actions.insert(action.id.clone());
                }
            }
        }
    }

    pub fn find_window(&self, id: &str) -> Option<&FloatWindow> {
        self.windows.iter().find(|w| w.id == id)
    }
}

fn uuid_like() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // 仅靠时间戳在时钟粒度较粗时可能重复，追加自增序号保证唯一。
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}{:x}", nanos, seq)
}
