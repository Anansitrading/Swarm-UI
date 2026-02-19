import { useEffect } from 'react'
import type { SpriteInfo } from '../../../types/sprite'
import { useSpriteStore } from '../../../stores/spriteStore'
import { useModalStore }  from '../../../stores/modalStore'

export function CheckpointTab({ sprite }: { sprite: SpriteInfo }) {
  const { checkpoints, listCheckpoints, createCheckpoint, restoreCheckpoint, getOp } = useSpriteStore()
  const { confirm, prompt } = useModalStore()

  const name   = sprite.name
  const cps    = checkpoints[name] ?? []
  const listOp = getOp(`${name}:checkpoints`)
  const createOp = getOp(`${name}:checkpoint-create`)
  const restoreOp = getOp(`${name}:checkpoint-restore`)

  const isReachable = sprite.status === 'warm' || sprite.status === 'running'

  useEffect(() => {
    if (isReachable) listCheckpoints(name)
  }, [name, isReachable])

  const handleCreate = async () => {
    const comment = await prompt({ title: 'New checkpoint', placeholder: 'Description (optional)' })
    if (comment === null) return
    await createCheckpoint(name, comment || undefined)
  }

  const handleRestore = async (checkpointId: string) => {
    const ok = await confirm({
      title: 'Restore checkpoint?',
      body: 'This will replace the current sprite state. This cannot be undone.',
      danger: true,
      confirmLabel: 'Restore',
    })
    if (!ok) return
    await restoreCheckpoint(name, checkpointId)
  }

  return (
    <div className="space-y-3">
      {/* Create button */}
      <div className="flex items-center justify-between">
        <span className="text-xs text-zinc-500">{cps.length} checkpoint{cps.length !== 1 ? 's' : ''}</span>
        <button
          onClick={handleCreate}
          disabled={createOp.loading}
          className="px-2.5 py-1 text-xs bg-zinc-700 hover:bg-zinc-600 text-white rounded disabled:opacity-40 transition-colors"
        >
          {createOp.loading ? 'Creating…' : '+ New checkpoint'}
        </button>
      </div>

      {/* Create progress */}
      {createOp.progress.length > 0 && (
        <ProgressLog lines={createOp.progress} error={createOp.error} />
      )}

      {/* Restore progress */}
      {restoreOp.loading && (
        <ProgressLog lines={restoreOp.progress} error={restoreOp.error} label="Restoring…" />
      )}

      {/* List */}
      {listOp.loading && <p className="text-xs text-zinc-500">Loading…</p>}
      {listOp.error   && <p className="text-xs text-red-400">{listOp.error}</p>}

      {cps.length === 0 && !listOp.loading && (
        <p className="text-xs text-zinc-600">No checkpoints yet.</p>
      )}

      <div className="space-y-1">
        {cps.map(cp => (
          <div key={cp.id} className="flex items-start justify-between gap-2 p-2 rounded bg-zinc-800/50 hover:bg-zinc-800 transition-colors group">
            <div className="min-w-0">
              <p className="text-xs text-zinc-300 truncate">{cp.comment || <span className="text-zinc-500 italic">No comment</span>}</p>
              <p className="text-xs text-zinc-600 mt-0.5">{cp.id.slice(0, 12)}… · {formatDate(cp.created_at ?? cp.create_time)}</p>
            </div>
            <button
              onClick={() => handleRestore(cp.id)}
              disabled={restoreOp.loading}
              className="shrink-0 px-2 py-0.5 text-xs text-zinc-400 hover:text-white bg-zinc-700 hover:bg-zinc-600 rounded opacity-0 group-hover:opacity-100 transition-all disabled:opacity-40"
            >
              Restore
            </button>
          </div>
        ))}
      </div>
    </div>
  )
}

function ProgressLog({ lines, error, label }: { lines: string[]; error: string | null; label?: string }) {
  return (
    <div className="bg-zinc-950 rounded p-2 font-mono text-xs space-y-0.5 max-h-28 overflow-y-auto">
      {label && <p className="text-zinc-500">{label}</p>}
      {lines.map((l, i) => <p key={i} className="text-zinc-300">{l}</p>)}
      {error && <p className="text-red-400">{error}</p>}
    </div>
  )
}

function formatDate(iso?: string | null) {
  if (!iso) return '—'
  return new Date(iso).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' })
}
