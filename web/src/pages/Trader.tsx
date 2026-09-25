import { useState } from 'react'
import { Link, useParams } from 'react-router'
import type { TraderDetail } from '../api'
import { LineChart } from '../components/LineChart'
import { ActivityType, Loading, Segmented, Signed, Tile, When } from '../components/ui'
import { cents, num, pct, shares, shortDate, traderName, usd } from '../format'
import { useApi } from '../useApi'
import { useNow } from '../useNow'

type Chart = 'pnl' | 'value'

export function Trader() {
  const { wallet } = useParams()
  const { data, error, loading } = useApi<TraderDetail>(`/api/traders/${encodeURIComponent(wallet ?? '')}`, 60_000)
  const now = useNow(10_000)
  const [chart, setChart] = useState<Chart>('pnl')
  const [type, setType] = useState('')

  if (!data) return <Loading error={error} loading={loading} />
  const t = data.trader
  // Redeemable means resolved; the losing side redeems for nothing.
  const resolved = data.positions.filter((p) => p.redeemable)
  const toRedeem = resolved.reduce((s, p) => s + p.current_value, 0)
  const types = [...new Set(data.activity.map((a) => a.activity_type))].sort()
  const activity = type ? data.activity.filter((a) => a.activity_type === type) : data.activity

  return (
    <>
      <div className="crumbs"><Link to="/traders">Traders</Link> /</div>
      <div className="trader-head">
        {t.profile_image && <img src={t.profile_image} alt="" />}
        <div>
          <h1>
            {traderName(t)}
            {data.profile.verified && <span className="verified" title="Verified on Polymarket">✓</span>}
          </h1>
          <div className="muted mono">
            {t.proxy_wallet}{' · '}
            <a href={`https://polymarket.com/profile/${t.proxy_wallet}`} target="_blank" rel="noreferrer">polymarket.com ↗</a>
          </div>
          {data.profile.bio && <p className="bio">{data.profile.bio}</p>}
        </div>
      </div>

      <div className="tiles">
        <Tile label="PnL" value={<Signed value={t.pnl}>{usd(t.pnl, { compact: true, signed: true })}</Signed>} />
        <Tile label="Portfolio value" value={usd(t.value, { compact: true })} />
        <Tile label="Volume (stored trades)" value={usd(t.volume, { compact: true })} />
        <Tile label="Trades" value={num(t.trades)} sub={<Link to={`/trades?wallet=${t.proxy_wallet}`}>view trades →</Link>} />
        <Tile
          label="Open positions"
          value={num(data.positions.length)}
          sub={resolved.length > 0 ? `${resolved.length} resolved${toRedeem > 0 ? ` · ${usd(toRedeem, { compact: true })} to redeem` : ''}` : undefined}
        />
      </div>

      <div className="section-head">
        <h2>{chart === 'pnl' ? 'PnL over time' : 'Portfolio value over time'}</h2>
        <Segmented<Chart>
          value={chart}
          onChange={setChart}
          options={[
            { value: 'pnl', label: 'PnL' },
            { value: 'value', label: 'Portfolio value' },
          ]}
        />
      </div>
      <div className="panel">
        {chart === 'pnl' ? (
          <LineChart
            series={[{ name: 'PnL', points: data.pnl.map((p) => ({ t: Date.parse(p.ts), v: p.pnl })) }]}
            formatValue={(v) => usd(v, { compact: true })}
          />
        ) : (
          <LineChart
            series={[{ name: 'Value', points: data.values.map((p) => ({ t: Date.parse(p.ts), v: p.value })) }]}
            formatValue={(v) => usd(v, { compact: true })}
          />
        )}
      </div>

      <h2>Positions</h2>
      {data.positions.length === 0 ? (
        <div className="empty">No positions stored yet.</div>
      ) : (
        <div className="scroll">
          <table>
            <thead>
              <tr>
                <th>Market</th>
                <th>Outcome</th>
                <th className="num">Shares</th>
                <th className="num">Avg</th>
                <th className="num">Now</th>
                <th className="num">Cost</th>
                <th className="num">Value</th>
                <th className="num">Unrealized</th>
                <th className="num">Realized</th>
                <th className="num">Ends</th>
              </tr>
            </thead>
            <tbody>
              {data.positions.map((p) => (
                <tr key={p.asset}>
                  <td className="wrap">
                    {p.has_market ? (
                      <Link to={`/markets/${p.condition_id}`}>{p.question}</Link>
                    ) : (
                      <span title="Market not scraped yet">{p.question ?? <span className="mono muted">{p.condition_id}</span>}</span>
                    )}
                    {p.redeemable && (
                      <span className={`pill ${p.current_value > 0 ? 'ok' : ''}`} title="Market resolved; shares can be redeemed">
                        {p.current_value > 0 ? 'redeemable' : 'resolved'}
                      </span>
                    )}
                    {p.mergeable && <span className="pill" title="Holds both sides; can merge back to USDC">mergeable</span>}
                  </td>
                  <td>{p.outcome ?? '—'}</td>
                  <td className="num">{shares(p.size)}</td>
                  <td className="num">{cents(p.avg_price)}</td>
                  <td className="num">{cents(p.cur_price)}</td>
                  <td className="num">{usd(p.initial_value)}</td>
                  <td className="num">{usd(p.current_value)}</td>
                  <td className="num">
                    <Signed value={p.cash_pnl}>{usd(p.cash_pnl, { signed: true })} <span className="muted">({pct(p.percent_pnl)})</span></Signed>
                  </td>
                  <td className="num"><Signed value={p.realized_pnl}>{usd(p.realized_pnl, { signed: true })}</Signed></td>
                  <td className="num">{shortDate(p.end_date)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="section-head">
        <h2>Activity <Link className="more" to={`/activity?wallet=${t.proxy_wallet}`}>all activity →</Link></h2>
        {types.length > 1 && (
          <Segmented value={type} onChange={setType} options={[{ value: '', label: 'All' }, ...types.map((v) => ({ value: v, label: v.toLowerCase().replaceAll('_', ' ') }))]} />
        )}
      </div>
      {activity.length === 0 ? (
        <div className="empty">No activity stored yet.</div>
      ) : (
        <div className="scroll">
          <table>
            <thead>
              <tr>
                <th>When</th>
                <th>Type</th>
                <th>Market</th>
                <th>Outcome</th>
                <th className="num">Price</th>
                <th className="num">Shares</th>
                <th className="num">USDC</th>
              </tr>
            </thead>
            <tbody>
              {activity.map((a, i) => (
                <tr key={i}>
                  <td><When ts={a.ts} now={now} tx={a.transaction_hash} /></td>
                  <td><ActivityType type={a.activity_type} side={a.side} /></td>
                  <td className="wrap"><Link to={`/markets/${a.condition_id}`}>{a.title ?? a.condition_id}</Link></td>
                  <td>{a.outcome ?? '—'}</td>
                  <td className="num">{a.price > 0 ? cents(a.price) : '—'}</td>
                  <td className="num">{shares(a.size)}</td>
                  <td className="num">{usd(a.usdc_size)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  )
}
