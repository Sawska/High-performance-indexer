import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { Market, Page } from '../api'
import { lastQuery, mockApi } from '../test/fetch'
import { market } from '../test/fixtures'
import { bodyRows, renderRoute, tableWithHeader } from '../test/render'
import { Markets } from './Markets'

const ICON = 'https://polymarket-upload.s3.us-east-2.amazonaws.com/rain.png'

const page: Page<Market> = {
  total: 3,
  rows: [
    market({ id: '1', question: 'Rain in London?', icon: ICON, one_day_change: 0.034 }),
    market({ id: '2', question: 'Snow in Paris?', one_day_change: -0.005, closed: true }),
    market({ id: '3', question: 'Fog in SF?', one_day_change: null }),
  ],
}

async function renderMarkets(url = '/markets', data: Page<Market> = page) {
  const fetchMock = mockApi({ '/api/markets': data })
  const view = renderRoute('/markets', url, <Markets />)
  await screen.findByText(`${data.total.toLocaleString()} markets`)
  return { ...view, fetchMock }
}

describe('Markets page', () => {
  it('requests live markets by volume by default', async () => {
    const { fetchMock } = await renderMarkets()
    expect(lastQuery(fetchMock, '/api/markets')).toEqual({ status: 'live', sort: 'volume', limit: '50', offset: '0' })
  })

  it('passes q, status, sort, limit and offset from the URL', async () => {
    const { fetchMock } = await renderMarkets('/markets?q=rain&status=all&sort=movers&offset=50')
    expect(lastQuery(fetchMock, '/api/markets')).toEqual({ q: 'rain', status: 'all', sort: 'movers', limit: '50', offset: '50' })
    expect(screen.getByRole('searchbox')).toHaveValue('rain')
    expect(screen.getByDisplayValue('All')).toBeInTheDocument()
    expect(screen.getByDisplayValue('Biggest 24h move')).toBeInTheDocument()
  })

  it('offers a "Biggest 24h move" sort that requests sort=movers', async () => {
    const { fetchMock } = await renderMarkets('/markets?offset=50')
    const option = screen.getByRole('option', { name: 'Biggest 24h move' })
    expect(option).toHaveAttribute('value', 'movers')

    const sortSelect = option.closest('select')!
    await userEvent.selectOptions(sortSelect, 'movers')
    await waitFor(() => expect(lastQuery(fetchMock, '/api/markets')).toMatchObject({ sort: 'movers', offset: '0' }))
    expect(screen.getByTestId('location')).toHaveTextContent('/markets?sort=movers')
  })

  it('renders the 24h move as signed, colored cents', async () => {
    await renderMarkets()
    const table = tableWithHeader('24h')
    expect(bodyRows(table).map((r) => r[3])).toEqual(['+3¢', '−0.5¢', '—'])
    const cells = [...table.querySelectorAll('tbody tr')].map((tr) => tr.querySelectorAll('td')[3].firstElementChild!)
    expect(cells[0]).toHaveClass('pos')
    expect(cells[1]).toHaveClass('neg')
    expect(cells[2].className).toBe('')
  })

  it('renders the rest of each row', async () => {
    await renderMarkets()
    const [rain, snow] = bodyRows(tableWithHeader('24h'))
    expect(rain).toEqual(['Rain in London?', 'Yes 62¢No 38¢', '61¢ / 63¢', '+3¢', '$2.3M', '$45.6K', 'Dec 31, 2026'])
    expect(snow[0]).toBe('Snow in Paris? closed')
    expect(screen.getByRole('link', { name: 'Rain in London?' })).toHaveAttribute('href', '/markets/1')
  })

  it('shows the market icon when set, a placeholder otherwise', async () => {
    await renderMarkets()
    const [rain, snow] = within(tableWithHeader('24h')).getAllByRole('row').slice(1)
    expect(rain.querySelector('img')).toHaveAttribute('src', ICON)
    expect(rain.querySelector('.icon-ph')).toBeNull()
    expect(snow.querySelector('img')).toBeNull()
    expect(snow.querySelector('.icon-ph')).not.toBeNull()
  })

  it('shows an empty state', async () => {
    await renderMarkets('/markets?q=zzz', { total: 0, rows: [] })
    expect(screen.getByText('No markets match.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Next →' })).toBeNull()
  })

  it('pages with the Pager', async () => {
    const { fetchMock } = await renderMarkets('/markets', { ...page, total: 120 })
    await userEvent.click(screen.getByRole('button', { name: 'Next →' }))
    await waitFor(() => expect(lastQuery(fetchMock, '/api/markets')).toMatchObject({ offset: '50' }))
    expect(screen.getByTestId('location')).toHaveTextContent('/markets?offset=50')
  })
})
