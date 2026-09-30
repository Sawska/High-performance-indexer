import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { lastQuery, mockApi } from '../test/fetch'
import {
  CONDITION,
  MAKER,
  TAKER,
  TOKEN_YES,
  chainCollateral,
  chainCondition,
  chainFill,
  chainPosition,
  chainRedemption,
  chainStatus,
  chainTransfer,
} from '../test/fixtures'
import { bodyRows, renderRoute, tiles } from '../test/render'
import { Chain } from './Chain'

type Routes = Record<string, unknown>

function renderChain(url = '/chain', routes: Routes = {}) {
  const fetchMock = mockApi({
    '/api/chain/status': chainStatus(),
    '/api/chain/fills': [chainFill()],
    '/api/chain/positions': [chainPosition()],
    '/api/chain/redemptions': [chainRedemption()],
    '/api/chain/transfers': [chainTransfer()],
    '/api/chain/collateral': [chainCollateral()],
    '/api/chain/conditions': [chainCondition()],
    ...routes,
  })
  const view = renderRoute('/chain', url, <Chain />)
  return { ...view, fetchMock }
}

describe('On-chain page', () => {
  it('opens on fills and reports how far behind the head it is', async () => {
    const { fetchMock, container } = renderChain()
    await screen.findByRole('table')

    expect(lastQuery(fetchMock, '/api/chain/fills')).toEqual({ limit: '100', offset: '0' })
    const t = tiles(container)
    expect(t['Chain head'].value).toBe('94,641,500')
    expect(t['Indexed to']).toEqual({ value: '94,641,470', sub: '30 blocks behind' })
    // 37,151 + 78,418 across the two tables the status reports.
    expect(t['Rows stored'].value).toBe('115,569')
    expect(screen.getByText('following head')).toBeInTheDocument()
  })

  it('turns a large block lag into a readable duration', async () => {
    renderChain('/chain', { '/api/chain/status': chainStatus({ lag: 30_000 }) })
    // 30,000 blocks at ~2.1s each is a bit over 17 hours.
    expect(await screen.findByText('17h 30m behind')).toBeInTheDocument()
  })

  it('says so when this server is not the one indexing', async () => {
    renderChain('/chain', { '/api/chain/status': chainStatus({ enabled: false, head: null, cursor: null, lag: null }) })
    expect(await screen.findByText(/not running the on-chain indexer/)).toBeInTheDocument()
    expect(screen.getByText('not indexing here')).toBeInTheDocument()
    // The stored cursor still shows, because the data is there regardless.
    expect(tiles(document.body)['Indexed to'].value).toBe('94,641,470')
  })

  it('shows a fill with its price, value and exchange version', async () => {
    renderChain()
    const table = await screen.findByRole('table')
    const [row] = bodyRows(table)

    expect(row[1]).toBe('Will it rain in London tomorrow?')
    expect(row[2]).toBe('BUY')
    expect(row[5]).toBe('62¢')
    expect(row[6]).toBe('100')
    expect(row[7]).toBe('$62')
    expect(row[9]).toContain('v2')
    expect(screen.getByRole('link', { name: 'Will it rain in London tomorrow?' })).toHaveAttribute(
      'href',
      `/markets/${CONDITION}`,
    )
  })

  it('marks a neg-risk orders_matched fill', async () => {
    renderChain('/chain', {
      '/api/chain/fills': [chainFill({ neg_risk: true, event: 'orders_matched', taker: null, fee: null })],
    })
    const table = await screen.findByRole('table')
    const [row] = bodyRows(table)
    expect(row[9]).toContain('neg-risk')
    expect(row[9]).toContain('matched')
    expect(row[4]).toBe('—') // orders_matched carries no taker
    expect(row[8]).toBe('')
  })

  it('falls back to the token id when the catalogue has no market', async () => {
    renderChain('/chain', { '/api/chain/fills': [chainFill({ condition_id: null, question: null })] })
    const table = await screen.findByRole('table')
    const link = within(table).getByTitle(TOKEN_YES)
    expect(link).toHaveTextContent('71321045…2563')
    // The token replaces the market name; it must not trail an empty placeholder.
    expect(bodyRows(table)[0][1]).toBe('71321045…2563')
  })

  it('shows a dash when a fill has neither a market nor a token', async () => {
    renderChain('/chain', { '/api/chain/fills': [chainFill({ condition_id: null, question: null, token_id: null })] })
    const table = await screen.findByRole('table')
    expect(bodyRows(table)[0][1]).toBe('—')
  })

  it('switches feed, and only sends the filters that feed accepts', async () => {
    const { fetchMock } = renderChain(`/chain?wallet=${MAKER}&token_id=${TOKEN_YES}`)
    await screen.findByRole('table')
    expect(lastQuery(fetchMock, '/api/chain/fills')).toEqual({
      wallet: MAKER,
      token_id: TOKEN_YES,
      limit: '100',
      offset: '0',
    })

    await userEvent.click(screen.getByRole('button', { name: 'Splits & merges' }))
    // positions takes wallet and condition_id, so the token filter is dropped.
    await waitFor(() =>
      expect(lastQuery(fetchMock, '/api/chain/positions')).toEqual({ wallet: MAKER, limit: '100', offset: '0' }),
    )
    expect(screen.queryByText(/^Token:/)).toBeNull()
    expect(screen.getByText(`Address: ${MAKER.slice(0, 6)}…${MAKER.slice(-4)}`)).toBeInTheDocument()
  })

  it('narrows to an address when one in the table is clicked', async () => {
    const { fetchMock } = renderChain()
    const table = await screen.findByRole('table')
    await userEvent.click(within(table).getByTitle(TAKER))

    expect(screen.getByTestId('location')).toHaveTextContent(`wallet=${TAKER}`)
    await waitFor(() => expect(lastQuery(fetchMock, '/api/chain/fills').wallet).toBe(TAKER))
  })

  it('clears an address filter from its chip', async () => {
    const { fetchMock } = renderChain(`/chain?wallet=${MAKER}`)
    await screen.findByRole('table')
    await userEvent.click(screen.getByRole('button', { name: 'Clear filter' }))
    await waitFor(() => expect(lastQuery(fetchMock, '/api/chain/fills')).toEqual({ limit: '100', offset: '0' }))
  })

  it('shows a split as locking collateral up', async () => {
    renderChain('/chain?feed=positions')
    const table = await screen.findByRole('table')
    const [row] = bodyRows(table)
    expect(row[1]).toBe('SPLIT')
    expect(row[4]).toBe('2') // the partition has two outcomes
    expect(row[5]).toBe('$100')
  })

  it('shows a losing redemption as a zero payout rather than a blank', async () => {
    renderChain('/chain?feed=redemptions', { '/api/chain/redemptions': [chainRedemption({ payout: 0 })] })
    const table = await screen.findByRole('table')
    const [row] = bodyRows(table)
    expect(row[3]).toBe('1, 2')
    expect(row[4]).toBe('$0')
  })

  it('names the winning outcome of a resolved condition', async () => {
    renderChain('/chain?feed=conditions')
    const table = await screen.findByRole('table')
    const [row] = bodyRows(table)
    expect(row[4]).toContain('1 / 0')
    expect(row[4]).toContain('outcome 0')
  })

  it('shows an open condition as open, with no winner', async () => {
    renderChain('/chain?feed=conditions', {
      '/api/chain/conditions': [chainCondition({ resolved_at: null, payout_numerators: null })],
    })
    const table = await screen.findByRole('table')
    const [row] = bodyRows(table)
    expect(row[3]).toBe('open')
    expect(row[4]).toBe('—')
  })

  it('claims no winner when a resolution splits the payout', async () => {
    renderChain('/chain?feed=conditions', {
      '/api/chain/conditions': [chainCondition({ payout_numerators: ['1', '1'] })],
    })
    const table = await screen.findByRole('table')
    expect(bodyRows(table)[0][4]).not.toContain('outcome')
  })

  it('filters conditions by resolution state', async () => {
    const { fetchMock } = renderChain('/chain?feed=conditions')
    await screen.findByRole('table')
    await userEvent.selectOptions(screen.getByRole('combobox'), 'false')
    await waitFor(() =>
      expect(lastQuery(fetchMock, '/api/chain/conditions')).toEqual({ resolved: 'false', limit: '100', offset: '0' }),
    )
  })

  it('shows an ERC-1155 transfer with both sides', async () => {
    renderChain('/chain?feed=transfers')
    const table = await screen.findByRole('table')
    const [row] = bodyRows(table)
    expect(row[1]).toBe('0x1111…1111')
    expect(row[2]).toBe('0x2222…2222')
    expect(row[4]).toBe('100')
  })

  it('labels a collateral transfer with its token symbol', async () => {
    renderChain('/chain?feed=collateral')
    const table = await screen.findByRole('table')
    const [row] = bodyRows(table)
    expect(row[3]).toBe('USDC.e')
    expect(row[4]).toBe('$12.16')
  })

  it('falls back to the block number when the RPC gave no timestamp', async () => {
    renderChain('/chain', { '/api/chain/fills': [chainFill({ block_time: null })] })
    const table = await screen.findByRole('table')
    expect(within(table).getByText('#94,641,470')).toBeInTheDocument()
  })

  it('pages to older events and back', async () => {
    const full = Array.from({ length: 100 }, (_, i) => chainFill({ log_index: i }))
    const { fetchMock } = renderChain('/chain', {
      '/api/chain/fills': (u: URL) => (u.searchParams.get('offset') === '100' ? [chainFill()] : full),
    })
    const older = await screen.findByRole('button', { name: 'Older →' })
    await userEvent.click(older)

    await waitFor(() => expect(lastQuery(fetchMock, '/api/chain/fills').offset).toBe('100'))
    expect(screen.getByText('older events')).toBeInTheDocument()
  })

  it('says when a filter matched nothing', async () => {
    renderChain('/chain', { '/api/chain/fills': [] })
    expect(await screen.findByText('Nothing indexed for this filter yet.')).toBeInTheDocument()
  })

  it('surfaces an api error', async () => {
    renderChain('/chain', { '/api/chain/fills': new Response('{"error":"boom"}', { status: 500 }) })
    expect(await screen.findByText('boom')).toBeInTheDocument()
  })

  it('ignores an unknown feed in the url rather than rendering nothing', async () => {
    const { fetchMock } = renderChain('/chain?feed=nonsense')
    await screen.findByRole('table')
    expect(lastQuery(fetchMock, '/api/chain/fills')).toEqual({ limit: '100', offset: '0' })
  })
})
