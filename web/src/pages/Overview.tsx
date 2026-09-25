import { Link } from 'react-router'
import type { Overview as OverviewData } from '../api'
import { TradesTable, Loading, Signed, Tile } from '../components/ui'
import { cents, centsDelta, num, usd } from '../format'
import { useApi } from '../useApi'
import { useNow } from '../useNow'

export function Overview() {
  const { data, error, loading } = useApi<OverviewData>('/api/overview', 15_000)
  const now = useNow()

  if (!data) return <Loading error={error} loading={loading} />
  const t = data.totals

  return (
    <>
      <h1>Overview</h1>
      <div className="tiles">
        <Tile label="Live markets" value={num(t.live_markets)} sub={`${num(t.markets)} stored`} />
        <Tile label="Live volume" value={usd(t.live_volume, { compact: true })} sub="lifetime, open markets" />
        <Tile label="Live liquidity" value={usd(t.live_liquidity, { compact: true })} />
        <Tile label="Trades, 24h" value={num(t.trades_24h)} sub={`${usd(t.volume_24h, { compact: true })} traded`} />
        <Tile label="Traders seen" value={num(t.wallets)} />
      </div>

      <h2>Top markets by volume</h2>
      <div className="scroll">
        <table>
          <thead>
            <tr>
              <th>Market</th>
              <th className="num">Leading outcome</th>
              <th className="num">24h</th>
              <th className="num">Volume</th>
              <th className="num">Liquidity</th>
            </tr>
          </thead>
          <tbody>
            {data.top_markets.map((m) => {
              const lead = m.outcome_prices.reduce((b, p, i) => (p > m.outcome_prices[b] ? i : b), 0)
              return (
                <tr key={m.id}>
                  <td className="wrap">
                    <div className="market-cell">
                      {m.icon ? <img src={m.icon} alt="" loading="lazy" /> : <span className="icon-ph" />}
                      <Link to={`/markets/${m.id}`}>{m.question ?? m.slug}</Link>
                    </div>
                  </td>
                  <td className="num">
                    {m.outcomes[lead] ?? '—'} {cents(m.outcome_prices[lead])}
                  </td>
                  <td className="num"><Signed value={m.one_day_change}>{centsDelta(m.one_day_change)}</Signed></td>
                  <td className="num">{usd(m.volume, { compact: true })}</td>
                  <td className="num">{usd(m.liquidity, { compact: true })}</td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>

      <h2>Latest trades <Link className="more" to="/trades">all trades →</Link></h2>
      <TradesTable trades={data.recent_trades} now={now} />
    </>
  )
}
