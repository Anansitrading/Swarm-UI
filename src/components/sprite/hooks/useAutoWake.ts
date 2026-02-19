import { useEffect, useState, useRef } from 'react'
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
  const { execOnSprite, fetchSprites } = useSpriteStore()
  const isReachable = spriteStatus === 'warm' || spriteStatus === 'running'
  const [wakeState, setWakeState] = useState<WakeState>(isReachable ? 'ready' : 'idle')
  const retriesRef = useRef(0)

  useEffect(() => {
    if (isReachable) {
      setWakeState('ready')
      onReady()
      return
    }

    // Cold/tracked sprite — wake it then fetch
    let cancelled = false
    setWakeState('waking')

    const wakeAndFetch = async () => {
      try {
        // Wake by running a no-op command — this starts the sprite
        await execOnSprite(spriteName, 'true')
      } catch {
        // Expected on cold start — exec may time out while sprite boots
      }

      // Poll status until warm/running (max 20s)
      for (let i = 0; i < 10; i++) {
        if (cancelled) return
        await new Promise(r => setTimeout(r, 2000))
        await fetchSprites()
        const current = useSpriteStore.getState().sprites.find(s => s.name === spriteName)
        if (current && (current.status === 'warm' || current.status === 'running')) break
      }

      if (cancelled) return

      const current = useSpriteStore.getState().sprites.find(s => s.name === spriteName)
      if (current && (current.status === 'warm' || current.status === 'running')) {
        setWakeState('ready')
        onReady()
      } else {
        setWakeState('failed')
      }
    }

    wakeAndFetch()
    return () => { cancelled = true }
  }, [spriteName])

  const retry = () => {
    retriesRef.current++
    setWakeState('idle')
    // Re-trigger by toggling to idle then waking on next tick
    setTimeout(() => {
      setWakeState('waking')
      const run = async () => {
        try {
          await execOnSprite(spriteName, 'true')
        } catch { /* expected */ }

        for (let i = 0; i < 10; i++) {
          await new Promise(r => setTimeout(r, 2000))
          await fetchSprites()
          const current = useSpriteStore.getState().sprites.find(s => s.name === spriteName)
          if (current && (current.status === 'warm' || current.status === 'running')) break
        }

        const current = useSpriteStore.getState().sprites.find(s => s.name === spriteName)
        if (current && (current.status === 'warm' || current.status === 'running')) {
          setWakeState('ready')
          onReady()
        } else {
          setWakeState('failed')
        }
      }
      run()
    }, 0)
  }

  return { wakeState, retry }
}
