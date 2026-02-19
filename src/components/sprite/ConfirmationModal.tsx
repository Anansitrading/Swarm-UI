import { useState, useEffect } from 'react'
import { useModalStore } from '../../stores/modalStore'

export function ConfirmationModal() {
  const { state, accept, cancel } = useModalStore()
  const [input, setInput] = useState('')

  // Reset input when modal opens
  useEffect(() => {
    if (state.kind === 'prompt') setInput(state.opts.defaultValue ?? '')
  }, [state.kind])

  if (state.kind === 'none') return null

  const isDanger    = state.kind === 'confirm' && state.opts.danger
  const isPrompt    = state.kind === 'prompt'
  const title       = state.opts.title
  const body        = state.kind === 'confirm' ? state.opts.body : undefined
  const label       = state.kind === 'confirm' ? (state.opts.confirmLabel ?? 'Confirm') : 'OK'
  const placeholder = isPrompt ? state.opts.placeholder : undefined

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onMouseDown={(e) => e.target === e.currentTarget && cancel()}
    >
      <div className="bg-zinc-900 border border-zinc-700 rounded-lg p-5 w-[360px] shadow-2xl">
        <h2 className="text-sm font-semibold text-white mb-1">{title}</h2>
        {body && <p className="text-xs text-zinc-400 mb-4">{body}</p>}

        {isPrompt && (
          <input
            autoFocus
            className="w-full bg-zinc-800 border border-zinc-600 rounded px-3 py-2 text-sm text-white mt-2 mb-4 focus:outline-none focus:border-zinc-400"
            placeholder={placeholder}
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter') accept(input)
              if (e.key === 'Escape') cancel()
            }}
          />
        )}

        <div className="flex justify-end gap-2 mt-4">
          <button
            className="px-3 py-1.5 text-xs rounded bg-zinc-700 hover:bg-zinc-600 text-white transition-colors"
            onClick={cancel}
          >
            Cancel
          </button>
          <button
            className={`px-3 py-1.5 text-xs rounded text-white font-medium transition-colors ${
              isDanger ? 'bg-red-600 hover:bg-red-500' : 'bg-blue-600 hover:bg-blue-500'
            }`}
            onClick={() => isPrompt ? accept(input) : accept()}
          >
            {label}
          </button>
        </div>
      </div>
    </div>
  )
}
