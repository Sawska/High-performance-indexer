import { useState, type ReactNode } from 'react'
import { Link, useParams } from 'react-router'
import type { MarketDetail, PricePoint } from '../api'
import { LineChart, type Series } from '../components/LineChart'
import { Loading, Segmented, Tile, TradesTable, When } from '../components/ui'
import { ago, cents, centsDelta, shares, shortDate, traderName, usd } from '../format'
import { useApi } from '../useApi'
import { useNow } from '../useNow'

type Chart = 'price' | 'spread'
type Feed = 'fills' | 'changes' | 'ticks'

export function Market() {
  const { id } = useParams()
  const { data, error, loading } = useApi<MarketDetail>(`/api/markets/${encodeURIComponent(id ?? '')}`, 30_000)
  const now = useNow()
  const [chart, setChart] = useState<Chart>('price')

  if (!data) return <Loading error={error} loading={loading} />
  const { market: m, info } = data

  const outcomeOf = (token: string) => m.outcomes[m.token_ids.indexOf(token)] ?? token.slice(0, 8)

  const toSeries = (points: PricePoint[]): Series[] =>
    m.token_ids.map((token) => ({
      name: outcomeOf(token),
      points: points.filter((p) => p.token === token).map((p) => ({ t: Date.parse(p.ts), v: p.price })),
    }))

  const negRisk = data.books.some((b) => b.neg_risk)
  const moves = [
    m.one_day_change != null && `24h ${centsDelta(m.one_day_change)}`,
    info.one_week_change != null && `7d ${centsDelta(info.one_week_change)}`,
  ].filter(Boolean).join(' · ')

  return (
    <>
      <div className="crumbs"><Link to="/markets">Markets</Link> /</div>
      <div className="market-head">
        {info.image && <img src={info.image} alt="" />}
        <h1 className="question">{m.question ?? m.slug}</h1>
      </div>
      <div className="meta muted">
        <span className={`pill ${m.closed ? '' : 'ok'}`}>{m.closed ? 'Closed' : 'Live'}</span>
        {info.accepting_orders === false && !m.closed && <span className="pill">not accepting orders</span>}
        {negRisk && <span className="pill" title="Outcomes share one neg-risk collateral pool">neg-risk</span>}
        <span>ends {shortDate(m.end_date)}</span>
        {m.category && <span>{m.category}</span>}
        {m.slug && (
          <a href={`https://polymarket.com/market/${m.slug}`} target="_blank" rel="noreferrer">polymarket.com ↗</a>
        )}
      </div>

      {info.description && (
        <details className="desc">
          <summary>Resolution rules</summary>
          <p>{info.description}</p>
          {info.resolution_source && (
            <p className="muted">
              Source:{' '}
              {/^https?:\/\//.test(info.resolution_source) ? (
                <a href={info.resolution_source} target="_blank" rel="noreferrer">{info.resolution_source}</a>
              ) : (
                info.resolution_source
              )}
            </p>
          )}
        </details>
      )}

      <div className="tiles">
        {m.outcomes.map((o, i) => (
          <Tile key={o} label={o} value={cents(m.outcome_prices[i])} sub={i === 0 && moves ? moves : undefined} />
        ))}
        <Tile label="Volume" value={usd(m.volume, { compact: true })} />
        <Tile label="Liquidity" value={usd(m.liquidity, { compact: true })} />
        <Tile label="Spread" value={cents(m.spread)} sub={`bid ${cents(m.best_bid)} · ask ${cents(m.best_ask)}`} />
      </div>

      <div className="section-head">
        <h2>{chart === 'price' ? 'Price, last 30 days' : 'Spread, last 30 days · hourly mean'}</h2>
        <Segmented<Chart>
          value={chart}
          onChange={setChart}
          options={[
            { value: 'price', label: 'Price' },
            { value: 'spread', label: 'Spread' },
          ]}
        />
      </div>
      <div className="panel">
        {chart === 'price' ? (
          <LineChart series={toSeries(data.history)} formatValue={(v) => cents(v)} yDomain={[0, 1]} />
        ) : (
          <LineChart
            series={toSeries(data.spread_history)}
            formatValue={(v) => cents(v)}
            yDomain={[0, Math.max(0.01, ...data.spread_history.map((p) => p.price * 1.2))]}
          />
        )}
      </div>

      <Quotes data={data} outcomeOf={outcomeOf} now={now} />

      <div className="two-col">
        {m.token_ids.slice(0, 2).map((token) => {
          const bids = data.book.filter((l) => l.asset_id === token && l.side === 'bid')
          const asks = data.book.filter((l) => l.asset_id === token && l.side === 'ask')
          const holders = data.holders.filter((h) => h.token === token)
          const depth = (ls: typeof bids) => usd(ls.reduce((s, l) => s + l.size * l.price, 0), { compact: true })
          return (
            <section key={token}>
              <h2>Order book · {outcomeOf(token)}</h2>
              {bids.length + asks.length === 0 ? (
                <div className="empty">No book snapshot yet.</div>
              ) : (
                <>
                  <div className="scroll">
                    <table>
                      <thead>
                        <tr>
                          <th className="num">Bid size</th>
                          <th className="num">Bid</th>
                          <th className="num">Ask</th>
                          <th className="num">Ask size</th>
                        </tr>
                      </thead>
                      <tbody>
                        {Array.from({ length: Math.max(bids.length, asks.length) }, (_, i) => (
                          <tr key={i}>
                            <td className="num">{bids[i] ? shares(bids[i].size) : ''}</td>
                            <td className="num pos">{bids[i] ? cents(bids[i].price) : ''}</td>
                            <td className="num neg">{asks[i] ? cents(asks[i].price) : ''}</td>
                            <td className="num">{asks[i] ? shares(asks[i].size) : ''}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                  <div className="caption muted">Top {Math.max(bids.length, asks.length)} levels: {depth(bids)} bid · {depth(asks)} ask</div>
                </>
              )}

              <h2>Top holders · {outcomeOf(token)}</h2>
              {holders.length === 0 ? (
                <div className="empty">No holders stored yet.</div>
              ) : (
                <div className="scroll">
                  <table>
                    <tbody>
                      {holders.map((h) => (
                        <tr key={h.proxy_wallet}>
                          <td><Link to={`/traders/${h.proxy_wallet}`}>{traderName(h)}</Link></td>
                          <td className="num">{shares(h.amount)} shares</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </section>
          )
        })}
      </div>

      <h2>Recent trades <Link className="more" to={`/trades?market=${m.condition_id}`}>all trades →</Link></h2>
      <TradesTable trades={data.trades} showMarket={false} now={now} />

      <LiveFeed data={data} outcomeOf={outcomeOf} now={now} />

      <h2>Contract</h2>
      <dl className="kv panel">
        <Row k="Market id">{m.id}</Row>
        <Row k="Condition id">{m.condition_id}</Row>
        {m.token_ids.map((t) => (
          <Row key={t} k={`Token · ${outcomeOf(t)}`}>{t}</Row>
        ))}
        <Row k="Starts">{shortDate(info.start_date)}</Row>
        <Row k="Ends">{shortDate(m.end_date)}</Row>
        <Row k="Tick size">{info.tick_size ?? '—'}</Row>
        <Row k="Min order">{info.min_order_size != null ? `${info.min_order_size} shares` : '—'}</Row>
        <Row k="Scraped">{ago(m.scraped_at, now)}</Row>
      </dl>
    </>
  )
}

function Row({ k, children }: { k: string; children: ReactNode }) {
  return (
    <>
      <dt>{k}</dt>
      <dd className="mono">{children}</dd>
    </>
  )
}

type Sub = { data: MarketDetail; outcomeOf: (token: string) => string; now: number }

/** The CLOB's own quote endpoints and the newest /books header, per outcome. */
function Quotes({ data, outcomeOf, now }: Sub) {
  const tokens = data.market.token_ids
  if (data.quotes.length === 0 && data.books.length === 0) return null
  const quote = (token: string, kind: string, side?: string) =>
    data.quotes.find((q) => q.asset_id === token && q.kind === kind && (side === undefined || q.side === side))
  const newest = data.quotes.reduce<string | null>((a, q) => (a && a > q.captured_at ? a : q.captured_at), null)

  return (
    <>
      <h2>CLOB quotes {newest && <span className="more muted">as of {ago(newest, now)}</span>}</h2>
      <div className="scroll">
        <table>
          <thead>
            <tr>
              <th>Outcome</th>
              <th className="num">Midpoint</th>
              <th className="num" title="/price side=BUY">Buy</th>
              <th className="num" title="/price side=SELL">Sell</th>
              <th className="num">Spread</th>
              <th className="num">Last trade</th>
              <th className="num">Tick</th>
              <th className="num">Min order</th>
              <th className="num">Book as of</th>
            </tr>
          </thead>
          <tbody>
            {tokens.map((t) => {
              const book = data.books.find((b) => b.asset_id === t)
              return (
                <tr key={t}>
                  <td>{outcomeOf(t)}</td>
                  <td className="num">{cents(quote(t, 'midpoint')?.value)}</td>
                  <td className="num">{cents(quote(t, 'price', 'BUY')?.value)}</td>
                  <td className="num">{cents(quote(t, 'price', 'SELL')?.value)}</td>
                  <td className="num">{cents(quote(t, 'spread')?.value)}</td>
                  <td className="num">{cents(book?.last_trade_price)}</td>
                  <td className="num">{book?.tick_size ?? '—'}</td>
                  <td className="num">{book?.min_order_size ?? '—'}</td>
                  <td className="num muted">{book ? ago(book.ts, now) : '—'}</td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    </>
  )
}

/** Websocket market channel; only tokens listed in ASSET_IDS have any. */
function LiveFeed({ data, outcomeOf, now }: Sub) {
  const counts = { fills: data.ws_trades.length, changes: data.price_changes.length, ticks: data.tick_changes.length }
  const first = (Object.keys(counts) as Feed[]).find((k) => counts[k] > 0)
  const [picked, setFeed] = useState<Feed | null>(null)
  if (!first) return null
  const feed = picked ?? first

  return (
    <>
      <div className="section-head">
        <h2>Live feed · websocket</h2>
        <Segmented<Feed>
          value={feed}
          onChange={setFeed}
          options={[
            { value: 'fills', label: `Fills ${counts.fills}` },
            { value: 'changes', label: `Book changes ${counts.changes}` },
            { value: 'ticks', label: `Tick size ${counts.ticks}` },
          ]}
        />
      </div>
      <div className="scroll">
        {feed === 'fills' && (
          <table>
            <thead>
              <tr>
                <th>When</th>
                <th>Outcome</th>
                <th>Side</th>
                <th className="num">Price</th>
                <th className="num">Shares</th>
                <th className="num">Fee</th>
              </tr>
            </thead>
            <tbody>
              {data.ws_trades.map((t, i) => (
                <tr key={i}>
                  <td><When ts={t.ts} now={now} tx={t.transaction_hash} /></td>
                  <td>{outcomeOf(t.asset_id)}</td>
                  <td><span className={`badge ${t.side === 'BUY' ? 'pos' : 'neg'}`}>{t.side}</span></td>
                  <td className="num">{cents(t.price)}</td>
                  <td className="num">{shares(t.size)}</td>
                  <td className="num">{t.fee_rate_bps != null ? `${t.fee_rate_bps} bps` : '—'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        {feed === 'changes' && (
          <table>
            <thead>
              <tr>
                <th>When</th>
                <th>Outcome</th>
                <th>Side</th>
                <th className="num">Price</th>
                <th className="num">New size</th>
                <th className="num">Best ask</th>
              </tr>
            </thead>
            <tbody>
              {data.price_changes.map((c, i) => (
                <tr key={i}>
                  <td><When ts={c.ts} now={now} /></td>
                  <td>{outcomeOf(c.asset_id)}</td>
                  <td><span className={`badge ${c.side === 'BUY' ? 'pos' : 'neg'}`}>{c.side}</span></td>
                  <td className="num">{cents(c.price)}</td>
                  <td className="num">{shares(c.size)}</td>
                  <td className="num">{cents(c.best_ask)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        {feed === 'ticks' && (
          <table>
            <thead>
              <tr>
                <th>When</th>
                <th>Outcome</th>
                <th className="num">Old tick</th>
                <th className="num">New tick</th>
              </tr>
            </thead>
            <tbody>
              {data.tick_changes.map((c, i) => (
                <tr key={i}>
                  <td><When ts={c.ts} now={now} /></td>
                  <td>{outcomeOf(c.asset_id)}</td>
                  <td className="num">{c.old_tick_size ?? '—'}</td>
                  <td className="num">{c.new_tick_size}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </>
  )
}
