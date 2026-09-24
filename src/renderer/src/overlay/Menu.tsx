import { useEffect, useLayoutEffect, useRef, useState, type FormEvent } from 'react'
import { api } from '../api'

/**
 * 下拉栏。
 * 运行在铺满窗口的菜单 webview 里；该 webview 每次打开都新建、关闭即销毁，
 * 所以不存在旧帧问题。展开动画从「按钮大小的小胶囊」放大成面板，内容同步淡入。
 */
export default function Menu() {
  const [address, setAddress] = useState('')
  const [expanded, setExpanded] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  const refresh = (): void => {
    inputRef.current?.focus()
    void api.getState().then((next) => {
      if (next) setAddress(next.item.url)
    })
  }

  // 展开：先回到收起态，下一帧再展开，保证过渡每次都能播放。
  const expand = (): void => {
    setExpanded(false)
    requestAnimationFrame(() => setExpanded(true))
  }

  useLayoutEffect(() => {
    const root = document.documentElement.style

    // 按钮尺寸由 Rust 注入（见 overlay.rs 的 pill_metrics_script），CSS 不写死。
    const pill = window.__GF_PILL__
    if (pill) {
      root.setProperty('--gf-pill-w', `${pill.width}px`)
      root.setProperty('--gf-pill-h', `${pill.height}px`)
      root.setProperty('--gf-pill-top', `${pill.top}px`)
    }

    // 量出面板的自然高度（绘制前完成，不会闪），供高度过渡使用。
    const el = menuRef.current
    if (el) {
      const previous = el.style.cssText
      el.style.transition = 'none'
      el.style.width = '280px'
      el.style.height = 'auto'
      root.setProperty('--gf-menu-h', `${el.offsetHeight}px`)
      el.style.cssText = previous
    }

    expand()
  }, [])

  useEffect(() => {
    refresh()
    return api.onMenuState((next) => {
      if (next) {
        expand()
        refresh()
      } else {
        setExpanded(false)
      }
    })
  }, [])

  const submitAddress = (event: FormEvent): void => {
    event.preventDefault()
    if (address.trim()) void api.navigate(address)
    void api.closeMenu()
  }

  return (
    <>
      <div className="menu-backdrop" onMouseDown={() => void api.closeMenu()} />

      {/* 展开瞬间用来衔接按钮上的三个点，随后淡出 */}
      <div className="menu-dots" aria-hidden>
        <span />
        <span />
        <span />
      </div>

      <div ref={menuRef} className={`menu${expanded ? ' expanded' : ''}`}>
        <form className="menu-address" onSubmit={submitAddress}>
          <input
            ref={inputRef}
            value={address}
            spellCheck={false}
            placeholder="输入完整网址后回车，如 https://…"
            onChange={(event) => setAddress(event.target.value)}
          />
        </form>

        <div className="menu-sep" />

        <button type="button" className="menu-item" onClick={() => void api.back()}>
          <span className="mi-icon">‹</span>
          上一页
        </button>
        <button type="button" className="menu-item" onClick={() => void api.forward()}>
          <span className="mi-icon">›</span>
          下一页
        </button>
        <button type="button" className="menu-item" onClick={() => void api.reload()}>
          <span className="mi-icon">⟳</span>
          刷新
        </button>
        <button type="button" className="menu-item" onClick={() => void api.openSettings()}>
          <span className="mi-icon">⚙</span>
          设置
        </button>

        <div className="menu-sep" />

        <button type="button" className="menu-item danger" onClick={() => void api.hide()}>
          <span className="mi-icon">✕</span>
          隐藏小窗
        </button>
      </div>
    </>
  )
}
