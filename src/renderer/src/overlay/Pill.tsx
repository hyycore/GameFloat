import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { api } from '../api'

/** 超过这个位移就判定为拖拽窗口，而不是点击。 */
const DRAG_THRESHOLD = 4

/**
 * 顶部「三个点」悬浮按钮。
 * 运行在固定尺寸的按钮 webview 里：按住拖动窗口，点击开合下拉栏。
 */
export default function Pill() {
  const [active, setActive] = useState(false)
  const dragStart = useRef<{ x: number; y: number } | null>(null)
  const dragged = useRef(false)

  // 菜单可能被遮罩点击关闭，这里以 Rust 广播的状态为准。
  useEffect(() => api.onMenuState(setActive), [])

  const onPointerDown = (event: ReactPointerEvent<HTMLButtonElement>): void => {
    if (event.button !== 0) return
    dragged.current = false
    dragStart.current = { x: event.clientX, y: event.clientY }
    // 捕获指针：否则鼠标一旦移出按钮这一小块 webview，就收不到后续 pointermove，拖动会失效。
    try {
      event.currentTarget.setPointerCapture(event.pointerId)
    } catch {
      /* 指针已失效时忽略 */
    }
  }

  const onPointerMove = (event: ReactPointerEvent<HTMLButtonElement>): void => {
    const start = dragStart.current
    if (!start || dragged.current) return
    if (Math.hypot(event.clientX - start.x, event.clientY - start.y) < DRAG_THRESHOLD) return
    dragged.current = true
    // 交给系统拖拽前先释放指针捕获，避免与系统自己的鼠标捕获冲突。
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId)
    }
    void getCurrentWindow().startDragging()
  }

  const onPointerUp = (event: ReactPointerEvent<HTMLButtonElement>): void => {
    dragStart.current = null
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId)
    }
  }

  const onClick = (): void => {
    // 刚刚拖动过就不要顺带开合菜单。
    if (dragged.current) {
      dragged.current = false
      return
    }
    void api.toggleMenu().then(setActive)
  }

  return (
    <button
      type="button"
      className={`pill${active ? ' active' : ''}`}
      title="按住拖动窗口，点击打开菜单"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onClick={onClick}
    >
      <span className="dot" />
      <span className="dot" />
      <span className="dot" />
    </button>
  )
}
