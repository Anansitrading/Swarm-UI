import type { SpriteInfo } from '../../types/sprite'
import { useSpriteStore } from '../../stores/spriteStore'
import { useModalStore } from '../../stores/modalStore'

interface Props { sprite: SpriteInfo; onTerminal?: () => void; onDeleted?: () => void }

export function SpriteActionBar({ sprite, onTerminal, onDeleted }: Props) {
  const { createCheckpoint, deleteSprite, getOp } = useSpriteStore()
  const { confirm, prompt } = useModalStore()

  const checkpointOp = getOp(`${sprite.name}:checkpoint-create`)
  const isCreating   = checkpointOp.loading

  const handleTerminal = () => onTerminal?.()

  const handleCheckpoint = async () => {
    const comment = await prompt({
      title: `Checkpoint — ${sprite.name}`,
      placeholder: 'Optional comment…',
    })
    if (comment === null) return
    await createCheckpoint(sprite.name, comment || undefined)
  }

  const handleDelete = async () => {
    const ok = await confirm({
      title: `Delete ${sprite.name}?`,
      body: 'This will permanently destroy the sprite and all its data.',
      danger: true,
      confirmLabel: 'Delete',
    })
    if (!ok) return
    await deleteSprite(sprite.name)
    onDeleted?.()
  }

  return (
    <div className="flex items-center gap-1 ml-auto shrink-0">
      <ActionBtn
        title="Open terminal"
        onClick={handleTerminal}
        disabled={sprite.status === 'cold'}
      >
        <TerminalIcon />
      </ActionBtn>

      <ActionBtn
        title="Create checkpoint"
        onClick={handleCheckpoint}
        disabled={isCreating}
        loading={isCreating}
      >
        <CheckpointIcon />
      </ActionBtn>

      <ActionBtn
        title="Delete sprite"
        onClick={handleDelete}
        danger
      >
        <TrashIcon />
      </ActionBtn>
    </div>
  )
}

// ── Tiny icon button ──────────────────────────────────────────────────────

interface BtnProps {
  title: string
  onClick: () => void
  disabled?: boolean
  loading?: boolean
  danger?: boolean
  children: React.ReactNode
}

function ActionBtn({ title, onClick, disabled, loading, danger, children }: BtnProps) {
  return (
    <button
      title={title}
      disabled={disabled || loading}
      onClick={(e) => { e.stopPropagation(); onClick() }}
      className={`p-1.5 rounded transition-colors disabled:opacity-40 disabled:cursor-not-allowed ${
        danger
          ? 'text-zinc-500 hover:text-red-400 hover:bg-red-400/10'
          : 'text-zinc-500 hover:text-zinc-200 hover:bg-zinc-700'
      }`}
    >
      {loading ? <Spinner /> : children}
    </button>
  )
}

// ── Inline SVG icons ──────────────────────────────────────────────────────

const TerminalIcon = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
    <path d="M1.5 2A1.5 1.5 0 0 0 0 3.5v9A1.5 1.5 0 0 0 1.5 14h13a1.5 1.5 0 0 0 1.5-1.5v-9A1.5 1.5 0 0 0 14.5 2H1.5zm1.146 4.354a.5.5 0 0 1 0-.708l2-2a.5.5 0 1 1 .708.708L3.707 6l1.647 1.646a.5.5 0 0 1-.708.708l-2-2zM7 9.5a.5.5 0 0 1 .5-.5h3a.5.5 0 0 1 0 1h-3A.5.5 0 0 1 7 9.5z"/>
  </svg>
)

const CheckpointIcon = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
    <path d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1zm0 1a6 6 0 1 1 0 12A6 6 0 0 1 8 2zm0 2a.5.5 0 0 0-.5.5v4a.5.5 0 0 0 .146.354l2.5 2.5a.5.5 0 0 0 .708-.708L8.5 8.293V4.5A.5.5 0 0 0 8 4z"/>
  </svg>
)

const TrashIcon = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
    <path d="M5.5 5.5A.5.5 0 0 1 6 6v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5m2.5 0a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5m3 .5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0z"/>
    <path d="M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1zM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4zM2.5 3h11V2h-11z"/>
  </svg>
)

const Spinner = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" className="animate-spin">
    <path d="M8 1a7 7 0 0 1 7 7h-1A6 6 0 0 0 8 2V1z"/>
  </svg>
)
