import { Suspense, lazy, useEffect, useState } from 'react'
import type {
  FloatWindow,
  HotkeyAction,
  Settings as SettingsData,
  ShortcutIssue
} from '../../../shared/types'
import { api } from '../api'
import HotkeyInput from '../components/HotkeyInput'

// CodeMirror 体积较大，且只在打开代码编辑器时才需要，改为按需加载。
const CodeEditorModal = lazy(() => import('../components/CodeEditorModal'))

function newWindow(): FloatWindow {
  return {
    id: (crypto as Crypto).randomUUID(),
    name: '新小窗',
    url: 'https://',
    toggleHotkey: '',
    hotkeys: []
  }
}

function newAction(): HotkeyAction {
  return {
    id: (crypto as Crypto).randomUUID(),
    name: '新动作',
    hotkey: '',
    code: ''
  }
}

interface Editing {
  windowId: string
  actionId: string
}

type Tab = 'windows' | 'config'

export default function Settings() {
  const [draft, setDraft] = useState<SettingsData | null>(null)
  const [saved, setSaved] = useState(false)
  const [status, setStatus] = useState('')
  const [editing, setEditing] = useState<Editing | null>(null)
  const [visibility, setVisibility] = useState<Record<string, boolean>>({})
  const [issues, setIssues] = useState<ShortcutIssue[]>([])
  const [tab, setTab] = useState<Tab>('windows')

  useEffect(() => {
    void api.getSettings().then(setDraft)
  }, [])

  useEffect(() => {
    void api.getVisibility().then(setVisibility)
    return api.onVisibility(setVisibility)
  }, [])

  useEffect(() => {
    void api.getShortcutIssues().then(setIssues)
    return api.onShortcutIssues(setIssues)
  }, [])

  if (!draft) return <div className="loading">加载中…</div>

  const update = (patch: Partial<SettingsData>): void => {
    setDraft({ ...draft, ...patch })
    setSaved(false)
  }

  const updateWindow = (windowId: string, patch: Partial<FloatWindow>): void => {
    update({
      windows: draft.windows.map((item) =>
        item.id === windowId ? { ...item, ...patch } : item
      )
    })
  }

  const updateAction = (
    windowId: string,
    actionId: string,
    patch: Partial<HotkeyAction>
  ): void => {
    update({
      windows: draft.windows.map((item) =>
        item.id === windowId
          ? {
              ...item,
              hotkeys: item.hotkeys.map((action) =>
                action.id === actionId ? { ...action, ...patch } : action
              )
            }
          : item
      )
    })
  }

  const addWindow = (): void => update({ windows: [...draft.windows, newWindow()] })

  const removeWindow = (windowId: string): void => {
    update({ windows: draft.windows.filter((item) => item.id !== windowId) })
  }

  const toggleWindow = async (windowId: string): Promise<void> => {
    setVisibility(await api.toggleWindow(windowId))
  }

  const addAction = (windowId: string): void => {
    update({
      windows: draft.windows.map((item) =>
        item.id === windowId ? { ...item, hotkeys: [...item.hotkeys, newAction()] } : item
      )
    })
  }

  const removeAction = (windowId: string, actionId: string): void => {
    update({
      windows: draft.windows.map((item) =>
        item.id === windowId
          ? { ...item, hotkeys: item.hotkeys.filter((action) => action.id !== actionId) }
          : item
      )
    })
  }

  const save = async (): Promise<void> => {
    const next = await api.saveSettings(draft)
    setDraft(next)
    setSaved(true)
    window.setTimeout(() => setSaved(false), 2000)
  }

  const flashStatus = (message: string, ms = 3000): void => {
    setStatus(message)
    window.setTimeout(() => setStatus(''), ms)
  }

  const testAction = async (windowId: string, action: HotkeyAction): Promise<void> => {
    const result = await api.testAction(windowId, action.id)
    flashStatus(result.ok ? `「${action.name}」执行成功 ✓` : `执行失败：${result.error ?? ''}`)
  }

  const exportConfig = async (): Promise<void> => {
    const result = await api.exportConfig(draft)
    flashStatus(
      result.ok ? `已导出 ${result.count ?? 0} 个小窗到剪贴板 ✓` : `导出失败：${result.error ?? ''}`
    )
  }

  const importConfig = async (): Promise<void> => {
    const result = await api.importConfig()
    if (result.ok && result.settings) {
      setDraft(result.settings)
      setSaved(false)
      flashStatus(`已导入 ${result.count ?? 0} 个小窗（原小窗已全部删除）`, 4000)
    } else {
      flashStatus(`导入失败：${result.error ?? ''}`, 4000)
    }
  }

  const editingWindow = editing
    ? draft.windows.find((item) => item.id === editing.windowId) ?? null
    : null
  const editingAction = editingWindow
    ? editingWindow.hotkeys.find((action) => action.id === editing?.actionId) ?? null
    : null

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="sidebar-title">GameFloat</div>
        <nav className="nav">
          <button
            className={`nav-item${tab === 'windows' ? ' active' : ''}`}
            onClick={() => setTab('windows')}
          >
            小窗
          </button>
          <button
            className={`nav-item${tab === 'config' ? ' active' : ''}`}
            onClick={() => setTab('config')}
          >
            配置
          </button>
        </nav>
      </aside>

      <main className="content">
        <header className="toolbar">
          <h1>{tab === 'windows' ? '小窗' : '配置'}</h1>
          <div className="toolbar-right">
            {(saved || status) && <span className="status">{saved ? '已保存 ✓' : status}</span>}
            <button className="primary" onClick={save}>
              保存
            </button>
          </div>
        </header>

        <div className="scroll">
          {issues.length > 0 && (
            <div className="issues">
              <strong>以下快捷键未生效：</strong>
              <ul>
                {issues.map((issue, index) => (
                  <li key={`${issue.hotkey}-${index}`}>
                    <code>{issue.hotkey || '(未设置)'}</code> — {issue.reason}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {tab === 'windows' ? (
            <>
              {draft.windows.map((item) => (
                <section className="group" key={item.id}>
                  <div className="group-head">
                    <input
                      className="group-title"
                      value={item.name}
                      placeholder="小窗名称"
                      onChange={(e) => updateWindow(item.id, { name: e.target.value })}
                    />
                    <button className="subtle danger" onClick={() => removeWindow(item.id)}>
                      删除
                    </button>
                  </div>

                  <div className="row">
                    <span className="row-label">网址</span>
                    <div className="row-control">
                      <input
                        value={item.url}
                        placeholder="https://..."
                        spellCheck={false}
                        onChange={(e) => updateWindow(item.id, { url: e.target.value })}
                      />
                    </div>
                  </div>

                  <div className="row">
                    <span className="row-label">显示 / 隐藏</span>
                    <div className="row-control">
                      <HotkeyInput
                        value={item.toggleHotkey}
                        onChange={(toggleHotkey) => updateWindow(item.id, { toggleHotkey })}
                      />
                      <span className="spacer" />
                      <button
                        className={`switch${visibility[item.id] ? ' on' : ''}`}
                        role="switch"
                        aria-checked={Boolean(visibility[item.id])}
                        title="显示 / 隐藏该小窗"
                        onClick={() => void toggleWindow(item.id)}
                      >
                        <span className="knob" />
                      </button>
                    </div>
                  </div>

                  <div className="sub-head">
                    <span>快捷键动作</span>
                    <button className="subtle" onClick={() => addAction(item.id)}>
                      添加
                    </button>
                  </div>

                  {item.hotkeys.map((action) => (
                    <div className="action" key={action.id}>
                      <div className="row">
                        <span className="row-label">名称</span>
                        <div className="row-control">
                          <input
                            value={action.name}
                            placeholder="动作名称"
                            onChange={(e) =>
                              updateAction(item.id, action.id, { name: e.target.value })
                            }
                          />
                        </div>
                      </div>

                      <div className="row">
                        <span className="row-label">快捷键</span>
                        <div className="row-control">
                          <HotkeyInput
                            value={action.hotkey}
                            onChange={(hotkey) => updateAction(item.id, action.id, { hotkey })}
                          />
                        </div>
                      </div>

                      <div className="row">
                        <span className="row-label">代码</span>
                        <div className="row-control">
                          <button
                            className={`code${action.code.trim() ? ' has-code' : ''}`}
                            onClick={() =>
                              setEditing({ windowId: item.id, actionId: action.id })
                            }
                          >
                            {action.code.trim() ? '编辑代码…' : '设置代码…'}
                          </button>
                          <span className="spacer" />
                          <button className="subtle" onClick={() => void testAction(item.id, action)}>
                            测试
                          </button>
                          <button
                            className="subtle danger"
                            onClick={() => removeAction(item.id, action.id)}
                          >
                            删除
                          </button>
                        </div>
                      </div>
                    </div>
                  ))}

                  {item.hotkeys.length === 0 && (
                    <div className="row">
                      <span className="row-label" />
                      <div className="row-control">
                        <span className="muted">暂无快捷键动作</span>
                      </div>
                    </div>
                  )}
                </section>
              ))}

              <button className="add-window" onClick={addWindow}>
                添加小窗
              </button>
            </>
          ) : (
            <section className="group">
              <div className="row">
                <span className="row-label">导出到剪贴板</span>
                <div className="row-control">
                  <span className="muted">把当前配置（含未保存修改）复制为 JSON</span>
                  <span className="spacer" />
                  <button onClick={() => void exportConfig()}>导出</button>
                </div>
              </div>
              <div className="row">
                <span className="row-label">从剪贴板导入</span>
                <div className="row-control">
                  <span className="muted">先删除所有小窗，再用剪贴板数据替换</span>
                  <span className="spacer" />
                  <button onClick={() => void importConfig()}>导入</button>
                </div>
              </div>
            </section>
          )}
        </div>
      </main>

      {editing && editingAction && (
        <Suspense fallback={null}>
          <CodeEditorModal
            title={`编辑代码 · ${editingWindow?.name ?? ''} / ${editingAction.name}`}
            value={editingAction.code}
            onCancel={() => setEditing(null)}
            onConfirm={(code) => {
              updateAction(editing.windowId, editing.actionId, { code })
              setEditing(null)
            }}
          />
        </Suspense>
      )}
    </div>
  )
}
