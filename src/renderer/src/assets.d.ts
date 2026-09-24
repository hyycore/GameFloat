declare module '*.css'
declare module '*.png'
declare module '*.svg'
declare module '*.ico'

/** 由 Rust 在创建菜单 webview 时注入的按钮尺寸（见 overlay.rs 的 pill_metrics_script）。 */
interface Window {
  __GF_PILL__?: { width: number; height: number; top: number }
}
