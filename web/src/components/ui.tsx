import type { ReactNode } from 'react'
import { Link } from 'react-router'
import type { Trade } from '../api'
import { ago, cents, shares, traderName, txUrl, usd } from '../format'

export function Tile({ label, value, sub }: { label: string; value: ReactNode; sub?: ReactNode }) {
  return (
    <div className="tile">
      <div className="label">{label}</div>
      <div className="value">{value}</div>
      {sub && <div className="sub">{sub}</div>}
    </div>
  )
}

export function Loading({ error, loading }: { error: string | null; loading: boolean }) {
  if (error) return <div className="error-box">{error}</div>
  if (loading) return <div className="empty">Loading…</div>
  return null
}

export function Signed({ value, children }: { value: number | null; children: ReactNode }) {
  const cls = value == null || value === 0 ? '' : value > 0 ? 'pos' : 'neg'
  return <span className={cls}>{children}</span>
}

export function Pager({ offset, limit, total, onChange }: { offset: number; limit: number; total: number; onChange: (o: number) => void }) {
  if (total <= limit) return null
  return (
    <div className="pager">
      <button disabled={offset === 0} onClick={() => onChange(Math.max(0, offset - limit))}>← Prev</button>
      <span className="muted">
        {offset + 1}–{Math.min(offset + limit, total)} of {total.toLocaleString()}
      </span>
      <button disabled={offset + limit >= total} onClick={() => onChange(offset + limit)}>Next →</button>
    </div>
  )
}

/** For feeds that return no total: Next shows while a page comes back full. */
export function OffsetPager({ offset, limit, count, onChange }: { offset: number; limit: number; count: number; onChange: (o: number) => void }) {
  if (offset === 0 && count < limit) return null
  return (
    <div className="pager">
      <button disabled={offset === 0} onClick={() => onChange(Math.max(0, offset - limit))}>← Newer</button>
      <span className="muted">{offset + 1}–{offset + count}</span>
      <button disabled={count < limit} onClick={() => onChange(offset + limit)}>Older →</button>
    </div>
  )
}

export function Segmented<T extends string>({ value, options, onChange }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <div className="segmented" role="group">
      {options.map((o) => (
        <button key={o.value} aria-pressed={o.value === value} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  )
}

/** A removable filter, for filters set by a link rather than a control. */
export function Chip({ children, onClear }: { children: ReactNode; onClear: () => void }) {
  return (
    <span className="chip">
      <span>{children}</span>
      <button aria-label="Clear filter" onClick={onClear}>×</button>
    </span>
  )
}

/** Relative time, linking to the transaction when there is one. */
export function When({ ts, now, tx }: { ts: string; now: number; tx?: string | null }) {
  const title = new Date(ts).toLocaleString()
  if (!tx) return <span className="muted" title={title}>{ago(ts, now)}</span>
  return <a className="muted" href={txUrl(tx)} target="_blank" rel="noreferrer" title={`${title} · view transaction`}>{ago(ts, now)}</a>
}

/** A trade shows its side; other /activity kinds show the kind. */
export function ActivityType({ type, side }: { type: string; side: string | null }) {
  if (type === 'TRADE' && side) return <span className={`badge ${side === 'BUY' ? 'pos' : 'neg'}`}>{side}</span>
  return <span className="badge">{type.replace('_', ' ')}</span>
}

export function TradesTable({ trades, showMarket = true, now }: { trades: Trade[]; showMarket?: boolean; now: number }) {
  if (trades.length === 0) return <div className="empty">No trades stored yet.</div>
  return (
    <div className="scroll">
      <table>
        <thead>
          <tr>
            <th>When</th>
            <th>Trader</th>
            {showMarket && <th>Market</th>}
            <th>Side</th>
            <th>Outcome</th>
            <th className="num">Price</th>
            <th className="num">Shares</th>
            <th className="num">Value</th>
          </tr>
        </thead>
        <tbody>
          {trades.map((t, i) => (
            <tr key={`${t.ts}-${t.proxy_wallet}-${i}`}>
              <td><When ts={t.ts} now={now} tx={t.transaction_hash} /></td>
              <td><Link to={`/traders/${t.proxy_wallet}`}>{traderName(t)}</Link></td>
              {showMarket && (
                <td className="wrap"><Link to={`/markets/${t.condition_id}`}>{t.title ?? t.condition_id}</Link></td>
              )}
              <td><span className={`badge ${t.side === 'BUY' ? 'pos' : 'neg'}`}>{t.side}</span></td>
              <td>{t.outcome ?? '—'}</td>
              <td className="num">{cents(t.price)}</td>
              <td className="num">{shares(t.size)}</td>
              <td className="num">{usd(t.size * t.price)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
