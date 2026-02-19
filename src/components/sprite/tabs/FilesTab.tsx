import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { SpriteInfo } from '../../../types/sprite'
import { useAutoWake } from '../hooks/useAutoWake'
import { WakingOverlay } from '../WakingOverlay'

interface FileEntry {
  name: string
  isDir: boolean
  isLink: boolean
  size: string
  perms: string
}

function parseLsLine(line: string): FileEntry | null {
  const trimmed = line.trim()
  if (!trimmed || trimmed.startsWith('total ')) return null

  const parts = trimmed.split(/\s+/)
  if (parts.length < 9) return null

  const name = parts.slice(8).join(' ')
  if (!name) return null

  return {
    name,
    isDir: trimmed.startsWith('d'),
    isLink: trimmed.startsWith('l'),
    size: parts[4] ?? '',
    perms: parts[0] ?? '',
  }
}

function formatSize(size: string): string {
  const n = parseInt(size, 10)
  if (isNaN(n)) return size
  if (n >= 1048576) return `${(n / 1048576).toFixed(1)}M`
  if (n >= 1024) return `${(n / 1024).toFixed(1)}K`
  return size
}

export function FilesTab({ sprite }: { sprite: SpriteInfo }) {
  const [entries, setEntries] = useState<FileEntry[]>([])
  const [cwd, setCwd]         = useState('~')
  const [loading, setLoading] = useState(false)
  const [error, setError]     = useState<string | null>(null)

  const fetchFiles = useCallback(async (dir = '~') => {
    setLoading(true)
    setError(null)
    try {
      const result = await invoke<string>('sprite_exec_command', {
        name: sprite.name,
        command: `ls -la ${dir} 2>&1`,
      })
      const parsed = result.split('\n').map(parseLsLine).filter(Boolean) as FileEntry[]
      setEntries(parsed)
      setCwd(dir)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [sprite.name])

  const onReady = useCallback(() => fetchFiles(), [fetchFiles])
  const { wakeState, retry } = useAutoWake(sprite.name, sprite.status, onReady)

  const navigate = (name: string) => {
    if (name === '.') return
    const next = name === '..'
      ? cwd.split('/').slice(0, -1).join('/') || '/'
      : cwd === '/' ? `/${name}` : `${cwd}/${name}`
    fetchFiles(next)
  }

  if (wakeState !== 'ready') {
    return <WakingOverlay wakeState={wakeState} retry={retry} />
  }

  return (
    <div className="space-y-2">
      {/* Breadcrumb + refresh */}
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs text-zinc-400 font-mono truncate flex-1" title={cwd}>{cwd}</span>
        <button onClick={() => fetchFiles(cwd)} className="text-xs text-zinc-500 hover:text-zinc-300 transition-colors shrink-0">
          Refresh
        </button>
      </div>

      {loading && <p className="text-xs text-zinc-500">Loading…</p>}
      {error   && <p className="text-xs text-red-400">{error}</p>}

      {!loading && entries.length > 0 && (
        <div className="bg-zinc-950 rounded p-1 max-h-64 overflow-y-auto text-xs">
          {entries.map((entry, i) => {
            const isSpecial = entry.name === '.' || entry.name === '..'

            return (
              <div key={i} className="flex items-center gap-1.5 px-1.5 py-0.5 hover:bg-zinc-800/60 rounded" title={entry.perms}>
                {/* Name — takes all available space */}
                <div className="flex-1 min-w-0 truncate">
                  {entry.isDir && !isSpecial ? (
                    <button
                      onClick={() => navigate(entry.name)}
                      className="text-blue-400 hover:text-blue-300 hover:underline truncate block"
                    >
                      {entry.name}/
                    </button>
                  ) : isSpecial && entry.name === '..' ? (
                    <button
                      onClick={() => navigate('..')}
                      className="text-zinc-500 hover:text-zinc-300 hover:underline"
                    >
                      ..
                    </button>
                  ) : entry.isLink ? (
                    <span className="text-cyan-400 truncate block">{entry.name}</span>
                  ) : (
                    <span className="text-zinc-300 truncate block">{entry.name}</span>
                  )}
                </div>
                {/* Size — right-aligned, compact */}
                {!isSpecial && (
                  <span className="text-zinc-600 text-[10px] shrink-0 tabular-nums">
                    {formatSize(entry.size)}
                  </span>
                )}
              </div>
            )
          })}
        </div>
      )}

      {!loading && entries.length === 0 && !error && (
        <p className="text-xs text-zinc-600">Empty directory.</p>
      )}
    </div>
  )
}
