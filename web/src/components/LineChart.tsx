import { useLayoutEffect, useMemo, useRef, useState } from 'react'

export type Series = { name: string; points: { t: number; v: number }[] }

type Props = {
  series: Series[]
  height?: number
  formatValue: (v: number) => string
  yDomain?: [number, number]
}

function niceTicks(lo: number, hi: number, target = 4): number[] {
  const raw = (hi - lo) / target
  const mag = 10 ** Math.floor(Math.log10(raw))
  const step = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((s) => s >= raw) ?? raw
  const ticks: number[] = []
  for (let v = Math.ceil(lo / step) * step; v <= hi + step * 1e-9; v += step) ticks.push(v)
  return ticks
}

const PAD = { top: 12, right: 12, bottom: 24, left: 56 }
const MAX_SERIES = 2

export function LineChart({ series, height = 240, formatValue, yDomain }: Props) {
  const wrap = useRef<HTMLDivElement>(null)
  const [width, setWidth] = useState(600)
  const [hover, setHover] = useState<number | null>(null)

  useLayoutEffect(() => {
    const el = wrap.current
    if (!el) return
    const ro = new ResizeObserver(([e]) => setWidth(Math.max(240, e.contentRect.width)))
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  const shown = series.slice(0, MAX_SERIES).filter((s) => s.points.length > 0)

  const scale = useMemo(() => {
    const all = shown.flatMap((s) => s.points)
    if (all.length === 0) return null
    const t0 = Math.min(...all.map((p) => p.t))
    const t1 = Math.max(...all.map((p) => p.t))
    let [v0, v1] = yDomain ?? [Math.min(...all.map((p) => p.v)), Math.max(...all.map((p) => p.v))]
    // A near-flat series would otherwise stretch noise across the whole
    // height and repeat one tick label.
    const minSpan = Math.max(Math.abs(v0), Math.abs(v1)) * 0.02 || 2
    if (v1 - v0 < minSpan) {
      const mid = (v0 + v1) / 2
      v0 = mid - minSpan / 2
      v1 = mid + minSpan / 2
    }
    const w = width - PAD.left - PAD.right
    const h = height - PAD.top - PAD.bottom
    const x = (t: number) => PAD.left + (t1 === t0 ? w / 2 : ((t - t0) / (t1 - t0)) * w)
    const y = (v: number) => PAD.top + h - ((v - v0) / (v1 - v0)) * h
    const ticks = niceTicks(v0, v1)
    return { t0, t1, x, y, ticks, w }
  }, [shown, width, height, yDomain])

  if (!scale) return <div className="empty">No data yet.</div>

  const { t0, t1, x, y, ticks } = scale

  const hoverT = hover === null ? null : t0 + ((hover - PAD.left) / scale.w) * (t1 - t0)
  const nearest =
    hoverT === null
      ? []
      : shown.map((s) =>
          s.points.reduce((best, p) => (Math.abs(p.t - hoverT) < Math.abs(best.t - hoverT) ? p : best)),
        )

  const dateFmt = (t: number) =>
    new Date(t).toLocaleDateString(undefined, t1 - t0 < 3 * 86400_000 ? { hour: 'numeric', minute: '2-digit' } : { month: 'short', day: 'numeric' })

  return (
    <div className="chart" ref={wrap}>
      {shown.length > 1 && (
        <div className="legend">
          {shown.map((s, i) => (
            <span key={s.name}>
              <i style={{ background: `var(--series-${i + 1})` }} />
              {s.name}
            </span>
          ))}
        </div>
      )}
      <svg
        width={width}
        height={height}
        onMouseMove={(e) => {
          const r = e.currentTarget.getBoundingClientRect()
          const px = e.clientX - r.left
          setHover(px >= PAD.left && px <= width - PAD.right ? px : null)
        }}
        onMouseLeave={() => setHover(null)}
        role="img"
        aria-label={shown.map((s) => s.name).join(', ')}
      >
        {ticks.map((v) => (
          <g key={v}>
            <line className="grid" x1={PAD.left} x2={width - PAD.right} y1={y(v)} y2={y(v)} />
            <text className="axis" x={PAD.left - 8} y={y(v)} dy="0.32em" textAnchor="end">
              {formatValue(v)}
            </text>
          </g>
        ))}
        <text className="axis" x={PAD.left} y={height - 6}>{dateFmt(t0)}</text>
        <text className="axis" x={width - PAD.right} y={height - 6} textAnchor="end">{dateFmt(t1)}</text>

        {shown.map((s, i) => (
          <polyline
            key={s.name}
            fill="none"
            stroke={`var(--series-${i + 1})`}
            strokeWidth={2}
            strokeLinejoin="round"
            points={s.points.map((p) => `${x(p.t)},${y(p.v)}`).join(' ')}
          />
        ))}

        {hoverT !== null && nearest.length > 0 && (
          <g>
            <line className="crosshair" x1={x(nearest[0].t)} x2={x(nearest[0].t)} y1={PAD.top} y2={height - PAD.bottom} />
            {nearest.map((p, i) => (
              <circle key={i} cx={x(p.t)} cy={y(p.v)} r={4} fill={`var(--series-${i + 1})`} stroke="var(--panel)" strokeWidth={2} />
            ))}
          </g>
        )}
      </svg>
      {hoverT !== null && nearest.length > 0 && (
        <div
          className="tooltip"
          style={{ left: Math.min(x(nearest[0].t) + 12, width - 170), top: PAD.top }}
        >
          <div className="muted">{new Date(nearest[0].t).toLocaleString()}</div>
          {nearest.map((p, i) => (
            <div key={i}>
              <i style={{ background: `var(--series-${i + 1})` }} />
              {shown[i].name} <b>{formatValue(p.v)}</b>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
