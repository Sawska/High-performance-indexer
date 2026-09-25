import { Link, useSearchParams } from 'react-router'
import { qs, type Page, type Trader } from '../api'
import { Loading, Pager, Signed } from '../components/ui'
import { num, shortWallet, traderName, usd } from '../format'
import { useApi } from '../useApi'
import { useDebounced } from '../useDebounced'

const LIMIT = 50

export function Traders() {
  const [params, setParams] = useSearchParams()
  const q = params.get('q') ?? ''
  const sort = params.get('sort') ?? 'pnl'
  const offset = Number(params.get('offset') ?? 0)
  const debouncedQ = useDebounced(q, 300)

  const { data, error, loading } = useApi<Page<Trader>>(
    `/api/traders${qs({ q: debouncedQ, sort, limit: LIMIT, offset })}`,
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
      <h1>Traders</h1>
      <div className="filters">
        <input type="search" placeholder="Name or wallet…" value={q} onChange={(e) => set('q', e.target.value)} />
        <select value={sort} onChange={(e) => set('sort', e.target.value)}>
          <option value="pnl">Top PnL</option>
          <option value="loss">Worst PnL</option>
          <option value="value">Portfolio value</option>
          <option value="volume">Volume traded</option>
        </select>
        {data && <span className="muted">{data.total.toLocaleString()} traders</span>}
      </div>

      {!data ? (
        <Loading error={error} loading={loading} />
      ) : (
        <>
          <div className={`scroll ${loading ? 'stale' : ''}`}>
            <table>
              <thead>
                <tr>
                  <th className="num">#</th>
                  <th>Trader</th>
                  <th className="num">PnL</th>
                  <th className="num">Portfolio</th>
                  <th className="num">Volume</th>
                  <th className="num">Trades</th>
                </tr>
              </thead>
              <tbody>
                {data.rows.length === 0 && (
                  <tr><td colSpan={6} className="muted">No traders match.</td></tr>
                )}
                {data.rows.map((t, i) => (
                  <tr key={t.proxy_wallet}>
                    <td className="num muted">{offset + i + 1}</td>
                    <td>
                      <Link to={`/traders/${t.proxy_wallet}`}>{traderName(t)}</Link>
                      {(t.name || t.pseudonym) && <span className="muted mono"> {shortWallet(t.proxy_wallet)}</span>}
                    </td>
                    <td className="num"><Signed value={t.pnl}>{usd(t.pnl, { compact: true, signed: true })}</Signed></td>
                    <td className="num">{usd(t.value, { compact: true })}</td>
                    <td className="num">{usd(t.volume, { compact: true })}</td>
                    <td className="num">{num(t.trades)}</td>
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
