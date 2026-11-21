import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@/lib/tauri'
import { useRequeueStatus } from '@/lib/requeuePlan'

// The play-plan strip under the overlay's suggestion list: how many games
// are left, a one-click "stop after the game in play", the deadline's day
// offset and its clock.
//
// Everything here changes the plan *while the bot is running*. The budget
// and the deadline live in the config and are read afresh every time a game
// ends, so a change takes effect on the next game — nothing restarts, and
// the game in play is always played to the end.
//
// The deadline is stored as an absolute moment; the day offset is only a way
// of *entering* it, and it is derived from the stored date when it is
// displayed — so the plan cannot slide forward by being read or re-saved.

/** Every control on the strip shares this: the same height, the same
 *  border, the same text size. A row of mismatched sizes reads as broken
 *  chrome rather than as controls. */
const CTRL =
  'h-[18px] shrink-0 rounded border border-border bg-transparent px-1 text-[11px] leading-none text-foreground/90 hover:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-border'

export function RequeuePlanBar({ enabled = true }: { enabled?: boolean }) {
  const { t } = useTranslation()
  const [plan, refresh] = useRequeueStatus(enabled)
  const [busy, setBusy] = useState(false)
  const cutoffRef = useRef<HTMLInputElement>(null)
  // The two number fields are edited through a local draft: the poll runs
  // every couple of seconds and would otherwise overwrite the digits the
  // user is halfway through typing.
  const [remDraft, setRemDraft] = useState<string | null>(null)
  const [daysDraft, setDaysDraft] = useState<string | null>(null)

  // The clock field is deliberately *uncontrolled*. A controlled time input
  // fights the browser: the hour and minute segments are edited separately,
  // and a controlled value would overwrite the segment being typed in (or
  // steal the focus) on every keystroke. Writing the DOM value directly —
  // and only while the field is not focused — keeps it in step with a
  // deadline changed elsewhere (the Settings page) without getting in the
  // way of an edit.
  useEffect(() => {
    const el = cutoffRef.current
    if (!el) return
    if (document.activeElement === el) return
    const next = plan?.stop_clock ?? ''
    if (el.value !== next) el.value = next
  }, [plan?.stop_clock])

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

  const patch = (p: Record<string, unknown>) =>
    run(() => invoke('set_requeue_plan', { patch: p }))

  // The backend reads the live counter itself: a game may have finished since
  // the overlay last polled, and "stop after the game in play" has to mean the
  // game that is running *now*, not the one the last poll saw.
  const stopAfterCurrent = () =>
    run(() => invoke('stop_after_current_game'))

  const remainingValue = remDraft ?? (plan.remaining != null ? String(plan.remaining) : '')
  const daysValue = daysDraft ?? (plan.stop_days != null ? String(Math.max(0, plan.stop_days)) : '')
  // "+" is only shown when there is an offset to speak of: on the day the
  // deadline falls, the plan is "today at that clock" — no plus, no days.
  const showPlus = Number(daysValue) > 0

  const commitRemaining = () => {
    const raw = remDraft
    setRemDraft(null)
    if (raw == null) return
    const n = Number(raw)
    if (!Number.isFinite(n) || n < 0) return
    // "还剩几局" is relative to what is already played; the budget is the
    // absolute number the counter is measured against.
    patch({ max_games: plan.games + Math.trunc(n) })
  }

  const commitDays = () => {
    const raw = daysDraft
    setDaysDraft(null)
    if (raw == null) return
    const n = Number(raw)
    if (!Number.isFinite(n) || n < 0) return
    const clock = cutoffRef.current?.value || plan.stop_clock || '23:30'
    patch({ stop_clock: clock, stop_days: Math.trunc(n) })
  }

  const commitClock = (value: string) => {
    if (!value) return
    const days = plan.stop_days != null ? Math.max(0, plan.stop_days) : 0
    patch({ stop_clock: value, stop_days: days })
  }

  const clearDeadline = () => patch({ clear_stop_at: true })

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
    <div className="shrink-0 border-t border-border/60 px-1.5 py-1 text-[11px] leading-none">
      <div className="flex items-center gap-1">
        <span
          className={
            'min-w-0 flex-1 truncate tabular-nums ' +
            (plan.deadline_passed ? 'text-amber-400' : 'text-muted-foreground')
          }
          title={title}
        >
          {statusText}
          {plan.deadline_passed ? ` · ${t('overlay.plan_expired')}` : ''}
        </span>
        <button
          type="button"
          onClick={stopAfterCurrent}
          disabled={busy}
          aria-label={t('overlay.plan_stop_after_current')}
          title={t('overlay.plan_stop_after_current')}
          className={`${CTRL} cursor-pointer px-1.5 disabled:opacity-50`}
          style={{ pointerEvents: 'auto' }}
        >
          {t('overlay.plan_stop_after_current')}
        </button>
      </div>
      <div className="mt-1 flex items-center gap-1">
        <span className="shrink-0 text-muted-foreground" title={t('overlay.plan_remaining_title')}>
          {t('overlay.plan_remaining')}
        </span>
        <input
          type="number"
          inputMode="numeric"
          min={0}
          value={remainingValue}
          onChange={(e) => setRemDraft(e.target.value)}
          onBlur={commitRemaining}
          onKeyDown={(e) => {
            if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
          }}
          aria-label={t('overlay.plan_remaining_title')}
          title={t('overlay.plan_remaining_title')}
          className={`${CTRL} w-[34px] text-center tabular-nums`}
          style={{ pointerEvents: 'auto' }}
        />
        <span
          className="shrink-0 text-muted-foreground"
          title={t('overlay.plan_days_title')}
        >
          {showPlus ? '+' : ''}
        </span>
        <input
          type="number"
          inputMode="numeric"
          min={0}
          value={daysValue}
          onChange={(e) => setDaysDraft(e.target.value)}
          onBlur={commitDays}
          onKeyDown={(e) => {
            if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
          }}
          aria-label={t('overlay.plan_days_title')}
          title={t('overlay.plan_days_title')}
          className={`${CTRL} w-[30px] text-center tabular-nums`}
          style={{ pointerEvents: 'auto' }}
        />
        <span className="shrink-0 text-muted-foreground" title={t('overlay.plan_days_title')}>
          {t('overlay.plan_days')}
        </span>
        <input
          ref={cutoffRef}
          type="time"
          defaultValue={plan.stop_clock ?? ''}
          onChange={(e) => commitClock(e.target.value)}
          onBlur={(e) => {
            if (!e.target.value) void patch({ clear_stop_at: true })
          }}
          aria-label={t('overlay.plan_clock')}
          title={t('overlay.plan_clock')}
          className={`${CTRL} w-[66px] tabular-nums`}
          style={{ pointerEvents: 'auto' }}
        />
        {plan.stop_at && (
          <button
            type="button"
            onClick={clearDeadline}
            disabled={busy}
            aria-label={t('overlay.plan_clear')}
            title={t('overlay.plan_clear')}
            className={`${CTRL} w-[18px] cursor-pointer px-0 text-center disabled:opacity-50`}
            style={{ pointerEvents: 'auto' }}
          >
            ×
          </button>
        )}
      </div>
    </div>
  )
}
