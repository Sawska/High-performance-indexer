import { Link, useSearchParams } from 'react-router'
import {
  qs,
  type ChainCollateralTransfer,
  type ChainCondition,
  type ChainFill,
  type ChainPosition,
  type ChainRedemption,
  type ChainStatus,
  type ChainTokenTransfer,
} from '../api'
import { Chip, Loading, OffsetPager, Segmented, Tile, When } from '../components/ui'
import { ago, cents, duration, shares, shortWallet, txUrl, usd } from '../format'
import { useApi } from '../useApi'
import { useNow } from '../useNow'

const LIMIT = 100
const STATUS_POLL_MS = 5000

/** Polygon block time, for turning a block lag into something human. */
const BLOCK_MS = 2100

type Feed = 'fills' | 'positions' | 'redemptions' | 'transfers' | 'collateral' | 'conditions'

/** Builds a link that narrows the current feed to one address, token or condition. */
type FilterTo = (key: 'wallet' | 'token_id' | 'condition_id', value: string) => string

const FEEDS: { value: Feed; label: string }[] = [
  { value: 'fills', label: 'Fills' },
  { value: 'positions', label: 'Splits & merges' },
  { value: 'redemptions', label: 'Redemptions' },
  { value: 'transfers', label: 'Token transfers' },
  { value: 'collateral', label: 'Collateral' },
  { value: 'conditions', label: 'Conditions' },
]

/** Which filters each feed's endpoint actually honours. */
const ACCEPTS: Record<Feed, ('wallet' | 'token_id' | 'condition_id')[]> = {
  fills: ['wallet', 'token_id'],
  positions: ['wallet', 'condition_id'],
  redemptions: ['wallet', 'condition_id'],
  transfers: ['wallet', 'token_id'],
  collateral: ['wallet'],
  conditions: ['condition_id'],
}

const isFeed = (v: string): v is Feed => FEEDS.some((f) => f.value === v)

const shortId = (v: string) => (v.length > 14 ? `${v.slice(0, 8)}…${v.slice(-4)}` : v)

/** A log's position on the chain: relative time, linking out to the transaction. */
function Ts({ row, now }: { row: { block_time: string | null; block_number: number; transaction_hash: string }; now: number }) {
  if (row.block_time) return <When ts={row.block_time} now={now} tx={row.transaction_hash} />
  // block_time is null when the RPC would not give up the header.
  return (
    <a className="muted" href={txUrl(row.transaction_hash)} target="_blank" rel="noreferrer" title="view transaction">
      #{row.block_number.toLocaleString()}
    </a>
  )
}

export function Chain() {
  const [params, setParams] = useSearchParams()
  const raw = params.get('feed') ?? 'fills'
  const feed: Feed = isFeed(raw) ? raw : 'fills'
  const wallet = params.get('wallet') ?? ''
  const tokenId = params.get('token_id') ?? ''
  const conditionId = params.get('condition_id') ?? ''
  const resolved = params.get('resolved') ?? ''
  const offset = Number(params.get('offset') ?? 0)
  const now = useNow()

  const accepts = ACCEPTS[feed]
  const query = {
    wallet: accepts.includes('wallet') ? wallet : '',
    token_id: accepts.includes('token_id') ? tokenId : '',
    condition_id: accepts.includes('condition_id') ? conditionId : '',
    resolved: feed === 'conditions' ? resolved : '',
    limit: LIMIT,
    offset,
  }

  const { data: status } = useApi<ChainStatus>('/api/chain/status', STATUS_POLL_MS)
  const path = `/api/chain/${feed}${qs(query)}`

  const set = (key: string, value: string) => {
    const next = new URLSearchParams(params)
    if (value) next.set(key, value)
    else next.delete(key)
    if (key !== 'offset') next.delete('offset')
    setParams(next, { replace: true })
  }

  /** A link that narrows the current feed to one address, token or condition. */
  const filterTo = (key: 'wallet' | 'token_id' | 'condition_id', value: string) => {
    const next = new URLSearchParams(params)
    next.set(key, value)
    next.delete('offset')
    return `/chain?${next.toString()}`
  }

  return (
    <>
      <div className="page-head">
        <h1>On-chain</h1>
        {status && <Lag status={status} now={now} />}
      </div>

      {status && <Status status={status} now={now} />}

      <div className="filters">
        <Segmented value={feed} options={FEEDS} onChange={(v) => set('feed', v)} />
        {feed === 'conditions' && (
          <select value={resolved} onChange={(e) => set('resolved', e.target.value)}>
            <option value="">Open and resolved</option>
            <option value="true">Resolved</option>
            <option value="false">Open</option>
          </select>
        )}
        {wallet && accepts.includes('wallet') && (
          <Chip onClear={() => set('wallet', '')}>Address: {shortWallet(wallet)}</Chip>
        )}
        {tokenId && accepts.includes('token_id') && (
          <Chip onClear={() => set('token_id', '')}>Token: {shortId(tokenId)}</Chip>
        )}
        {conditionId && accepts.includes('condition_id') && (
          <Chip onClear={() => set('condition_id', '')}>Condition: {shortId(conditionId)}</Chip>
        )}
        <span className="muted">{offset === 0 ? 'newest first, refreshes every 15s' : 'older events'}</span>
      </div>

      {/* Keyed by feed: switching feed must not render one feed's rows through
          another's columns, which a shared hook would do while the new page
          loads. Changing a filter keeps the instance, so rows dim rather than
          disappear. */}
      <FeedPanel
        key={feed}
        feed={feed}
        path={path}
        offset={offset}
        now={now}
        filterTo={filterTo}
        onOffset={(o) => set('offset', String(o))}
      />
    </>
  )
}

function FeedPanel({
  feed,
  path,
  offset,
  now,
  filterTo,
  onOffset,
}: {
  feed: Feed
  path: string
  offset: number
  now: number
  filterTo: FilterTo
  onOffset: (offset: number) => void
}) {
  const { data, error, loading } = useApi<unknown[]>(path, offset === 0 ? 15_000 : undefined)

  if (!data) return <Loading error={error} loading={loading} />

  return (
    <>
      <div className={loading ? 'stale' : ''}>
        <Table feed={feed} rows={data} now={now} filterTo={filterTo} />
      </div>
      <OffsetPager offset={offset} limit={LIMIT} count={data.length} onChange={onOffset} />
    </>
  )
}

function Lag({ status, now }: { status: ChainStatus; now: number }) {
  if (!status.enabled) return <span className="pill">not indexing here</span>
  if (status.lag === null) return <span className="pill">starting…</span>
  // Anything inside the confirmation depth is as current as this design gets.
  const caughtUp = status.lag <= 100
  return (
    <span className={`pill ${caughtUp ? 'ok' : ''}`} title={`updated ${ago(status.updated_at, now)}`}>
      {caughtUp ? 'following head' : `${duration(status.lag * BLOCK_MS)} behind`}
    </span>
  )
}

function Status({ status, now }: { status: ChainStatus; now: number }) {
  const cursor = status.cursor ?? status.stored_cursor
  const total = status.tables.reduce((sum, t) => sum + t.rows, 0)

  return (
    <>
      {!status.enabled && (
        <div className="empty">
          This server is not running the on-chain indexer — set <code>CHAIN_ENABLED=true</code> to start it.
          The tables below are whatever was indexed previously.
        </div>
      )}
      <div className="tiles">
        <Tile label="Chain head" value={status.head?.toLocaleString() ?? '—'} sub="polygon" />
        <Tile
          label="Indexed to"
          value={cursor?.toLocaleString() ?? '—'}
          sub={status.lag === null ? undefined : `${status.lag.toLocaleString()} blocks behind`}
        />
        <Tile label="Rows stored" value={total.toLocaleString()} sub={`${status.tables.length} tables`} />
        <Tile
          label="Written this run"
          value={status.rows_written.toLocaleString()}
          sub={status.ranges_done > 0 ? `${status.ranges_done.toLocaleString()} ranges` : undefined}
        />
        <Tile label="Reorgs" value={status.reorgs.toLocaleString()} />
        <Tile label="Errors" value={status.errors.toLocaleString()} sub={status.last_error ?? undefined} />
      </div>
      {status.stored_cursor_at && (
        <p className="muted">Cursor last moved {ago(status.stored_cursor_at, now)}.</p>
      )}
    </>
  )
}

function Table({ feed, rows, now, filterTo }: { feed: Feed; rows: unknown[]; now: number; filterTo: FilterTo }) {
  if (rows.length === 0) return <div className="empty">Nothing indexed for this filter yet.</div>

  switch (feed) {
    case 'fills':
      return <Fills rows={rows as ChainFill[]} now={now} filterTo={filterTo} />
    case 'positions':
      return <Positions rows={rows as ChainPosition[]} now={now} filterTo={filterTo} />
    case 'redemptions':
      return <Redemptions rows={rows as ChainRedemption[]} now={now} filterTo={filterTo} />
    case 'transfers':
      return <Transfers rows={rows as ChainTokenTransfer[]} now={now} filterTo={filterTo} />
    case 'collateral':
      return <Collateral rows={rows as ChainCollateralTransfer[]} now={now} filterTo={filterTo} />
    case 'conditions':
      return <Conditions rows={rows as ChainCondition[]} now={now} filterTo={filterTo} />
  }
}

/** An address, as a link that narrows the feed to it. */
function Addr({ a, filterTo }: { a: string | null; filterTo: FilterTo }) {
  if (!a) return <span className="muted">—</span>
  return (
    <Link className="mono" to={filterTo('wallet', a)} title={a}>
      {shortWallet(a)}
    </Link>
  )
}

/** A market, when the catalogue join found one; otherwise the bare condition. */
function MarketCell({ conditionId, question, filterTo }: { conditionId: string | null; question: string | null; filterTo: FilterTo }) {
  if (conditionId && question) return <Link to={`/markets/${conditionId}`}>{question}</Link>
  if (conditionId) {
    return (
      <Link className="mono" to={filterTo('condition_id', conditionId)} title={conditionId}>
        {shortId(conditionId)}
      </Link>
    )
  }
  return <span className="muted">—</span>
}

function Fills({ rows, now, filterTo }: { rows: ChainFill[]; now: number; filterTo: FilterTo }) {
  return (
    <div className="scroll">
      <table>
        <thead>
          <tr>
            <th>When</th>
            <th>Market</th>
            <th>Side</th>
            <th>Maker</th>
            <th>Taker</th>
            <th className="num">Price</th>
            <th className="num">Shares</th>
            <th className="num">Value</th>
            <th className="num">Fee</th>
            <th>Exchange</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={`${r.transaction_hash}-${r.log_index}`}>
              <td><Ts row={r} now={now} /></td>
              {/* A fill names a market only once the catalogue has caught up with
                  the token; until then the token itself is the useful handle. */}
              <td className="wrap">
                {r.question && r.condition_id ? (
                  <Link to={`/markets/${r.condition_id}`}>{r.question}</Link>
                ) : r.token_id ? (
                  <Link className="mono" to={filterTo('token_id', r.token_id)} title={r.token_id}>
                    {shortId(r.token_id)}
                  </Link>
                ) : (
                  <span className="muted">—</span>
                )}
              </td>
              <td>{r.side ? <span className={`badge ${r.side === 'BUY' ? 'pos' : 'neg'}`}>{r.side}</span> : '—'}</td>
              <td><Addr a={r.maker} filterTo={filterTo} /></td>
              <td><Addr a={r.taker} filterTo={filterTo} /></td>
              <td className="num">{cents(r.price)}</td>
              <td className="num">{shares(r.size)}</td>
              <td className="num">{r.price != null && r.size != null ? usd(r.price * r.size) : '—'}</td>
              <td className="num">{r.fee ? usd(r.fee) : ''}</td>
              <td>
                <span className="badge">v{r.exchange_version}</span>
                {r.neg_risk && <span className="pill">neg-risk</span>}
                {r.event === 'orders_matched' && <span className="pill">matched</span>}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function Positions({ rows, now, filterTo }: { rows: ChainPosition[]; now: number; filterTo: FilterTo }) {
  return (
    <div className="scroll">
      <table>
        <thead>
          <tr>
            <th>When</th>
            <th>Action</th>
            <th>Holder</th>
            <th>Market</th>
            <th className="num">Outcomes</th>
            <th className="num">Collateral</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={`${r.transaction_hash}-${r.log_index}`}>
              <td><Ts row={r} now={now} /></td>
              <td>
                {/* A split locks collateral into outcome tokens; a merge unlocks it. */}
                <span className={`badge ${r.kind === 'split' ? 'pos' : 'neg'}`}>{r.kind.toUpperCase()}</span>
              </td>
              <td><Addr a={r.stakeholder} filterTo={filterTo} /></td>
              <td className="wrap"><MarketCell conditionId={r.condition_id} question={r.question} filterTo={filterTo} /></td>
              <td className="num">{r.partition_ids.length}</td>
              <td className="num">{usd(r.amount)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function Redemptions({ rows, now, filterTo }: { rows: ChainRedemption[]; now: number; filterTo: FilterTo }) {
  return (
    <div className="scroll">
      <table>
        <thead>
          <tr>
            <th>When</th>
            <th>Redeemer</th>
            <th>Market</th>
            <th className="num">Index sets</th>
            <th className="num">Payout</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={`${r.transaction_hash}-${r.log_index}`}>
              <td><Ts row={r} now={now} /></td>
              <td><Addr a={r.redeemer} filterTo={filterTo} /></td>
              <td className="wrap"><MarketCell conditionId={r.condition_id} question={r.question} filterTo={filterTo} /></td>
              <td className="num mono">{r.index_sets.join(', ') || '—'}</td>
              {/* A losing position redeems for nothing, which is worth showing as $0. */}
              <td className="num">{usd(r.payout)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function Transfers({ rows, now, filterTo }: { rows: ChainTokenTransfer[]; now: number; filterTo: FilterTo }) {
  return (
    <div className="scroll">
      <table>
        <thead>
          <tr>
            <th>When</th>
            <th>From</th>
            <th>To</th>
            <th>Token</th>
            <th className="num">Shares</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={`${r.transaction_hash}-${r.log_index}-${r.item_index}`}>
              <td><Ts row={r} now={now} /></td>
              <td><Addr a={r.from_address} filterTo={filterTo} /></td>
              <td><Addr a={r.to_address} filterTo={filterTo} /></td>
              <td>
                <Link className="mono" to={filterTo('token_id', r.token_id)} title={r.token_id}>
                  {shortId(r.token_id)}
                </Link>
              </td>
              <td className="num">{shares(r.amount)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function Collateral({ rows, now, filterTo }: { rows: ChainCollateralTransfer[]; now: number; filterTo: FilterTo }) {
  return (
    <div className="scroll">
      <table>
        <thead>
          <tr>
            <th>When</th>
            <th>From</th>
            <th>To</th>
            <th>Token</th>
            <th className="num">Amount</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={`${r.transaction_hash}-${r.log_index}`}>
              <td><Ts row={r} now={now} /></td>
              <td><Addr a={r.from_address} filterTo={filterTo} /></td>
              <td><Addr a={r.to_address} filterTo={filterTo} /></td>
              <td><span className="badge">{r.symbol ?? shortWallet(r.token)}</span></td>
              <td className="num">{usd(r.amount)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

/** The index of the winning outcome, or null while a condition is unresolved
 *  or the payouts are split. */
function winner(payouts: string[] | null): number | null {
  if (!payouts || payouts.length === 0) return null
  const nums = payouts.map(Number)
  const max = Math.max(...nums)
  if (max <= 0) return null
  return nums.filter((n) => n === max).length === 1 ? nums.indexOf(max) : null
}

function Conditions({ rows, now, filterTo }: { rows: ChainCondition[]; now: number; filterTo: FilterTo }) {
  return (
    <div className="scroll">
      <table>
        <thead>
          <tr>
            <th>Market</th>
            <th className="num">Outcomes</th>
            <th>Prepared</th>
            <th>Resolved</th>
            <th>Payouts</th>
            <th>Oracle</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => {
            const won = winner(r.payout_numerators)
            return (
              <tr key={r.condition_id}>
                <td className="wrap"><MarketCell conditionId={r.condition_id} question={r.question} filterTo={filterTo} /></td>
                <td className="num">{r.outcome_slot_count ?? '—'}</td>
                <td className="muted">{r.prepared_at ? ago(r.prepared_at, now) : '—'}</td>
                <td className="muted">{r.resolved_at ? ago(r.resolved_at, now) : <span className="pill">open</span>}</td>
                <td className="mono">
                  {r.payout_numerators ? r.payout_numerators.join(' / ') : '—'}
                  {won !== null && <span className="pill ok">outcome {won}</span>}
                </td>
                <td><Addr a={r.oracle} filterTo={filterTo} /></td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
