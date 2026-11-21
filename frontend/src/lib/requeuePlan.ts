import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@/lib/tauri'
import type { RequeueStatus } from '@/types'

// How often the play plan is re-read, ms. The counter only moves when a
// game ends, so this is about freshness, not about precision.
export const REQUEUE_POLL_MS = 2000

/** The live play plan: how many games are done, how many are left, and
 *  when the batch must stop.
 *
 *  The plan lives in the backend (the counter) and in the config (the
 *  budget and the cutoff), and both are read fresh every time a game
 *  ends — so changing either takes effect on the next game without a
 *  restart. This hook only watches the counter.
 *
 *  The returned refresh is what to call after a write, so the UI does not
 *  have to wait a full poll cycle to show the new value.
 */
export function useRequeueStatus(
  enabled = true,
): [RequeueStatus | null, () => void] {
  const [plan, setPlan] = useState<RequeueStatus | null>(null)

  const refresh = useCallback(() => {
    invoke<RequeueStatus>('get_requeue_status')
      .then(setPlan)
      .catch(() => {
        /* the backend is not up yet — the next tick tries again */
      })
  }, [])

  useEffect(() => {
    if (!enabled) return
    let cancelled = false
    const tick = () => {
      invoke<RequeueStatus>('get_requeue_status')
        .then((s) => {
          if (!cancelled) setPlan(s)
        })
        .catch(() => {})
    }
    tick()
    const id = window.setInterval(tick, REQUEUE_POLL_MS)
    return () => {
      cancelled = true
      window.clearInterval(id)
    }
  }, [enabled])

  return [enabled ? plan : null, refresh]
}
