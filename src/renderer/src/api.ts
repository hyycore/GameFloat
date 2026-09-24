import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { ConfigResult, JsResult, OverlayState, Settings, ShortcutIssue } from '../../shared/types'

/** 订阅一个后端事件，返回取消订阅函数。 */
function subscribe<T>(event: string, callback: (payload: T) => void): () => void {
  let unlisten: (() => void) | null = null
  let disposed = false
  void listen<T>(event, (e) => callback(e.payload)).then((fn) => {
    if (disposed) fn()
    else unlisten = fn
  })
  return () => {
    disposed = true
    if (unlisten) unlisten()
  }
}

export const api = {
  getState: (): Promise<OverlayState | null> => invoke('overlay_get_state'),
  getSettings: (): Promise<Settings> => invoke('settings_get'),
  saveSettings: (patch: Settings): Promise<Settings> => invoke('settings_save', { patch }),
  testAction: (windowId: string, actionId: string): Promise<JsResult> =>
    invoke('hotkeys_test', { windowId, actionId }),

  getShortcutIssues: (): Promise<ShortcutIssue[]> => invoke('shortcuts_status'),
  onShortcutIssues: (callback: (issues: ShortcutIssue[]) => void): (() => void) =>
    subscribe('shortcuts:issues', callback),

  exportConfig: (settings: Settings): Promise<ConfigResult> =>
    invoke('config_export', { settings }),
  importConfig: (): Promise<ConfigResult> => invoke('config_import'),

  getVisibility: (): Promise<Record<string, boolean>> => invoke('windows_visibility'),
  toggleWindow: (id: string): Promise<Record<string, boolean>> =>
    invoke('windows_toggle', { id }),
  onVisibility: (callback: (visibility: Record<string, boolean>) => void): (() => void) =>
    subscribe('windows:visibility-changed', callback),

  hide: (): Promise<void> => invoke('overlay_hide'),
  reload: (): Promise<void> => invoke('overlay_reload'),
  back: (): Promise<void> => invoke('overlay_back'),
  forward: (): Promise<void> => invoke('overlay_forward'),
  navigate: (url: string): Promise<void> => invoke('overlay_navigate', { url }),
  openSettings: (): Promise<void> => invoke('overlay_open_settings'),

  toggleMenu: (): Promise<boolean> => invoke('overlay_menu_toggle'),
  closeMenu: (): Promise<void> => invoke('overlay_menu_close'),
  onMenuState: (callback: (open: boolean) => void): (() => void) =>
    subscribe('overlay:menu-state', callback)
}
