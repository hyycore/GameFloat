import { useState } from 'react'
import CodeMirror from '@uiw/react-codemirror'
import { javascript } from '@codemirror/lang-javascript'

interface Props {
  title: string
  value: string
  onCancel: () => void
  onConfirm: (value: string) => void
}

export default function CodeEditorModal({ title, value, onCancel, onConfirm }: Props) {
  const [code, setCode] = useState(value)

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal" onClick={(event) => event.stopPropagation()}>
        <div className="modal-head">
          <h3>{title}</h3>
          <button className="icon" onClick={onCancel}>
            ✕
          </button>
        </div>
        <div className="modal-body">
          <CodeMirror
            value={code}
            height="380px"
            theme="dark"
            extensions={[javascript()]}
            onChange={(next) => setCode(next)}
          />
        </div>
        <div className="modal-foot">
          <button onClick={onCancel}>取消</button>
          <button className="primary" onClick={() => onConfirm(code)}>
            确定
          </button>
        </div>
      </div>
    </div>
  )
}
