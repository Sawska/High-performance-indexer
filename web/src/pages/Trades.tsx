import { useSearchParams } from 'react-router'
import { qs, type Trade } from '../api'
import { Chip, Loading, OffsetPager, TradesTable } from '../components/ui'
import { shortWallet } from '../format'
import { useApi } from '../useApi'
import { useNow } from '../useNow'

const LIMIT = 100

export function Trades() {
  const [params, setParams] = useSearchParams()
  const side = params.get('side') ?? ''
  const minUsd = params.get('min_usd') ?? ''
  const market = params.get('market') ?? ''
  const wallet = params.get('wallet') ?? ''
  const offset = Number(params.get('offset') ?? 0)
  const now = useNow()

  // Only the first page follows the feed; older pages stay put while read.
  const { data, error, loading } = useApi<Trade[]>(
    `/api/trades${qs({ side, min_usd: minUsd, market, wallet, limit: LIMIT, offset })}`,
    offset === 0 ? 15_000 : undefined,
  )

  const set = (key: string, value: string) => {
    const next = new URLSearchParams(params)
    if (value) next.set(key, value)
    else next.delete(key)
    if (key !== 'offset') next.delete('offset')
    setParams(next, { replace: true })
  }

  // Filters set by a link carry an id; the rows name what it is.
  const marketTitle = data?.[0]?.title ?? market.slice(0, 10) + '…'
  const walletName = data?.[0] && (data[0].name || data[0].pseudonym)

  return (
    <>
      <h1>Trades</h1>
      <div className="filters">
        <select value={side} onChange={(e) => set('side', e.target.value)}>
          <option value="">Buys and sells</option>
          <option value="BUY">Buys</option>
          <option value="SELL">Sells</option>
        </select>
        <select value={minUsd} onChange={(e) => set('min_usd', e.target.value)}>
          <option value="">Any size</option>
          <option value="100">≥ $100</option>
          <option value="1000">≥ $1,000</option>
          <option value="10000">≥ $10,000</option>
        </select>
        {market && <Chip onClear={() => set('market', '')}>Market: {marketTitle}</Chip>}
        {wallet && <Chip onClear={() => set('wallet', '')}>Trader: {walletName || shortWallet(wallet)}</Chip>}
        <span className="muted">{offset === 0 ? 'newest first, refreshes every 15s' : 'older trades'}</span>
      </div>
      {data ? (
        <>
          <div className={loading ? 'stale' : ''}>
            <TradesTable trades={data} now={now} showMarket={!market} />
          </div>
          <OffsetPager offset={offset} limit={LIMIT} count={data.length} onChange={(o) => set('offset', String(o))} />
        </>
      ) : (
        <Loading error={error} loading={loading} />
      )}
    </>
  )
}
