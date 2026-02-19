export function StatusBadge({ status }: { status: string }) {
  const style =
    status === 'running' ? 'bg-green-500/15 text-green-400 ring-green-500/30' :
    status === 'warm'    ? 'bg-amber-500/15 text-amber-400 ring-amber-500/30' :
    status === 'cold'    ? 'bg-zinc-500/15 text-zinc-400 ring-zinc-500/30' :
                           'bg-blue-500/15 text-blue-400 ring-blue-500/30'

  return (
    <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium ring-1 ring-inset ${style}`}>
      {status}
    </span>
  )
}
