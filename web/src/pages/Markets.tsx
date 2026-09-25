import { Link, useSearchParams } from 'react-router'
import { qs, type Market, type Page } from '../api'
import { Loading, Pager, Signed } from '../components/ui'
import { cents, centsDelta, shortDate, usd } from '../format'
import { useApi } from '../useApi'
import { useDebounced } from '../useDebounced'

const LIMIT = 50

export function Markets() {
  const [params, setParams] = useSearchParams()
  const q = params.get('q') ?? ''
  const status = params.get('status') ?? 'live'
  const sort = params.get('sort') ?? 'volume'
  const offset = Number(params.get('offset') ?? 0)
  const debouncedQ = useDebounced(q, 300)

  const { data, error, loading } = useApi<Page<Market>>(
    `/api/markets${qs({ q: debouncedQ, status, sort, limit: LIMIT, offset })}`,
  )

  const set = (key: string, value: string) => {
    const next = new URLSearchParams(params)
    if (value) next.set(key, value)
    else next.delete(key)
    if (key !== 'offset') next.delete('offset')
    setParams(next, { replace: true })
  }

  return (
    <>
      <h1>Markets</h1>
      <div className="filters">
        <input type="search" placeholder="Search questions…" value={q} onChange={(e) => set('q', e.target.value)} />
        <select value={status} onChange={(e) => set('status', e.target.value)}>
          <option value="live">Live</option>
          <option value="closed">Closed</option>
          <option value="all">All</option>
        </select>
        <select value={sort} onChange={(e) => set('sort', e.target.value)}>
          <option value="volume">Volume</option>
          <option value="liquidity">Liquidity</option>
          <option value="ending">Ending soonest</option>
          <option value="spread">Widest spread</option>
          <option value="movers">Biggest 24h move</option>
        </select>
        {data && <span className="muted">{data.total.toLocaleString()} markets</span>}
      </div>

      {!data ? (
        <Loading error={error} loading={loading} />
      ) : (
        <>
          <div className={`scroll ${loading ? 'stale' : ''}`}>
            <table>
              <thead>
                <tr>
                  <th>Market</th>
                  <th>Outcomes</th>
                  <th className="num">Bid / Ask</th>
                  <th className="num" title="24h move of the first outcome">24h</th>
                  <th className="num">Volume</th>
                  <th className="num">Liquidity</th>
                  <th className="num">Ends</th>
                </tr>
              </thead>
              <tbody>
                {data.rows.length === 0 && (
                  <tr><td colSpan={7} className="muted">No markets match.</td></tr>
                )}
                {data.rows.map((m) => (
                  <tr key={m.id}>
                    <td className="wrap">
                      <div className="market-cell">
                        {m.icon ? <img src={m.icon} alt="" loading="lazy" /> : <span className="icon-ph" />}
                        <span>
                          <Link to={`/markets/${m.id}`}>{m.question ?? m.slug}</Link>
                          {m.closed && <span className="badge"> closed</span>}
                        </span>
                      </div>
                    </td>
                    <td className="outcomes">
                      {m.outcomes.map((o, i) => (
                        <span key={o}>{o} <b>{cents(m.outcome_prices[i])}</b></span>
                      ))}
                    </td>
                    <td className="num">{cents(m.best_bid)} / {cents(m.best_ask)}</td>
                    <td className="num"><Signed value={m.one_day_change}>{centsDelta(m.one_day_change)}</Signed></td>
                    <td className="num">{usd(m.volume, { compact: true })}</td>
                    <td className="num">{usd(m.liquidity, { compact: true })}</td>
                    <td className="num">{shortDate(m.end_date)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <Pager offset={offset} limit={LIMIT} total={data.total} onChange={(o) => set('offset', String(o))} />
        </>
      )}
    </>
  )
}
