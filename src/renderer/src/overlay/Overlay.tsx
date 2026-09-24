import { getCurrentWebview } from '@tauri-apps/api/webview'
import Menu from './Menu'
import Pill from './Pill'

/**
 * 同一个页面入口会被两个 webview 加载：
 * - `w-<id>:ui`   → 顶部悬浮按钮
 * - `w-<id>:menu` → 铺满窗口的下拉栏
 * 通过 webview label 判断当前角色。
 */
const IS_MENU = getCurrentWebview().label.endsWith(':menu')

export default function Overlay() {
  return IS_MENU ? <Menu /> : <Pill />
}
