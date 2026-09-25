import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { TraderDetail } from '../api'
import { mockApi } from '../test/fetch'
import { activity, position, traderDetail, WALLET } from '../test/fixtures'
import { bodyRows, renderRoute, tableWithHeader, tiles } from '../test/render'
import { Trader } from './Trader'

const detail: TraderDetail = traderDetail({
  pnl: [
    { ts: '2026-09-20T12:00:00Z', pnl: 10_000 },
    { ts: '2026-09-25T12:00:00Z', pnl: 12_500 },
  ],
  values: [
    { ts: '2026-09-20T12:00:00Z', value: 3_000 },
    { ts: '2026-09-25T12:00:00Z', value: 3_400 },
  ],
  positions: [
    // Won and not redeemed yet.
    position({ asset: 'a1', initial_value: 80, current_value: 120, cash_pnl: 40, percent_pnl: 50, realized_pnl: 25, redeemable: true }),
    // Lost: redeemable for nothing, market not scraped.
    position({
      asset: 'a2',
      condition_id: '0x5n0w',
      has_market: false,
      question: 'Will it snow in Paris?',
      outcome: 'No',
      initial_value: 60,
      current_value: 0,
      cur_price: 0,
      cash_pnl: -60,
      percent_pnl: -100,
      realized_pnl: -10,
      redeemable: true,
    }),
    // Still open, holding both sides.
    position({ asset: 'a3', question: 'Will the Thames freeze?', mergeable: true }),
  ],
  activity: [
    activity({ activity_type: 'TRADE', side: 'BUY', title: 'Rain market', ts: '2026-09-25T11:00:00Z' }),
    activity({ activity_type: 'TRADE', side: 'SELL', title: 'Rain market', ts: '2026-09-25T10:00:00Z' }),
    activity({ activity_type: 'REDEEM', side: null, outcome: null, price: 0, usdc_size: 120, title: 'Snow market', ts: '2026-09-24T12:00:00Z' }),
  ],
})

async function renderTrader(data: TraderDetail = detail) {
  const fetchMock = mockApi({ [`/api/traders/${WALLET}`]: data })
  const view = renderRoute('/traders/:wallet', `/traders/${WALLET}`, <Trader />)
  await screen.findByRole('heading', { level: 1 })
  return { ...view, fetchMock }
}

const activityRows = () => bodyRows(tableWithHeader('USDC'))

describe('Trader page', () => {
  it('shows name, verified mark, wallet and bio', async () => {
    const { fetchMock } = await renderTrader()
    expect(fetchMock).toHaveBeenCalledWith(`/api/traders/${WALLET}`, expect.anything())
    const h1 = screen.getByRole('heading', { level: 1 })
    expect(h1).toHaveTextContent('alice')
    expect(within(h1).getByTitle('Verified on Polymarket')).toHaveTextContent('✓')
    expect(screen.getByText(WALLET, { exact: false, selector: '.mono' })).toHaveTextContent(`${WALLET} · polymarket.com ↗`)
    expect(screen.getByRole('link', { name: 'polymarket.com ↗' })).toHaveAttribute('href', `https://polymarket.com/profile/${WALLET}`)
    expect(screen.getByText('Weather markets only.')).toHaveClass('bio')
  })

  it('omits the verified mark and bio when absent', async () => {
    await renderTrader(traderDetail({ profile: { bio: null, verified: null } }))
    expect(screen.queryByTitle('Verified on Polymarket')).toBeNull()
    expect(document.querySelector('.bio')).toBeNull()
  })

  it('falls back to the pseudonym in the heading', async () => {
    const base = traderDetail()
    await renderTrader({ ...base, trader: { ...base.trader, name: null } })
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Brave-Otter')
  })

  it('renders the summary tiles and trade link', async () => {
    const { container } = await renderTrader()
    const t = tiles(container)
    expect(t['PnL'].value).toBe('+$12.5K')
    expect(screen.getByText('+$12.5K')).toHaveClass('pos')
    expect(t['Portfolio value'].value).toBe('$3,400')
    expect(t['Volume (stored trades)'].value).toBe('$98K')
    expect(t['Trades']).toEqual({ value: '42', sub: 'view trades →' })
    expect(screen.getByRole('link', { name: 'view trades →' })).toHaveAttribute('href', `/trades?wallet=${WALLET}`)
  })

  it('counts resolved positions and what is left to redeem', async () => {
    const { container } = await renderTrader()
    expect(tiles(container)['Open positions']).toEqual({ value: '3', sub: '2 resolved · $120 to redeem' })
  })

  it('leaves out the redeem amount when resolved positions are all worthless', async () => {
    const { container } = await renderTrader({ ...detail, positions: [detail.positions[1], detail.positions[2]] })
    expect(tiles(container)['Open positions']).toEqual({ value: '2', sub: '1 resolved' })
  })

  it('has no sub text without resolved positions', async () => {
    const { container } = await renderTrader({ ...detail, positions: [detail.positions[2]] })
    expect(tiles(container)['Open positions']).toEqual({ value: '1', sub: null })
  })

  it('switches the chart between PnL and portfolio value', async () => {
    await renderTrader()
    expect(screen.getByRole('heading', { name: 'PnL over time' })).toBeInTheDocument()
    expect(screen.getByRole('img', { name: 'PnL' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Portfolio value' }))
    expect(screen.getByRole('heading', { name: 'Portfolio value over time' })).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'PnL over time' })).toBeNull()
    expect(screen.getByRole('img', { name: 'Value' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Portfolio value' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('lists positions with cost, realized PnL and resolution pills', async () => {
    await renderTrader()
    expect(screen.getByRole('columnheader', { name: 'Cost' })).toBeInTheDocument()
    const table = tableWithHeader('Realized')
    const rows = bodyRows(table)
    // Market, Outcome, Shares, Avg, Now, Cost, Value, Unrealized, Realized, Ends
    expect(rows[0]).toEqual([
      'Will it rain in London tomorrow?redeemable',
      'Yes',
      '200',
      '50¢',
      '62¢',
      '$80',
      '$120',
      '+$40 (50.0%)',
      '+$25',
      'Dec 31, 2026',
    ])
    expect(rows[1].slice(5, 9)).toEqual(['$60', '$0', '−$60 (-100.0%)', '−$10'])

    const [won, lost, open] = within(table).getAllByRole('row').slice(1)
    expect(within(won).getByText('redeemable')).toHaveClass('pill', 'ok')
    expect(within(won).getByRole('link', { name: 'Will it rain in London tomorrow?' })).toHaveAttribute(
      'href',
      `/markets/${detail.positions[0].condition_id}`,
    )
    expect(within(won).getByText('+$25')).toHaveClass('pos')

    const resolved = within(lost).getByText('resolved')
    expect(resolved).toHaveClass('pill')
    expect(resolved).not.toHaveClass('ok')
    expect(within(lost).queryByRole('link')).toBeNull()
    expect(within(lost).getByTitle('Market not scraped yet')).toHaveTextContent('Will it snow in Paris?')
    expect(within(lost).getByText('−$10')).toHaveClass('neg')

    expect(within(open).queryByText(/^(redeemable|resolved)$/)).toBeNull()
    expect(within(open).getByText('mergeable')).toHaveClass('pill')
  })

  it('shows an empty state without positions', async () => {
    await renderTrader(traderDetail())
    expect(screen.getByText('No positions stored yet.')).toBeInTheDocument()
    expect(screen.getByText('No activity stored yet.')).toBeInTheDocument()
  })

  it('filters activity by type', async () => {
    await renderTrader()
    expect(activityRows()).toHaveLength(3)
    const all = screen.getByRole('button', { name: 'All' })
    expect(all).toHaveAttribute('aria-pressed', 'true')

    await userEvent.click(screen.getByRole('button', { name: 'redeem' }))
    const redeems = activityRows()
    expect(redeems).toHaveLength(1)
    // When, Type, Market, Outcome, Price, Shares, USDC
    expect(redeems[0].slice(1)).toEqual(['REDEEM', 'Snow market', '—', '—', '100', '$120'])

    await userEvent.click(screen.getByRole('button', { name: 'trade' }))
    expect(activityRows().map((r) => r[1])).toEqual(['BUY', 'SELL'])

    await userEvent.click(all)
    expect(activityRows()).toHaveLength(3)
  })

  it('hides the type filter when there is only one type', async () => {
    await renderTrader({ ...detail, activity: [detail.activity[0]] })
    expect(activityRows()).toHaveLength(1)
    expect(screen.queryByRole('button', { name: 'All' })).toBeNull()
  })

  it('links to the full activity feed for this wallet', async () => {
    await renderTrader()
    expect(screen.getByRole('link', { name: 'all activity →' })).toHaveAttribute('href', `/activity?wallet=${WALLET}`)
  })
})
