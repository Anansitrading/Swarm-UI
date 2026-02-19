type WakeState = 'idle' | 'waking' | 'ready' | 'failed'

interface Props {
  wakeState: WakeState
  retry: () => void
}

export function WakingOverlay({ wakeState, retry }: Props) {
  if (wakeState === 'waking') {
    return (
      <div className="space-y-2">
        <div className="flex items-center gap-2 text-xs text-zinc-400">
          <svg className="animate-spin w-3 h-3 shrink-0" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 1a7 7 0 0 1 7 7h-1A6 6 0 0 0 8 2V1z"/>
          </svg>
          Waking sprite — this takes a few seconds…
        </div>
        <div className="h-1 bg-zinc-800 rounded overflow-hidden">
          <div className="h-full bg-zinc-600 rounded animate-pulse w-1/2" />
        </div>
      </div>
    )
  }

  if (wakeState === 'failed') {
    return (
      <div className="space-y-2 text-xs">
        <p className="text-red-400">Sprite didn't wake within 20s.</p>
        <button
          onClick={retry}
          className="px-2 py-1 text-xs bg-zinc-700 hover:bg-zinc-600 text-white rounded transition-colors"
        >
          Retry
        </button>
      </div>
    )
  }

  return null
}
