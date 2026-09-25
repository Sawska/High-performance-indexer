import { screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { Overview as OverviewData } from '../api'
import { mockApi } from '../test/fetch'
import { market, trade, WALLET } from '../test/fixtures'
import { bodyRows, renderRoute, tableWithHeader, tiles } from '../test/render'
import { Overview } from './Overview'

const data: OverviewData = {
  totals: {
    markets: 5678,
    live_markets: 1234,
    live_volume: 12_500_000,
    live_liquidity: 345_000,
    wallets: 9876,
    trades_24h: 4321,
    volume_24h: 2_100_000,
  },
  top_markets: [market({ outcome_prices: [0.38, 0.62], one_day_change: -0.034 })],
  recent_trades: [trade()],
}

describe('Overview page', () => {
  it('renders the totals tiles from /api/overview', async () => {
    const fetchMock = mockApi({ '/api/overview': data })
    const { container } = renderRoute('/', '/', <Overview />)
    await screen.findByRole('heading', { name: 'Overview' })
    expect(fetchMock).toHaveBeenCalledWith('/api/overview', expect.anything())
    expect(tiles(container)).toEqual({
      'Live markets': { value: '1,234', sub: '5,678 stored' },
      'Live volume': { value: '$12.5M', sub: 'lifetime, open markets' },
      'Live liquidity': { value: '$345K', sub: null },
      'Trades, 24h': { value: '4,321', sub: '$2.1M traded' },
      'Traders seen': { value: '9,876', sub: null },
    })
  })

  it('lists top markets by their leading outcome and latest trades', async () => {
    mockApi({ '/api/overview': data })
    renderRoute('/', '/', <Overview />)
    await screen.findByRole('heading', { name: 'Overview' })
    // Market, Leading outcome, 24h, Volume, Liquidity
    const top = tableWithHeader('Leading outcome')
    expect(bodyRows(top)).toEqual([['Will it rain in London tomorrow?', 'No 62¢', '−3¢', '$2.3M', '$45.6K']])
    expect(within(top).getByRole('link', { name: 'Will it rain in London tomorrow?' })).toHaveAttribute('href', '/markets/512345')

    const latest = tableWithHeader('Trader')
    expect(within(latest).getByRole('link', { name: 'alice' })).toHaveAttribute('href', `/traders/${WALLET}`)
    expect(screen.getByRole('link', { name: 'all trades →' })).toHaveAttribute('href', '/trades')
  })

  it('shows the API error', async () => {
    mockApi({ '/api/overview': () => new Response('{"error":"clickhouse unavailable"}', { status: 503 }) })
    renderRoute('/', '/', <Overview />)
    expect(await screen.findByText('clickhouse unavailable')).toHaveClass('error-box')
  })
})
