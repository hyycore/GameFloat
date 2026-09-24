import { useEffect, useState } from 'react'

const MODIFIER_KEYS = new Set(['Control', 'Alt', 'Shift', 'Meta', 'AltGraph'])

const CODE_MAP: Record<string, string> = {
  Minus: '-',
  Equal: '=',
  BracketLeft: '[',
  BracketRight: ']',
  Backslash: '\\',
  Semicolon: ';',
  Quote: "'",
  Comma: ',',
  Period: '.',
  Slash: '/',
  Backquote: '`'
}

const SPECIAL_MAP: Record<string, string> = {
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  ' ': 'Space',
  Enter: 'Enter',
  Escape: 'Esc',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Insert: 'Insert',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown'
}

function normalizeKey(event: KeyboardEvent): string | null {
  const { code, key } = event
  if (/^Key[A-Z]$/.test(code)) return code.slice(3)
  if (/^Digit[0-9]$/.test(code)) return code.slice(5)
  if (/^Numpad[0-9]$/.test(code)) return `num${code.slice(6)}`
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) return key
  if (CODE_MAP[code]) return CODE_MAP[code]
  return SPECIAL_MAP[key] ?? null
}

function toAccelerator(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key)) return null
  const key = normalizeKey(event)
  if (!key) return null

  const parts: string[] = []
  if (event.ctrlKey) parts.push('Ctrl')
  if (event.altKey) parts.push('Alt')
  if (event.shiftKey) parts.push('Shift')
  if (event.metaKey) parts.push('Super')

  const isFunctionKey = /^F([1-9]|1[0-9]|2[0-4])$/.test(key)
  if (parts.length === 0 && !isFunctionKey) return null

  parts.push(key)
  return parts.join('+')
}

interface Props {
  value: string
  onChange: (value: string) => void
}

export default function HotkeyInput({ value, onChange }: Props) {
  const [recording, setRecording] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!recording) return undefined
    const handler = (event: KeyboardEvent): void => {
      event.preventDefault()
      event.stopPropagation()
      if (event.key === 'Escape') {
        setRecording(false)
        setError('')
        return
      }
      const accelerator = toAccelerator(event)
      if (!accelerator) {
        setError('请同时按住 Ctrl / Alt / Shift 等修饰键')
        return
      }
      onChange(accelerator)
      setRecording(false)
      setError('')
    }
    window.addEventListener('keydown', handler, true)
    return () => window.removeEventListener('keydown', handler, true)
  }, [recording, onChange])

  return (
    <div className="hotkey-input">
      <button
        type="button"
        className={recording ? 'recording' : ''}
        onClick={() => {
          setRecording((current) => !current)
          setError('')
        }}
      >
        {recording ? '请按下按键…（Esc 取消）' : value || '点击录制快捷键'}
      </button>
      {value && !recording && (
        <button type="button" className="clear" onClick={() => onChange('')}>
          清除
        </button>
      )}
      {error && <span className="hotkey-error">{error}</span>}
    </div>
  )
}
