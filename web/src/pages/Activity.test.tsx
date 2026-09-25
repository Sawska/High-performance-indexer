import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { ActivityFeedRow } from '../api'
import { lastQuery, mockApi } from '../test/fetch'
import { activityRow, CONDITION, WALLET } from '../test/fixtures'
import { bodyRows, renderRoute, tableWithHeader } from '../test/render'
import { Activity } from './Activity'

function renderActivity(url: string, data: ActivityFeedRow[] = []) {
  const fetchMock = mockApi({ '/api/activity': data })
  const view = renderRoute('/activity', url, <Activity />)
  return { ...view, fetchMock }
}

const typeSelect = () => screen.getByRole('option', { name: 'All types' }).closest('select')!

describe('Activity page', () => {
  it('passes type and wallet from the URL and shows the wallet chip', async () => {
    const { fetchMock } = renderActivity(`/activity?type=REDEEM&wallet=${WALLET}`)
    expect(await screen.findByText('No activity matches.')).toBeInTheDocument()
    expect(lastQuery(fetchMock, '/api/activity')).toEqual({ type: 'REDEEM', wallet: WALLET, limit: '100', offset: '0' })
    // No rows to take a name from, so the chip shows the short wallet.
    expect(screen.getByText('Trader: 0xabcd…1234')).toBeInTheDocument()
    expect(typeSelect()).toHaveValue('REDEEM')
    expect(screen.getByDisplayValue('redeem')).toBeInTheDocument()
  })

  it('lists every activity type in the type select', async () => {
    renderActivity('/activity')
    await screen.findByText('No activity matches.')
    const options = within(typeSelect()).getAllByRole('option') as HTMLOptionElement[]
    expect(options.map((o) => [o.value, o.textContent])).toEqual([
      ['', 'All types'],
      ['TRADE', 'trade'],
      ['SPLIT', 'split'],
      ['MERGE', 'merge'],
      ['REDEEM', 'redeem'],
      ['CONVERSION', 'conversion'],
      ['REWARD', 'reward'],
      ['YIELD', 'yield'],
      ['MAKER_REBATE', 'maker rebate'],
      ['TAKER_REBATE', 'taker rebate'],
    ])
  })

  it('requests the selected type', async () => {
    const { fetchMock } = renderActivity('/activity')
    await screen.findByText('No activity matches.')
    expect(lastQuery(fetchMock, '/api/activity')).toEqual({ limit: '100', offset: '0' })
    await userEvent.selectOptions(typeSelect(), 'MERGE')
    await waitFor(() => expect(lastQuery(fetchMock, '/api/activity')).toEqual({ type: 'MERGE', limit: '100', offset: '0' }))
    expect(screen.getByTestId('location')).toHaveTextContent('/activity?type=MERGE')
  })

  it('renders rows with a trader column when not filtered by wallet', async () => {
    renderActivity('/activity', [
      activityRow(),
      activityRow({ activity_type: 'REDEEM', side: null, outcome: '', price: 0, usdc_size: 120, name: null, pseudonym: 'Brave-Otter' }),
      activityRow({ activity_type: 'REWARD', side: null, condition_id: '', title: null, outcome: null, price: 0, usdc_size: 3 }),
    ])
    const table = await screen.findByRole('table')
    expect(tableWithHeader('Trader')).toBe(table)
    // When, Trader, Type, Market, Outcome, Price, Shares, USDC
    expect(bodyRows(table).map((r) => r.slice(1))).toEqual([
      ['alice', 'BUY', 'Will it rain in London tomorrow?', 'Yes', '62¢', '100', '$62'],
      ['Brave-Otter', 'REDEEM', 'Will it rain in London tomorrow?', '—', '—', '100', '$120'],
      ['alice', 'REWARD', '—', '—', '—', '100', '$3'],
    ])
    expect(within(table).getByRole('link', { name: 'Brave-Otter' })).toHaveAttribute('href', `/traders/${WALLET}`)
    expect(within(table).getAllByRole('link', { name: 'Will it rain in London tomorrow?' })[0]).toHaveAttribute(
      'href',
      `/markets/${CONDITION}`,
    )
  })

  it('names the wallet chip from the rows and drops the trader column', async () => {
    renderActivity(`/activity?wallet=${WALLET}`, [activityRow()])
    expect(await screen.findByText('Trader: alice')).toBeInTheDocument()
    expect(screen.queryByRole('columnheader', { name: 'Trader' })).toBeNull()
  })

  it('clears the wallet chip', async () => {
    const { fetchMock } = renderActivity(`/activity?type=REDEEM&wallet=${WALLET}`)
    await screen.findByText('No activity matches.')
    await userEvent.click(screen.getByRole('button', { name: 'Clear filter' }))
    await waitFor(() => expect(lastQuery(fetchMock, '/api/activity')).toEqual({ type: 'REDEEM', limit: '100', offset: '0' }))
    expect(screen.queryByText(/^Trader:/)).toBeNull()
  })
})
