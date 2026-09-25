import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { describe, expect, it, vi } from 'vitest'
import { CONDITION, trade, TX, WALLET } from '../test/fixtures'
import { ActivityType, Chip, Loading, OffsetPager, Pager, Segmented, Signed, Tile, TradesTable, When } from './ui'

const NOW = Date.parse('2026-09-25T12:00:00Z')

describe('Tile', () => {
  it('renders label, value and optional sub', () => {
    const { container, rerender } = render(<Tile label="Volume" value="$1.2M" sub="lifetime" />)
    expect(container.querySelector('.label')).toHaveTextContent('Volume')
    expect(container.querySelector('.value')).toHaveTextContent('$1.2M')
    expect(container.querySelector('.sub')).toHaveTextContent('lifetime')
    rerender(<Tile label="Volume" value="$1.2M" />)
    expect(container.querySelector('.sub')).toBeNull()
  })
})

describe('Loading', () => {
  it('prefers the error, then the spinner, else nothing', () => {
    const { container, rerender } = render(<Loading error="boom" loading />)
    expect(screen.getByText('boom')).toHaveClass('error-box')
    rerender(<Loading error={null} loading />)
    expect(screen.getByText('Loading…')).toBeInTheDocument()
    rerender(<Loading error={null} loading={false} />)
    expect(container).toBeEmptyDOMElement()
  })
})

describe('Signed', () => {
  it('colors by sign', () => {
    render(
      <>
        <Signed value={3}>up</Signed>
        <Signed value={-3}>down</Signed>
        <Signed value={0}>flat</Signed>
        <Signed value={null}>none</Signed>
      </>,
    )
    expect(screen.getByText('up')).toHaveClass('pos')
    expect(screen.getByText('down')).toHaveClass('neg')
    expect(screen.getByText('flat').className).toBe('')
    expect(screen.getByText('none').className).toBe('')
  })
})

describe('Pager', () => {
  it('is hidden when everything fits on one page', () => {
    const { container, rerender } = render(<Pager offset={0} limit={50} total={50} onChange={() => {}} />)
    expect(container).toBeEmptyDOMElement()
    rerender(<Pager offset={0} limit={50} total={0} onChange={() => {}} />)
    expect(container).toBeEmptyDOMElement()
  })

  it('disables Prev on the first page and pages forward', async () => {
    const onChange = vi.fn()
    render(<Pager offset={0} limit={50} total={1234} onChange={onChange} />)
    expect(screen.getByRole('button', { name: '← Prev' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Next →' })).toBeEnabled()
    expect(screen.getByText('1–50 of 1,234')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Next →' }))
    expect(onChange).toHaveBeenCalledWith(50)
  })

  it('disables Next on the last page and pages back', async () => {
    const onChange = vi.fn()
    render(<Pager offset={100} limit={50} total={120} onChange={onChange} />)
    expect(screen.getByRole('button', { name: 'Next →' })).toBeDisabled()
    expect(screen.getByText('101–120 of 120')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: '← Prev' }))
    expect(onChange).toHaveBeenCalledWith(50)
  })

  it('never pages back past zero', async () => {
    const onChange = vi.fn()
    render(<Pager offset={20} limit={50} total={120} onChange={onChange} />)
    await userEvent.click(screen.getByRole('button', { name: '← Prev' }))
    expect(onChange).toHaveBeenCalledWith(0)
  })
})

describe('OffsetPager', () => {
  it('is hidden on the first page when it came back short', () => {
    const { container } = render(<OffsetPager offset={0} limit={100} count={37} onChange={() => {}} />)
    expect(container).toBeEmptyDOMElement()
  })

  it('offers Older only while pages come back full', async () => {
    const onChange = vi.fn()
    const { rerender } = render(<OffsetPager offset={0} limit={100} count={100} onChange={onChange} />)
    expect(screen.getByRole('button', { name: '← Newer' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Older →' })).toBeEnabled()
    expect(screen.getByText('1–100')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Older →' }))
    expect(onChange).toHaveBeenLastCalledWith(100)

    rerender(<OffsetPager offset={100} limit={100} count={40} onChange={onChange} />)
    expect(screen.getByRole('button', { name: 'Older →' })).toBeDisabled()
    expect(screen.getByRole('button', { name: '← Newer' })).toBeEnabled()
    expect(screen.getByText('101–140')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: '← Newer' }))
    expect(onChange).toHaveBeenLastCalledWith(0)
  })

  it('stays visible on a later page that came back empty', () => {
    render(<OffsetPager offset={100} limit={100} count={0} onChange={() => {}} />)
    expect(screen.getByRole('button', { name: '← Newer' })).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Older →' })).toBeDisabled()
  })
})

describe('Segmented', () => {
  const options = [
    { value: 'price', label: 'Price' },
    { value: 'spread', label: 'Spread' },
  ] as const

  it('marks the selected option pressed and reports clicks', async () => {
    const onChange = vi.fn()
    render(<Segmented value="price" options={[...options]} onChange={onChange} />)
    expect(screen.getByRole('group')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Price' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByRole('button', { name: 'Spread' })).toHaveAttribute('aria-pressed', 'false')
    await userEvent.click(screen.getByRole('button', { name: 'Spread' }))
    expect(onChange).toHaveBeenCalledWith('spread')
  })
})

describe('Chip', () => {
  it('shows its content and clears', async () => {
    const onClear = vi.fn()
    render(<Chip onClear={onClear}>Market: Rain</Chip>)
    expect(screen.getByText('Market: Rain')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Clear filter' }))
    expect(onClear).toHaveBeenCalledTimes(1)
  })
})

describe('When', () => {
  const ts = '2026-09-25T11:55:00Z'

  it('links to polygonscan when there is a transaction', () => {
    render(<When ts={ts} now={NOW} tx={TX} />)
    const link = screen.getByRole('link', { name: '5m 0s ago' })
    expect(link).toHaveAttribute('href', `https://polygonscan.com/tx/${TX}`)
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noreferrer')
  })

  it('renders plain text without a transaction', () => {
    const { rerender } = render(<When ts={ts} now={NOW} />)
    expect(screen.getByText('5m 0s ago').tagName).toBe('SPAN')
    expect(screen.queryByRole('link')).toBeNull()
    rerender(<When ts={ts} now={NOW} tx={null} />)
    expect(screen.queryByRole('link')).toBeNull()
  })
})

describe('ActivityType', () => {
  it('shows the side of a trade, colored', () => {
    render(
      <>
        <ActivityType type="TRADE" side="BUY" />
        <ActivityType type="TRADE" side="SELL" />
      </>,
    )
    expect(screen.getByText('BUY')).toHaveClass('badge', 'pos')
    expect(screen.getByText('SELL')).toHaveClass('badge', 'neg')
  })

  it('shows the kind for other activity, underscores as spaces', () => {
    render(
      <>
        <ActivityType type="REDEEM" side={null} />
        <ActivityType type="MAKER_REBATE" side={null} />
        <ActivityType type="NEW_KIND_OF_THING" side={null} />
        <ActivityType type="TRADE" side={null} />
      </>,
    )
    expect(screen.getByText('REDEEM').className).toBe('badge')
    expect(screen.getByText('MAKER REBATE').className).toBe('badge')
    expect(screen.getByText('NEW KIND OF THING').className).toBe('badge')
    expect(screen.getByText('TRADE').className).toBe('badge')
  })
})

describe('TradesTable', () => {
  const renderTable = (props: Parameters<typeof TradesTable>[0]) =>
    render(
      <MemoryRouter>
        <TradesTable {...props} />
      </MemoryRouter>,
    )

  it('shows an empty state', () => {
    renderTable({ trades: [], now: NOW })
    expect(screen.getByText('No trades stored yet.')).toBeInTheDocument()
    expect(screen.queryByRole('table')).toBeNull()
  })

  it('links each row to its trader and market', () => {
    renderTable({
      trades: [
        trade({ side: 'BUY', price: 0.62, size: 250 }),
        trade({ name: null, pseudonym: null, side: 'SELL', title: null, outcome: null }),
      ],
      now: NOW,
    })
    const [first, second] = screen.getAllByRole('row').slice(1)
    expect(within(first).getByRole('link', { name: 'alice' })).toHaveAttribute('href', `/traders/${WALLET}`)
    expect(within(first).getByRole('link', { name: 'Will it rain in London tomorrow?' })).toHaveAttribute(
      'href',
      `/markets/${CONDITION}`,
    )
    expect(within(first).getByText('BUY')).toHaveClass('pos')
    expect(within(first).getByText('62¢')).toBeInTheDocument()
    expect(within(first).getByText('$155')).toBeInTheDocument()
    expect(within(first).getByRole('link', { name: '1h 0m ago' })).toHaveAttribute('href', `https://polygonscan.com/tx/${TX}`)

    // No name falls back to the short wallet; no title to the condition id.
    expect(within(second).getByRole('link', { name: '0xabcd…1234' })).toHaveAttribute('href', `/traders/${WALLET}`)
    expect(within(second).getByRole('link', { name: CONDITION })).toHaveAttribute('href', `/markets/${CONDITION}`)
    expect(within(second).getByText('SELL')).toHaveClass('neg')
    expect(within(second).getByText('—')).toBeInTheDocument()
  })

  it('hides the market column with showMarket=false', () => {
    renderTable({ trades: [trade()], now: NOW, showMarket: false })
    expect(screen.queryByRole('columnheader', { name: 'Market' })).toBeNull()
    expect(screen.queryByRole('link', { name: 'Will it rain in London tomorrow?' })).toBeNull()
    expect(screen.getAllByRole('columnheader')).toHaveLength(7)
  })
})
