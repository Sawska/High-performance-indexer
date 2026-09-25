import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { LineChart, type Series } from './LineChart'

const series: Series[] = [{ name: 'Yes', points: [{ t: 0, v: 0.2 }, { t: 86_400_000, v: 0.4 }] }]
const cents = (v: number) => `${Math.round(v * 100)}¢`

/** Records what ResizeObserver watches and lets a test report a new width. */
function stubResizeObserver() {
  const watched: Element[] = []
  let report: ResizeObserverCallback = () => {}
  vi.stubGlobal(
    'ResizeObserver',
    class {
      constructor(cb: ResizeObserverCallback) {
        report = cb
      }
      observe(el: Element) {
        watched.push(el)
      }
      unobserve() {}
      disconnect() {}
    },
  )
  const resize = (width: number) =>
    report([{ contentRect: { width } } as ResizeObserverEntry], {} as ResizeObserver)
  return { watched, resize }
}

describe('LineChart', () => {
  it('shows an empty state without data', () => {
    stubResizeObserver()
    render(<LineChart series={[{ name: 'Yes', points: [] }]} formatValue={cents} />)
    expect(screen.getByText('No data yet.')).toBeInTheDocument()
  })

  it('keeps tracking its width when data arrives after an empty first render', async () => {
    const { watched, resize } = stubResizeObserver()
    const { container, rerender } = render(<LineChart series={[]} formatValue={cents} />)
    expect(watched).toHaveLength(1)

    rerender(<LineChart series={series} formatValue={cents} />)
    const svg = container.querySelector('svg')!
    expect(watched[0]).toContainElement(svg)

    resize(900)
    expect(await screen.findByRole('img')).toHaveAttribute('width', '900')
  })

  it('draws one line per series with a legend when there are two', () => {
    stubResizeObserver()
    const two: Series[] = [...series, { name: 'No', points: [{ t: 0, v: 0.8 }, { t: 86_400_000, v: 0.6 }] }]
    const { container } = render(<LineChart series={two} formatValue={cents} yDomain={[0, 1]} />)
    expect(container.querySelectorAll('polyline')).toHaveLength(2)
    expect(screen.getByText('No')).toBeInTheDocument()
  })

  it('pads a flat series so its tick labels differ', () => {
    stubResizeObserver()
    const flat: Series[] = [{ name: 'Value', points: [{ t: 0, v: 763_600 }, { t: 1, v: 763_610 }] }]
    const { container } = render(<LineChart series={flat} formatValue={(v) => String(Math.round(v))} />)
    const ticks = [...container.querySelectorAll('text.axis')].slice(0, -2).map((t) => t.textContent)
    expect(ticks.length).toBeGreaterThan(1)
    expect(new Set(ticks).size).toBe(ticks.length)
  })
})
