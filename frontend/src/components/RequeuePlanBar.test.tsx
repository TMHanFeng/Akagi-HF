import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'

import { RequeuePlanBar } from './RequeuePlanBar'
import type { RequeueStatus } from '@/types'

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) =>
      opts ? `${key}:${JSON.stringify(opts)}` : key,
  }),
}))

const invoke = vi.fn()
vi.mock('@/lib/tauri', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) =>
    args === undefined ? invoke(cmd) : invoke(cmd, args),
  HAS_TAURI: false,
}))

// The patch is spread over the base, and a spread of an optional property
// leaves that property optional — so the result is cast back to the strict
// shape the backend sends.
const status = (patch: Partial<RequeueStatus> = {}): RequeueStatus =>
  ({
    games: 12,
    max_games: 20,
    remaining: 8,
    stop_at: null,
    stop_clock: null,
    stop_days: null,
    deadline_passed: false,
    ...patch,
  }) as RequeueStatus

function mockStatus(s: RequeueStatus) {
  invoke.mockImplementation((cmd: string) => {
    if (cmd === 'get_requeue_status') return Promise.resolve(s)
    return Promise.resolve(undefined)
  })
}

/** The knobs live in a panel that opens on demand. */
async function openPanel() {
  fireEvent.click(screen.getByRole('button', { name: 'overlay.plan_settings' }))
  await waitFor(() =>
    expect(screen.getByLabelText('overlay.plan_remaining_title')).toBeTruthy(),
  )
}

beforeEach(() => {
  invoke.mockReset()
})

describe('RequeuePlanBar', () => {
  it('shows how many games are left', async () => {
    mockStatus(status())
    render(<RequeuePlanBar />)
    await waitFor(() =>
      expect(screen.getByText(/plan_status/).textContent).toContain('"games":12'),
    )
    expect(screen.getByText(/plan_status/).textContent).toContain('"remaining":8')
  })

  it('says "no limit" when the budget is unlimited', async () => {
    mockStatus(status({ max_games: 0, remaining: null }))
    render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_unlimited/)).toBeTruthy())
  })

  it('says the budget is spent when nothing is left', async () => {
    mockStatus(status({ games: 20, remaining: 0 }))
    render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_exhausted/)).toBeTruthy())
  })

  it('stops after the game in play on a click', async () => {
    mockStatus(status())
    render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_status/)).toBeTruthy())
    fireEvent.click(screen.getByRole('button', { name: 'overlay.plan_stop_after_current' }))
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('stop_after_current_game'),
    )
  })

  it('keeps the knobs hidden until they are asked for', async () => {
    mockStatus(status())
    render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_status/)).toBeTruthy())
    expect(screen.queryByLabelText('overlay.plan_remaining_title')).toBeNull()
    expect(screen.queryByLabelText('overlay.plan_clock')).toBeNull()

    await openPanel()
    expect(screen.getByLabelText('overlay.plan_remaining_title')).toBeTruthy()
    expect(screen.getByLabelText('overlay.plan_clock')).toBeTruthy()
  })

  it('turns the number of games left into a budget', async () => {
    mockStatus(status())
    render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_status/)).toBeTruthy())
    await openPanel()
    const input = screen.getByLabelText('overlay.plan_remaining_title')
    fireEvent.change(input, { target: { value: '5' } })
    fireEvent.blur(input)
    await waitFor(() =>
      // 12 games are already done, so "5 more" is a budget of 17.
      expect(invoke).toHaveBeenCalledWith('set_requeue_plan', {
        patch: { max_games: 17 },
      }),
    )
  })

  it('writes the day offset together with the clock', async () => {
    mockStatus(status())
    render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_status/)).toBeTruthy())
    await openPanel()
    const clock = screen.getByLabelText('overlay.plan_clock') as HTMLInputElement
    fireEvent.change(clock, { target: { value: '23:30' } })
    const days = screen.getByLabelText('overlay.plan_days_title')
    fireEvent.change(days, { target: { value: '2' } })
    fireEvent.blur(days)
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('set_requeue_plan', {
        patch: { stop_clock: '23:30', stop_days: 2 },
      }),
    )
  })

  it('shows the deadline as a reading in the one-line strip', async () => {
    mockStatus(
      status({
        stop_at: '2026-11-24T23:30',
        stop_clock: '23:30',
        stop_days: 2,
      }),
    )
    render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_status/)).toBeTruthy())
    const text = screen.getByText(/plan_status/).textContent ?? ''
    expect(text).toContain('23:30')
  })

  it('shows the day offset that is left, and no plus on the day itself', async () => {
    mockStatus(
      status({
        stop_at: '2026-11-24T23:30',
        stop_clock: '23:30',
        stop_days: 2,
      }),
    )
    const first = render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_status/)).toBeTruthy())
    fireEvent.click(
      first.getByRole('button', { name: 'overlay.plan_settings' }),
    )
    const days = first.getByLabelText('overlay.plan_days_title') as HTMLInputElement
    expect(days.value).toBe('2')
    expect(first.getByText('overlay.plan_days_label').parentElement?.textContent)
      .toContain('+')

    mockStatus(
      status({
        stop_at: '2026-11-22T23:30',
        stop_clock: '23:30',
        stop_days: 0,
      }),
    )
    const second = render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_status/)).toBeTruthy())
    fireEvent.click(
      second.getAllByRole('button', { name: 'overlay.plan_settings' })[1],
    )
    const days0 = second.getAllByLabelText('overlay.plan_days_title')[1] as HTMLInputElement
    expect(days0.value).toBe('0')
    expect(second.getAllByText('overlay.plan_days_label')[1].parentElement?.textContent)
      .not.toContain('+')
  })

  it('clears the deadline on a click', async () => {
    mockStatus(
      status({
        stop_at: '2026-11-24T23:30',
        stop_clock: '23:30',
        stop_days: 2,
      }),
    )
    render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_status/)).toBeTruthy())
    await openPanel()
    fireEvent.click(screen.getByRole('button', { name: 'overlay.plan_clear' }))
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('set_requeue_plan', {
        patch: { clear_stop_at: true },
      }),
    )
  })

  it('renders nothing when the requeue module is off', () => {
    mockStatus(status())
    const { container } = render(<RequeuePlanBar enabled={false} />)
    expect(container.textContent).toBe('')
  })
})
