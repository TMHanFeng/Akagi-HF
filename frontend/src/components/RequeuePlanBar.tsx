import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@/lib/tauri'
import { useRequeueStatus } from '@/lib/requeuePlan'

// The play-plan row under the overlay's suggestion list: how many games are
// left, a one-click "stop after the game in play", and the daily cutoff.
//
// Everything here changes the plan *while the bot is running*. The budget
// and the cutoff live in the config and are read afresh every time a game
// ends, so a change takes effect on the next game — nothing restarts, and
// the game in play is always finished.
//
// The counter is the backend's: `get_requeue_status` reports it, and it
// survives a restart of the app.

export function RequeuePlanBar({ enabled = true }: { enabled?: boolean }) {
  const { t } = useTranslation()
  const [plan, refresh] = useRequeueStatus(enabled)
  const [busy, setBusy] = useState(false)
  const cutoffRef = useRef<HTMLInputElement>(null)

  // The cutoff field is deliberately *uncontrolled*. A controlled time input
  // fights the browser: the hour and minute segments are edited separately,
  // and a controlled value would overwrite the segment the user is typing
  // in (or steal the focus) on every keystroke. Writing the DOM value
  // directly — and only while the field is not focused — keeps the field
  // in step with a cutoff changed elsewhere (the Settings page) without
  // getting in the way of an edit.
  useEffect(() => {
    const el = cutoffRef.current
    if (!el) return
    if (document.activeElement === el) return
    const next = plan?.stop_at ?? ''
    if (el.value !== next) el.value = next
  }, [plan?.stop_at])

  if (!enabled || !plan) return null

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true)
    try {
      await fn()
      refresh()
    } catch {
      /* a rejected write leaves the plan as it was; the next poll shows it */
    } finally {
      setBusy(false)
    }
  }

  const stopAfterCurrent = () =>
    run(() => invoke('stop_after_current_game'))

  const commitCutoff = (value: string | null) =>
    run(() =>
      invoke('set_requeue_plan', {
        patch: value ? { stop_at: value } : { clear_stop_at: true },
      }),
    )

  const statusText =
    plan.remaining == null
      ? t('overlay.plan_unlimited', { games: plan.games })
      : plan.remaining === 0
        ? t('overlay.plan_exhausted', { games: plan.games })
        : t('overlay.plan_status', { games: plan.games, remaining: plan.remaining })

  const title =
    plan.remaining == null
      ? t('overlay.plan_unlimited', { games: plan.games })
      : t('overlay.plan_status', { games: plan.games, remaining: plan.remaining ?? 0 })

  return (
    <div className="flex shrink-0 items-center gap-1 border-t border-border/60 px-1.5 py-1 text-[11px] leading-none">
      <span
        className={
          'min-w-0 flex-1 truncate tabular-nums ' +
          (plan.deadline_passed ? 'text-amber-400' : 'text-muted-foreground')
        }
        title={title}
      >
        {statusText}
      </span>
      <button
        type="button"
        onClick={stopAfterCurrent}
        disabled={busy}
        aria-label={t('overlay.plan_stop_after_current')}
        title={t('overlay.plan_stop_after_current')}
        className="shrink-0 cursor-pointer rounded border border-border px-1.5 py-0.5 text-[11px] text-foreground/90 hover:bg-accent disabled:opacity-50"
        style={{ pointerEvents: 'auto' }}
      >
        {t('overlay.plan_stop_after_current')}
      </button>
      <input
        ref={cutoffRef}
        type="time"
        defaultValue={plan.stop_at ?? ''}
        onChange={(e) => {
          const v = e.target.value
          // A complete time is committed straight away. An empty field is
          // the user clearing it — a cleared segment also reads as empty
          // while the rest is still being typed, so that is committed on
          // blur instead.
          if (v) void commitCutoff(v)
        }}
        onBlur={(e) => {
          if (!e.target.value) void commitCutoff(null)
        }}
        aria-label={t('overlay.plan_stop_at')}
        title={t('overlay.plan_stop_at')}
        className="shrink-0 cursor-pointer rounded border border-border bg-transparent px-1 py-0.5 text-[11px] tabular-nums text-foreground/90"
        style={{ pointerEvents: 'auto' }}
      />
    </div>
  )
}
