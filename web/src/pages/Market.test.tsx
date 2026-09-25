import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { MarketDetail } from '../api'
import { mockApi } from '../test/fetch'
import { CONDITION, market, marketDetail, TOKEN_NO, TOKEN_YES, trade, TX, WALLET } from '../test/fixtures'
import { bodyRows, renderRoute, tableWithHeader, tiles } from '../test/render'
import { Market } from './Market'

const AT = '2026-09-25T11:58:00Z'

const detail: MarketDetail = marketDetail({
  market: market({ one_day_change: 0.034 }),
  info: { ...marketDetail().info, one_week_change: -0.05 },
  history: [
    { token: TOKEN_YES, ts: '2026-09-24T12:00:00Z', price: 0.58 },
    { token: TOKEN_YES, ts: '2026-09-25T12:00:00Z', price: 0.62 },
    { token: TOKEN_NO, ts: '2026-09-24T12:00:00Z', price: 0.42 },
    { token: TOKEN_NO, ts: '2026-09-25T12:00:00Z', price: 0.38 },
  ],
  spread_history: [
    { token: TOKEN_YES, ts: '2026-09-24T12:00:00Z', price: 0.03 },
    { token: TOKEN_YES, ts: '2026-09-25T12:00:00Z', price: 0.02 },
  ],
  quotes: [
    { asset_id: TOKEN_YES, kind: 'midpoint', side: null, value: 0.6, captured_at: AT },
    { asset_id: TOKEN_YES, kind: 'price', side: 'BUY', value: 0.61, captured_at: AT },
    { asset_id: TOKEN_YES, kind: 'price', side: 'SELL', value: 0.59, captured_at: AT },
    { asset_id: TOKEN_YES, kind: 'spread', side: null, value: 0.02, captured_at: AT },
    { asset_id: TOKEN_NO, kind: 'midpoint', side: null, value: 0.4, captured_at: AT },
  ],
  books: [
    { asset_id: TOKEN_YES, ts: AT, tick_size: 0.01, min_order_size: 5, neg_risk: true, last_trade_price: 0.63 },
  ],
  book: [
    { asset_id: TOKEN_YES, side: 'bid', price: 0.6, size: 250 },
    { asset_id: TOKEN_YES, side: 'bid', price: 0.59, size: 400 },
    { asset_id: TOKEN_YES, side: 'ask', price: 0.62, size: 300 },
  ],
  holders: [{ token: TOKEN_YES, proxy_wallet: WALLET, name: null, pseudonym: 'Brave-Otter', amount: 1500 }],
  trades: [trade()],
})

const liveFeed: Pick<MarketDetail, 'ws_trades' | 'price_changes' | 'tick_changes'> = {
  ws_trades: [
    { asset_id: TOKEN_YES, ts: AT, price: 0.62, size: 50, side: 'BUY', fee_rate_bps: 30, transaction_hash: TX },
    { asset_id: TOKEN_NO, ts: AT, price: 0.38, size: null, side: 'SELL', fee_rate_bps: null, transaction_hash: null },
  ],
  price_changes: [{ asset_id: TOKEN_YES, ts: AT, price: 0.61, size: 1200, side: 'BUY', best_ask: 0.63 }],
  tick_changes: [{ asset_id: TOKEN_NO, ts: AT, old_tick_size: 0.01, new_tick_size: 0.001 }],
}

async function renderMarket(data: MarketDetail = detail) {
  const fetchMock = mockApi({ [`/api/markets/${data.market.id}`]: data })
  const view = renderRoute('/markets/:id', `/markets/${data.market.id}`, <Market />)
  await screen.findByRole('heading', { level: 1, name: data.market.question ?? '' })
  return { ...view, fetchMock }
}

/** The <dd> text for a <dt> in the Contract list. */
const contract = (term: string) => screen.getByText(term, { selector: 'dt' }).nextElementSibling?.textContent

describe('Market page', () => {
  it('fetches the market by route id and renders the question', async () => {
    const { fetchMock } = await renderMarket()
    expect(fetchMock).toHaveBeenCalledWith('/api/markets/512345', expect.objectContaining({ method: 'GET' }))
    expect(screen.getByText('Live')).toHaveClass('pill', 'ok')
    expect(screen.getByText('Weather')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'polymarket.com ↗' })).toHaveAttribute(
      'href',
      'https://polymarket.com/market/will-it-rain-in-london-tomorrow',
    )
  })

  it('shows a loading state, then an API error', async () => {
    mockApi({ '/api/markets/404': () => new Response('{"error":"market not found"}', { status: 404 }) })
    renderRoute('/markets/:id', '/markets/404', <Market />)
    expect(screen.getByText('Loading…')).toBeInTheDocument()
    expect(await screen.findByText('market not found')).toHaveClass('error-box')
  })

  it('renders a tile per outcome plus volume, liquidity and spread', async () => {
    const { container } = await renderMarket()
    expect(tiles(container)).toEqual({
      Yes: { value: '62¢', sub: '24h +3¢ · 7d −5¢' },
      No: { value: '38¢', sub: null },
      Volume: { value: '$2.3M', sub: null },
      Liquidity: { value: '$45.6K', sub: null },
      Spread: { value: '2¢', sub: 'bid 61¢ · ask 63¢' },
    })
  })

  it('renders the CLOB quotes table from quotes and books', async () => {
    await renderMarket()
    expect(screen.getByRole('heading', { name: /^CLOB quotes/ })).toHaveTextContent(/as of .+ ago$/)
    const rows = bodyRows(tableWithHeader('Midpoint'))
    expect(rows).toHaveLength(2)
    // Outcome, Midpoint, Buy, Sell, Spread, Last trade, Tick, Min order, (Book as of)
    expect(rows[0].slice(0, 8)).toEqual(['Yes', '60¢', '61¢', '59¢', '2¢', '63¢', '0.01', '5'])
    expect(rows[0][8]).toMatch(/ago$/)
    expect(rows[1]).toEqual(['No', '40¢', '—', '—', '—', '—', '—', '—', '—'])
  })

  it('hides the CLOB quotes table with no quotes or books', async () => {
    await renderMarket(marketDetail())
    expect(screen.queryByRole('heading', { name: /^CLOB quotes/ })).toBeNull()
    expect(screen.queryByRole('columnheader', { name: 'Midpoint' })).toBeNull()
  })

  it('shows the neg-risk pill only when a book is neg-risk', async () => {
    await renderMarket()
    expect(screen.getByText('neg-risk')).toHaveClass('pill')
  })

  it('omits the neg-risk pill otherwise', async () => {
    await renderMarket({ ...detail, books: detail.books.map((b) => ({ ...b, neg_risk: false })) })
    expect(screen.queryByText('neg-risk')).toBeNull()
  })

  it('switches the chart between price and spread', async () => {
    await renderMarket()
    expect(screen.getByRole('heading', { name: 'Price, last 30 days' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Price' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByRole('img', { name: 'Yes, No' })).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Spread' }))
    expect(screen.getByRole('heading', { name: 'Spread, last 30 days · hourly mean' })).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'Price, last 30 days' })).toBeNull()
    expect(screen.getByRole('button', { name: 'Spread' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByRole('img', { name: 'Yes' })).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Price' }))
    expect(screen.getByRole('heading', { name: 'Price, last 30 days' })).toBeInTheDocument()
  })

  it('renders order book levels and depth per outcome', async () => {
    await renderMarket()
    const yes = screen.getByRole('heading', { name: 'Order book · Yes' }).closest('section')!
    const [bookTable, holdersTable] = within(yes).getAllByRole('table')
    expect(bodyRows(bookTable)).toEqual([
      ['250', '60¢', '62¢', '300'],
      ['400', '59¢', '', ''],
    ])
    expect(within(yes).getByText('Top 2 levels: $386 bid · $186 ask')).toBeInTheDocument()
    expect(within(holdersTable).getByRole('link', { name: 'Brave-Otter' })).toHaveAttribute('href', `/traders/${WALLET}`)
    expect(within(holdersTable).getByText('1,500 shares')).toBeInTheDocument()

    const no = screen.getByRole('heading', { name: 'Order book · No' }).closest('section')!
    expect(within(no).getByText('No book snapshot yet.')).toBeInTheDocument()
    expect(within(no).getByText('No holders stored yet.')).toBeInTheDocument()
  })

  it('links recent trades to the full trades feed for this market', async () => {
    await renderMarket()
    expect(screen.getByRole('link', { name: 'all trades →' })).toHaveAttribute('href', `/trades?market=${CONDITION}`)
    // The market column is redundant on the market's own page.
    expect(screen.getByRole('link', { name: 'alice' })).toHaveAttribute('href', `/traders/${WALLET}`)
    expect(screen.queryByRole('link', { name: 'Will it rain in London tomorrow?' })).toBeNull()
  })

  it('lists ids and token ids in the Contract section', async () => {
    await renderMarket()
    expect(screen.getByRole('heading', { name: 'Contract' })).toBeInTheDocument()
    expect(contract('Market id')).toBe('512345')
    expect(contract('Condition id')).toBe(CONDITION)
    expect(contract('Token · Yes')).toBe(TOKEN_YES)
    expect(contract('Token · No')).toBe(TOKEN_NO)
    expect(contract('Starts')).toBe('Sep 1, 2026')
    expect(contract('Ends')).toBe('Dec 31, 2026')
    expect(contract('Tick size')).toBe('0.01')
    expect(contract('Min order')).toBe('5 shares')
  })

  it('omits the live feed when the websocket stored nothing', async () => {
    await renderMarket()
    expect(screen.queryByRole('heading', { name: 'Live feed · websocket' })).toBeNull()
    expect(screen.queryByRole('button', { name: /^Fills/ })).toBeNull()
  })

  it('shows the live feed and switches between fills, book changes and tick sizes', async () => {
    await renderMarket({ ...detail, ...liveFeed })
    expect(screen.getByRole('heading', { name: 'Live feed · websocket' })).toBeInTheDocument()
    const fills = screen.getByRole('button', { name: 'Fills 2' })
    const changes = screen.getByRole('button', { name: 'Book changes 1' })
    const ticks = screen.getByRole('button', { name: 'Tick size 1' })
    expect(fills).toHaveAttribute('aria-pressed', 'true')

    // When, Outcome, Side, Price, Shares, Fee
    const fillRows = bodyRows(tableWithHeader('Fee')).map((r) => r.slice(1))
    expect(fillRows).toEqual([
      ['Yes', 'BUY', '62¢', '50', '30 bps'],
      ['No', 'SELL', '38¢', '—', '—'],
    ])

    await userEvent.click(changes)
    expect(changes).toHaveAttribute('aria-pressed', 'true')
    expect(screen.queryByRole('columnheader', { name: 'Fee' })).toBeNull()
    expect(bodyRows(tableWithHeader('Best ask')).map((r) => r.slice(1))).toEqual([['Yes', 'BUY', '61¢', '1,200', '63¢']])

    await userEvent.click(ticks)
    expect(ticks).toHaveAttribute('aria-pressed', 'true')
    expect(screen.queryByRole('columnheader', { name: 'Best ask' })).toBeNull()
    expect(bodyRows(tableWithHeader('Old tick')).map((r) => r.slice(1))).toEqual([['No', '0.01', '0.001']])

    await userEvent.click(fills)
    expect(screen.getByRole('columnheader', { name: 'Fee' })).toBeInTheDocument()
  })

  it('opens the live feed on the first kind that has rows', async () => {
    await renderMarket({ ...detail, tick_changes: liveFeed.tick_changes })
    expect(screen.getByRole('heading', { name: 'Live feed · websocket' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Fills 0' })).toHaveAttribute('aria-pressed', 'false')
    expect(screen.getByRole('button', { name: 'Tick size 1' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByRole('columnheader', { name: 'New tick' })).toBeInTheDocument()
  })
})
