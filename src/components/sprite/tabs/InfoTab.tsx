import { useEffect } from 'react'
import type { SpriteInfo } from '../../../types/sprite'
import { useSpriteStore } from '../../../stores/spriteStore'

export function InfoTab({ sprite }: { sprite: SpriteInfo }) {
  const { details, getSprite, getOp } = useSpriteStore()

  const name   = sprite.name
  const detail = details[name]
  const op     = getOp(`${name}:detail`)

  useEffect(() => { getSprite(name) }, [name])

  if (op.loading && !detail) {
    return <p className="text-xs text-zinc-500">Loading…</p>
  }

  if (op.error) {
    return <p className="text-xs text-red-400">{op.error}</p>
  }

  if (!detail) return null

  return (
    <div className="space-y-2 text-xs">
      <Row label="ID"           value={detail.id} />
      <Row label="Organization" value={detail.organization} />
      <Row label="URL"          value={detail.url} />
      <Row label="Auth"         value={detail.url_settings?.auth} />
      <Row label="Created"      value={formatDate(detail.created_at)} />
      <Row label="Updated"      value={formatDate(detail.updated_at)} />
      <Row label="Last started" value={formatDate(detail.last_started_at)} />
      <Row label="Last active"  value={formatDate(detail.last_active_at)} />
    </div>
  )
}

function Row({ label, value }: { label: string; value?: string | null }) {
  if (!value) return null
  return (
    <div className="flex items-start gap-3">
      <span className="text-zinc-500 w-24 shrink-0">{label}</span>
      <span className="text-zinc-300 font-mono break-all">{value}</span>
    </div>
  )
}

function formatDate(iso?: string | null) {
  if (!iso) return undefined
  try {
    return new Date(iso).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' })
  } catch {
    return iso
  }
}
