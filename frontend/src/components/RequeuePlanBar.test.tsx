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

const status = (patch: Partial<RequeueStatus> = {}): RequeueStatus => ({
  games: 12,
  max_games: 20,
  remaining: 8,
  stop_at: null,
  deadline_passed: false,
  ...patch,
})

function mockStatus(s: RequeueStatus) {
  invoke.mockImplementation((cmd: string) => {
    if (cmd === 'get_requeue_status') return Promise.resolve(s)
    return Promise.resolve(undefined)
  })
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

  it('writes a cutoff the user picks', async () => {
    mockStatus(status())
    render(<RequeuePlanBar />)
    await waitFor(() => expect(screen.getByText(/plan_status/)).toBeTruthy())
    const input = screen.getByLabelText('overlay.plan_stop_at') as HTMLInputElement
    fireEvent.change(input, { target: { value: '23:30' } })
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('set_requeue_plan', {
        patch: { stop_at: '23:30' },
      }),
    )
  })

  it('renders nothing when the requeue module is off', () => {
    mockStatus(status())
    const { container } = render(<RequeuePlanBar enabled={false} />)
    expect(container.textContent).toBe('')
  })
})
