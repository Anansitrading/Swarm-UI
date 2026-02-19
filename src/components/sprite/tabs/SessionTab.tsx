import { useEffect, useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { SpriteInfo } from '../../../types/sprite'

interface ClaudeSession {
  sessionId:    string
  project:      string
  firstMessage: string
  timestamp:    number   // Unix ms
  messageCount: number
}

export function SessionTab({ sprite }: { sprite: SpriteInfo }) {
  const [sessions, setSessions] = useState<ClaudeSession[]>([])
  const [loading, setLoading]   = useState(false)
  const [error, setError]       = useState<string | null>(null)

  const fetchSessions = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const output = await invoke<string>('sprite_exec_command', {
        name: sprite.name,
        command: 'cat ~/.claude/history.jsonl 2>/dev/null || echo "__NOTFOUND__"',
      })

      if (output.includes('__NOTFOUND__') || output.trim() === '') {
        setSessions([])
        return
      }

      // Each line is a user message: {display, timestamp, project, sessionId}
      // Aggregate by sessionId to build session list
      const sessMap = new Map<string, ClaudeSession>()

      for (const line of output.split('\n')) {
        if (!line.trim()) continue
        try {
          const entry = JSON.parse(line)
          const sid = entry.sessionId
          if (!sid) continue

          const existing = sessMap.get(sid)
          if (!existing) {
            sessMap.set(sid, {
              sessionId:    sid,
              project:      entry.project ?? '',
              firstMessage: entry.display ?? '',
              timestamp:    entry.timestamp ?? 0,
              messageCount: 1,
            })
          } else {
            existing.messageCount++
            if ((entry.timestamp ?? 0) > existing.timestamp) {
              existing.timestamp = entry.timestamp
            }
          }
        } catch { /* skip malformed lines */ }
      }

      // Sort by most recent activity
      const sorted = [...sessMap.values()].sort((a, b) => b.timestamp - a.timestamp)
      setSessions(sorted)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [sprite.name])

  useEffect(() => { fetchSessions() }, [fetchSessions])

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs text-zinc-500">{sessions.length} session{sessions.length !== 1 ? 's' : ''}</span>
        <button onClick={fetchSessions} className="text-xs text-zinc-500 hover:text-zinc-300 transition-colors">
          Refresh
        </button>
      </div>

      {loading && <p className="text-xs text-zinc-500">Reading ~/.claude/history.jsonl…</p>}
      {error   && <p className="text-xs text-red-400">{error}</p>}

      {!loading && sessions.length === 0 && !error && (
        <p className="text-xs text-zinc-600">No Claude sessions found on this sprite.</p>
      )}

      <div className="space-y-1">
        {sessions.map(sess => (
          <div key={sess.sessionId} className="p-2 rounded bg-zinc-800/50 hover:bg-zinc-800 transition-colors">
            <p className="text-xs text-zinc-300 truncate">
              {sess.firstMessage || <span className="text-zinc-500 italic">No message</span>}
            </p>
            <div className="flex items-center gap-2 mt-0.5">
              <span className="text-xs text-zinc-600 truncate font-mono">
                {sess.project.replace('/home/sprite', '~').replace('/home/user', '~')}
              </span>
              <span className="text-xs text-zinc-700 shrink-0">
                {sess.messageCount} msg{sess.messageCount !== 1 ? 's' : ''}
              </span>
              <span className="text-xs text-zinc-700 shrink-0">
                {new Date(sess.timestamp).toLocaleDateString(undefined, {
                  month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
                })}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
