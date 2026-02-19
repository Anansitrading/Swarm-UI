import { useEffect, useState, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useSpriteStore } from '../../../stores/spriteStore'

type WakeState = 'idle' | 'waking' | 'ready' | 'failed'

/**
 * Auto-wakes a cold/tracked sprite when a tab mounts, then calls `onReady`.
 * Returns the current wake state for rendering loading/failed UI.
 */
export function useAutoWake(
  spriteName: string,
  spriteStatus: string,
  onReady: () => void,
) {
  const { fetchSprites } = useSpriteStore()
  const isReachable = spriteStatus === 'warm' || spriteStatus === 'running'
  const [wakeState, setWakeState] = useState<WakeState>(isReachable ? 'ready' : 'idle')
  const retriesRef = useRef(0)

  const doWake = async (cancelled: () => boolean) => {
    try {
      await invoke('sprite_exec_command', { name: spriteName, command: 'true' })
    } catch {
      // Expected on cold start — exec may time out while sprite boots
    }

    // Poll status until warm/running (max 20s)
    for (let i = 0; i < 10; i++) {
      if (cancelled()) return
      await new Promise(r => setTimeout(r, 2000))
      await fetchSprites()
      const current = useSpriteStore.getState().sprites.find(s => s.name === spriteName)
      if (current && (current.status === 'warm' || current.status === 'running')) break
    }

    if (cancelled()) return

    const current = useSpriteStore.getState().sprites.find(s => s.name === spriteName)
    if (current && (current.status === 'warm' || current.status === 'running')) {
      setWakeState('ready')
      onReady()
    } else {
      setWakeState('failed')
    }
  }

  useEffect(() => {
    if (isReachable) {
      setWakeState('ready')
      onReady()
      return
    }

    let cancelled = false
    setWakeState('waking')
    doWake(() => cancelled)
    return () => { cancelled = true }
  }, [spriteName])

  const retry = () => {
    retriesRef.current++
    setWakeState('waking')
    doWake(() => false)
  }

  return { wakeState, retry }
}
