import { Link, useSearchParams } from 'react-router'
import { qs, type ActivityFeedRow } from '../api'
import { ActivityType, Chip, Loading, OffsetPager, When } from '../components/ui'
import { cents, shares, shortWallet, traderName, usd } from '../format'
import { useApi } from '../useApi'
import { useNow } from '../useNow'

const LIMIT = 100

// What data-api /activity reports; TRADE rows repeat what Trades shows.
const TYPES = ['TRADE', 'SPLIT', 'MERGE', 'REDEEM', 'CONVERSION', 'REWARD', 'YIELD', 'MAKER_REBATE', 'TAKER_REBATE']

export function Activity() {
  const [params, setParams] = useSearchParams()
  const type = params.get('type') ?? ''
  const wallet = params.get('wallet') ?? ''
  const offset = Number(params.get('offset') ?? 0)
  const now = useNow()

  const { data, error, loading } = useApi<ActivityFeedRow[]>(
    `/api/activity${qs({ type, wallet, limit: LIMIT, offset })}`,
    offset === 0 ? 30_000 : undefined,
  )

  const set = (key: string, value: string) => {
    const next = new URLSearchParams(params)
    if (value) next.set(key, value)
    else next.delete(key)
    if (key !== 'offset') next.delete('offset')
    setParams(next, { replace: true })
  }

  const walletName = data?.[0] && (data[0].name || data[0].pseudonym)

  return (
    <>
      <h1>Activity</h1>
      <div className="filters">
        <select value={type} onChange={(e) => set('type', e.target.value)}>
          <option value="">All types</option>
          {TYPES.map((t) => (
            <option key={t} value={t}>{t.toLowerCase().replaceAll('_', ' ')}</option>
          ))}
        </select>
        {wallet && <Chip onClear={() => set('wallet', '')}>Trader: {walletName || shortWallet(wallet)}</Chip>}
        <span className="muted">on-chain actions of scraped wallets: trades, splits, merges, redemptions, rewards</span>
      </div>

      {!data ? (
        <Loading error={error} loading={loading} />
      ) : data.length === 0 ? (
        <div className="empty">No activity matches.</div>
      ) : (
        <>
          <div className={`scroll ${loading ? 'stale' : ''}`}>
            <table>
              <thead>
                <tr>
                  <th>When</th>
                  {!wallet && <th>Trader</th>}
                  <th>Type</th>
                  <th>Market</th>
                  <th>Outcome</th>
                  <th className="num">Price</th>
                  <th className="num">Shares</th>
                  <th className="num">USDC</th>
                </tr>
              </thead>
              <tbody>
                {data.map((a, i) => (
                  <tr key={`${a.transaction_hash}-${a.proxy_wallet}-${i}`}>
                    <td><When ts={a.ts} now={now} tx={a.transaction_hash} /></td>
                    {!wallet && <td><Link to={`/traders/${a.proxy_wallet}`}>{traderName(a)}</Link></td>}
                    <td><ActivityType type={a.activity_type} side={a.side} /></td>
                    <td className="wrap">
                      {a.condition_id ? <Link to={`/markets/${a.condition_id}`}>{a.title ?? a.condition_id}</Link> : <span className="muted">—</span>}
                    </td>
                    <td>{a.outcome || '—'}</td>
                    <td className="num">{a.price > 0 ? cents(a.price) : '—'}</td>
                    <td className="num">{shares(a.size)}</td>
                    <td className="num">{usd(a.usdc_size)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <OffsetPager offset={offset} limit={LIMIT} count={data.length} onChange={(o) => set('offset', String(o))} />
        </>
      )}
    </>
  )
}
