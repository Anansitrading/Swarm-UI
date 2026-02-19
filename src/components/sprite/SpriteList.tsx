import { useEffect, useState, useCallback } from 'react'
import { useSpriteStore } from '../../stores/spriteStore'
import { useModalStore } from '../../stores/modalStore'
import { useLayoutStore } from '../../stores/layoutStore'
import { SpriteActionBar } from './SpriteActionBar'
import { SpriteExpandedPanel } from './SpriteExpandedPanel'
import { StatusBadge } from './StatusBadge'

interface SpriteListProps {
  onSelect: (name: string) => void
}

export function SpriteList({ onSelect }: SpriteListProps) {
  const { sprites, loading, error, fetchSprites, createSprite, clearError } =
    useSpriteStore()
  const { setSidebarTab } = useLayoutStore()
  const { prompt } = useModalStore()
  const [expanded, setExpanded] = useState<Set<string>>(new Set())

  useEffect(() => {
    fetchSprites()
  }, [fetchSprites])

  const toggleExpand = useCallback((name: string) => {
    setExpanded(prev => {
      const next = new Set(prev)
      if (next.has(name)) next.delete(name)
      else next.add(name)
      return next
    })
  }, [])

  const handleCreate = async () => {
    const name = await prompt({
      title: 'Create sprite',
      placeholder: 'Sprite name…',
    })
    if (!name) return
    try {
      await createSprite(name)
    } catch {
      // error visible in store
    }
  }

  // ── Loading state ──────────────────────────────────────────────────

  if (loading && sprites.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-zinc-500 text-sm gap-2">
        <div className="animate-spin h-5 w-5 border-2 border-zinc-400 border-t-transparent rounded-full" />
        <span>Loading sprites…</span>
      </div>
    )
  }

  // ── Error state ────────────────────────────────────────────────────

  if (error) {
    const isNotConfigured = error.includes('not configured')
    return (
      <div className="p-4 space-y-3">
        <div className="text-xs text-red-400 bg-red-400/10 border border-red-400/20 rounded p-3">
          {isNotConfigured
            ? 'Sprites API not configured.'
            : `Error: ${error}`}
        </div>
        {isNotConfigured ? (
          <button
            onClick={() => setSidebarTab('settings')}
            className="w-full px-3 py-2 text-xs bg-zinc-700 hover:bg-zinc-600 text-white rounded transition-colors"
          >
            Configure API in Settings
          </button>
        ) : (
          <button
            onClick={() => { clearError(); fetchSprites() }}
            className="w-full px-3 py-2 text-xs bg-zinc-700 hover:bg-zinc-600 text-white rounded transition-colors"
          >
            Retry
          </button>
        )}
      </div>
    )
  }

  // ── Empty state ────────────────────────────────────────────────────

  if (sprites.length === 0) {
    return (
      <div className="p-4 text-center space-y-3">
        <p className="text-sm text-zinc-500">No sprites found.</p>
        <button
          onClick={handleCreate}
          className="px-3 py-2 text-xs bg-zinc-700 hover:bg-zinc-600 text-white rounded transition-colors"
        >
          + Create sprite
        </button>
      </div>
    )
  }

  // ── Sprite list ────────────────────────────────────────────────────

  return (
    <div className="p-2 space-y-0.5">
      {/* Header bar */}
      <div className="flex items-center justify-between px-1 mb-2">
        <span className="text-[10px] text-zinc-500 uppercase tracking-wide">
          {sprites.length} sprite{sprites.length !== 1 ? 's' : ''}
        </span>
        <div className="flex items-center gap-2">
          <button
            onClick={handleCreate}
            className="text-[10px] text-blue-400 hover:text-blue-300 transition-colors"
          >
            + New
          </button>
          <button
            onClick={() => fetchSprites()}
            className="text-[10px] text-zinc-500 hover:text-zinc-300 transition-colors"
          >
            Refresh
          </button>
        </div>
      </div>

      {/* Cards */}
      {sprites.map(sprite => (
        <div key={sprite.name} className="rounded-lg border border-zinc-800 bg-zinc-900/50 overflow-hidden">
          {/* Collapsed header */}
          <div
            className="flex items-center gap-2 px-3 py-2.5 cursor-pointer hover:bg-zinc-800/40 transition-colors"
            onClick={() => toggleExpand(sprite.name)}
          >
            <ChevronIcon expanded={expanded.has(sprite.name)} />
            <span className="text-sm text-zinc-200 font-medium truncate flex-1 font-mono" title={sprite.name}>
              {sprite.name}
            </span>
            {sprite.region && (
              <span className="text-[10px] text-zinc-600 mr-1">{sprite.region}</span>
            )}
            <StatusBadge status={sprite.status} />
            <SpriteActionBar sprite={sprite} onTerminal={() => onSelect(sprite.name)} />
          </div>

          {/* Expanded panel */}
          {expanded.has(sprite.name) && <SpriteExpandedPanel sprite={sprite} />}
        </div>
      ))}
    </div>
  )
}

function ChevronIcon({ expanded }: { expanded: boolean }) {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 16 16"
      fill="currentColor"
      className={`text-zinc-500 transition-transform ${expanded ? 'rotate-90' : ''}`}
    >
      <path d="M6 4l4 4-4 4" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}
