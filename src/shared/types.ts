export interface WindowBounds {
  x?: number
  y?: number
  width: number
  height: number
}

export interface HotkeyAction {
  id: string
  name: string
  hotkey: string
  code: string
}

export interface FloatWindow {
  id: string
  name: string
  url: string
  toggleHotkey: string
  bounds?: WindowBounds
  hotkeys: HotkeyAction[]
}

export interface Settings {
  windows: FloatWindow[]
  defaultBounds: WindowBounds
}

export interface OverlayState {
  item: FloatWindow
}

export interface JsResult {
  ok: boolean
  error?: string
}

export interface ConfigResult {
  ok: boolean
  error?: string
  count?: number
  settings?: Settings
}

export interface ShortcutIssue {
  hotkey: string
  reason: string
}
