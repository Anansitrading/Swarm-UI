import { useState, useCallback } from 'react'
import type { SpriteInfo, ServiceStreamEvent } from '../../../types/sprite'
import { useSpriteStore } from '../../../stores/spriteStore'
import { useAutoWake } from '../hooks/useAutoWake'
import { WakingOverlay } from '../WakingOverlay'

export function ServiceTab({ sprite }: { sprite: SpriteInfo }) {
  const { services, serviceLogs, listServices, startService, stopService, getServiceLogs, getOp } = useSpriteStore()
  const [openLogService, setOpenLogService] = useState<string | null>(null)

  const name = sprite.name
  const svcs = services[name] ?? []
  const listOp = getOp(`${name}:services`)

  const onReady = useCallback(() => listServices(name), [name])
  const { wakeState, retry } = useAutoWake(name, sprite.status, onReady)

  const handleToggle = async (serviceName: string, currentStatus: string) => {
    if (currentStatus === 'running') await stopService(name, serviceName)
    else                             await startService(name, serviceName)
  }

  const handleLogs = (serviceName: string) => {
    if (openLogService === serviceName) {
      setOpenLogService(null)
      return
    }
    setOpenLogService(serviceName)
    getServiceLogs(name, serviceName)
  }

  if (wakeState !== 'ready') {
    return <WakingOverlay wakeState={wakeState} retry={retry} />
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs text-zinc-500">{svcs.length} service{svcs.length !== 1 ? 's' : ''}</span>
        <button onClick={() => listServices(name)} className="text-xs text-zinc-500 hover:text-zinc-300 transition-colors">
          Refresh
        </button>
      </div>

      {listOp.loading && <p className="text-xs text-zinc-500">Loading…</p>}
      {listOp.error   && <p className="text-xs text-red-400">{listOp.error}</p>}

      {svcs.length === 0 && !listOp.loading && (
        <p className="text-xs text-zinc-600">No services configured.</p>
      )}

      <div className="space-y-1">
        {svcs.map(svc => {
          const startOp = getOp(`${name}:service-start-${svc.name}`)
          const stopOp  = getOp(`${name}:service-stop-${svc.name}`)
          const logsOp  = getOp(`${name}:service-logs-${svc.name}`)
          const isRunning = svc.state?.status === 'running'
          const isBusy    = startOp.loading || stopOp.loading
          const logKey    = `${name}:${svc.name}`
          const logs      = serviceLogs[logKey] ?? []
          const logsOpen  = openLogService === svc.name

          return (
            <div key={svc.name} className="rounded bg-zinc-800/50">
              {/* Service row */}
              <div className="flex items-center gap-2 p-2 hover:bg-zinc-800 transition-colors">
                <StatusDot status={svc.state?.status ?? 'stopped'} />

                <div className="min-w-0 flex-1">
                  <p className="text-xs text-zinc-300 font-medium">{svc.name}</p>
                  <p className="text-xs text-zinc-600 truncate font-mono">{svc.cmd}</p>
                </div>

                <div className="flex items-center gap-1 shrink-0">
                  <button
                    onClick={() => handleToggle(svc.name, svc.state?.status ?? 'stopped')}
                    disabled={isBusy}
                    className={`px-2 py-0.5 text-xs rounded transition-colors disabled:opacity-40 ${
                      isRunning
                        ? 'text-red-400 bg-red-400/10 hover:bg-red-400/20'
                        : 'text-green-400 bg-green-400/10 hover:bg-green-400/20'
                    }`}
                  >
                    {isBusy ? '…' : isRunning ? 'Stop' : 'Start'}
                  </button>

                  <button
                    onClick={() => handleLogs(svc.name)}
                    className={`px-2 py-0.5 text-xs rounded transition-colors ${
                      logsOpen
                        ? 'text-zinc-200 bg-zinc-600'
                        : 'text-zinc-400 bg-zinc-700 hover:bg-zinc-600'
                    }`}
                  >
                    Logs
                  </button>
                </div>
              </div>

              {/* Inline log viewer */}
              {logsOpen && (
                <div className="border-t border-zinc-700">
                  <div className="flex items-center justify-between px-2 py-1 border-b border-zinc-700/50">
                    <span className="text-xs text-zinc-500">{logsOp.loading ? 'Streaming…' : `${logs.length} lines`}</span>
                    <button
                      onClick={() => getServiceLogs(name, svc.name)}
                      className="text-xs text-zinc-500 hover:text-zinc-300"
                    >
                      Refresh
                    </button>
                  </div>
                  <div className="p-2 max-h-48 overflow-y-auto font-mono text-xs space-y-0.5 bg-zinc-950">
                    {logs.length === 0 && !logsOp.loading && (
                      <p className="text-zinc-600">No log output.</p>
                    )}
                    {logs.map((e, i) => <LogLine key={i} event={e} />)}
                  </div>
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}

function LogLine({ event }: { event: ServiceStreamEvent }) {
  const text  = event.data ?? event.type
  const color =
    event.type === 'stderr' ? 'text-amber-400' :
    event.type === 'error'  ? 'text-red-400' :
    event.type === 'stopped'? 'text-zinc-500' :
                              'text-zinc-300'
  return <p className={`whitespace-pre-wrap break-all ${color}`}>{text}</p>
}

function StatusDot({ status }: { status: string }) {
  const color =
    status === 'running'  ? 'bg-green-500' :
    status === 'starting' ? 'bg-yellow-500 animate-pulse' :
    status === 'stopping' ? 'bg-orange-500 animate-pulse' :
    status === 'failed'   ? 'bg-red-500' :
                            'bg-zinc-600'
  return <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${color}`} />
}
