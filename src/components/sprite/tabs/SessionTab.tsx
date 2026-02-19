import { useEffect } from 'react'
import type { SpriteInfo } from '../../../types/sprite'
import { useSpriteStore } from '../../../stores/spriteStore'

export function SessionTab({ sprite }: { sprite: SpriteInfo }) {
  const { execSessions, listExecSessions, killExecSession, getOp } = useSpriteStore()

  const name     = sprite.name
  const sessions = execSessions[name] ?? []
  const listOp   = getOp(`${name}:exec-sessions`)

  const isReachable = sprite.status === 'warm' || sprite.status === 'running'

  useEffect(() => {
    if (isReachable) listExecSessions(name)
  }, [name, isReachable])

  const handleKill = (sessionId: string) =>
    killExecSession(name, sessionId, 'SIGTERM')

  if (!isReachable) {
    return (
      <div className="text-xs text-zinc-600 space-y-1">
        <p>Sprite is <span className="text-zinc-400 font-mono">{sprite.status}</span> — exec sessions are only available when warm or running.</p>
        <p className="text-zinc-700">Open a terminal to wake this sprite.</p>
      </div>
    )
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs text-zinc-500">{sessions.length} session{sessions.length !== 1 ? 's' : ''}</span>
        <button
          onClick={() => listExecSessions(name)}
          className="text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
        >
          Refresh
        </button>
      </div>

      {listOp.loading && <p className="text-xs text-zinc-500">Loading…</p>}
      {listOp.error   && <p className="text-xs text-red-400">{listOp.error}</p>}

      {sessions.length === 0 && !listOp.loading && (
        <p className="text-xs text-zinc-600">No active sessions.</p>
      )}

      <div className="space-y-1">
        {sessions.map(sess => {
          const sid    = String(sess.id)
          const killOp = getOp(`${name}:kill-${sid}`)
          return (
            <div key={sid} className="flex items-center gap-2 p-2 rounded bg-zinc-800/50 hover:bg-zinc-800 transition-colors group">
              <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${sess.is_active ? 'bg-green-500' : 'bg-zinc-600'}`} />
              <div className="min-w-0 flex-1">
                <p className="text-xs text-zinc-300 truncate font-mono">{sess.command ?? '—'}</p>
                {sess.workdir && <p className="text-xs text-zinc-600 truncate">{sess.workdir}</p>}
              </div>
              {sess.is_active && (
                <button
                  onClick={() => handleKill(sid)}
                  disabled={killOp.loading}
                  className="shrink-0 px-2 py-0.5 text-xs text-zinc-400 hover:text-red-400 bg-zinc-700 hover:bg-red-400/10 rounded opacity-0 group-hover:opacity-100 transition-all disabled:opacity-40"
                >
                  {killOp.loading ? 'Killing…' : 'Kill'}
                </button>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
