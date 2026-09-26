import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { Trade } from '../api'
import { lastQuery, mockApi, requests } from '../test/fetch'
import { CONDITION, trade, WALLET } from '../test/fixtures'
import { renderRoute } from '../test/render'
import { Trades } from './Trades'

const fullPage = Array.from({ length: 100 }, (_, i) => trade({ size: i + 1 }))

function renderTrades(url: string, data: Trade[] | ((u: URL) => Trade[]) = [trade()]) {
  const fetchMock = mockApi({ '/api/trades': data })
  const view = renderRoute('/trades', url, <Trades />)
  return { ...view, fetchMock }
}

describe('Trades page', () => {
  it('requests the newest page with no filters', async () => {
    const { fetchMock } = renderTrades('/trades')
    await screen.findByRole('table')
    expect(lastQuery(fetchMock, '/api/trades')).toEqual({ limit: '100', offset: '0' })
    expect(screen.queryByRole('button', { name: 'Clear filter' })).toBeNull()
    expect(screen.getByRole('columnheader', { name: 'Market' })).toBeInTheDocument()
  })

  it('filters by ?market= with a chip named after the market', async () => {
    const { fetchMock } = renderTrades(`/trades?market=${CONDITION}`)
    expect(await screen.findByText('Market: Will it rain in London tomorrow?')).toBeInTheDocument()
    expect(lastQuery(fetchMock, '/api/trades')).toEqual({ market: CONDITION, limit: '100', offset: '0' })
    expect(screen.queryByRole('columnheader', { name: 'Market' })).toBeNull()
  })

  it('names the market chip by a short id before rows arrive', () => {
    renderTrades(`/trades?market=${CONDITION}`)
    expect(screen.getByText(`Market: ${CONDITION.slice(0, 10)}…`)).toBeInTheDocument()
  })

  it('drops the market filter when its chip is cleared', async () => {
    const { fetchMock } = renderTrades(`/trades?market=${CONDITION}&side=BUY`)
    await screen.findByText('Market: Will it rain in London tomorrow?')
    await userEvent.click(screen.getByRole('button', { name: 'Clear filter' }))

    expect(screen.getByTestId('location')).toHaveTextContent('/trades?side=BUY')
    await waitFor(() => expect(lastQuery(fetchMock, '/api/trades')).toEqual({ side: 'BUY', limit: '100', offset: '0' }))
    expect(screen.queryByText(/^Market:/)).toBeNull()
    expect(await screen.findByRole('columnheader', { name: 'Market' })).toBeInTheDocument()
  })

  it('filters by ?wallet= with a chip named after the trader', async () => {
    const { fetchMock } = renderTrades(`/trades?wallet=${WALLET}`)
    expect(await screen.findByText('Trader: alice')).toBeInTheDocument()
    expect(lastQuery(fetchMock, '/api/trades')).toEqual({ wallet: WALLET, limit: '100', offset: '0' })
  })

  it('pages to older trades when a full page came back', async () => {
    const { fetchMock } = renderTrades('/trades', (u) => (u.searchParams.get('offset') === '100' ? [trade()] : fullPage))
    const older = await screen.findByRole('button', { name: 'Older →' })
    expect(older).toBeEnabled()
    expect(screen.getByRole('button', { name: '← Newer' })).toBeDisabled()
    expect(screen.getByText('1–100')).toBeInTheDocument()

    await userEvent.click(older)
    await waitFor(() => expect(lastQuery(fetchMock, '/api/trades')).toEqual({ limit: '100', offset: '100' }))
    expect(screen.getByTestId('location')).toHaveTextContent('/trades?offset=100')
    expect(await screen.findByText('101–101')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Older →' })).toBeDisabled()
    expect(screen.getByText('older trades')).toBeInTheDocument()
    expect(requests(fetchMock, '/api/trades').filter((u) => u.searchParams.get('offset') === '100')).toHaveLength(1)
  })

  it('has no pager when the first page is short', async () => {
    renderTrades('/trades', [trade(), trade()])
    await screen.findByRole('table')
    expect(screen.queryByRole('button', { name: 'Older →' })).toBeNull()
  })
})
