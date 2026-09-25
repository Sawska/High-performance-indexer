export class ApiError extends Error {
  status: number
  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

export const UNAUTHORIZED_EVENT = 'api:unauthorized'

export async function api<T>(path: string, init?: { method?: string; body?: unknown }): Promise<T> {
  const res = await fetch(path, {
    method: init?.method ?? 'GET',
    credentials: 'same-origin',
    headers: init?.body !== undefined ? { 'Content-Type': 'application/json' } : undefined,
    body: init?.body !== undefined ? JSON.stringify(init.body) : undefined,
  })
  if (!res.ok) {
    let message = `${res.status} ${res.statusText}`
    try {
      const body = (await res.json()) as { error?: string }
      if (body.error) message = body.error
    } catch {
    }
    if (res.status === 401 && !path.startsWith('/api/auth/')) {
      window.dispatchEvent(new Event(UNAUTHORIZED_EVENT))
    }
    throw new ApiError(res.status, message)
  }
  const text = await res.text()
  return (text ? JSON.parse(text) : undefined) as T
}

export function qs(params: Record<string, string | number | undefined>): string {
  const p = new URLSearchParams()
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== '') p.set(k, String(v))
  }
  const s = p.toString()
  return s ? `?${s}` : ''
}

export type User = { id: number; email: string }

export type Market = {
  id: string
  question: string | null
  slug: string | null
  condition_id: string
  category: string | null
  closed: boolean
  end_date: string | null
  volume: number | null
  liquidity: number | null
  best_bid: number | null
  best_ask: number | null
  last_trade_price: number | null
  spread: number | null
  outcomes: string[]
  outcome_prices: number[]
  token_ids: string[]
  scraped_at: string
  icon: string | null
  one_day_change: number | null
}

export type Trade = {
  ts: string
  proxy_wallet: string
  name: string | null
  pseudonym: string | null
  condition_id: string
  title: string | null
  side: string
  outcome: string | null
  size: number
  price: number
  transaction_hash: string
}

export type Page<T> = { total: number; rows: T[] }

export type Overview = {
  totals: {
    markets: number
    live_markets: number
    live_volume: number
    live_liquidity: number
    wallets: number
    trades_24h: number
    volume_24h: number
  }
  top_markets: Market[]
  recent_trades: Trade[]
}

export type PricePoint = { token: string; ts: string; price: number }

export type MarketDetail = {
  market: Market
  info: {
    description: string | null
    image: string | null
    resolution_source: string | null
    start_date: string | null
    active: boolean | null
    accepting_orders: boolean | null
    tick_size: number | null
    min_order_size: number | null
    one_week_change: number | null
    one_month_change: number | null
  }
  history: PricePoint[]
  spread_history: PricePoint[]
  quotes: { asset_id: string; kind: 'midpoint' | 'spread' | 'price'; side: string | null; value: number; captured_at: string }[]
  books: {
    asset_id: string
    ts: string
    tick_size: number | null
    min_order_size: number | null
    neg_risk: boolean | null
    last_trade_price: number | null
  }[]
  book: { asset_id: string; side: 'bid' | 'ask'; price: number; size: number }[]
  holders: { token: string; proxy_wallet: string; name: string | null; pseudonym: string | null; amount: number }[]
  trades: Trade[]
  ws_trades: {
    asset_id: string
    ts: string
    price: number
    size: number | null
    side: string
    fee_rate_bps: number | null
    transaction_hash: string | null
  }[]
  price_changes: { asset_id: string; ts: string; price: number; size: number; side: string; best_ask: number | null }[]
  tick_changes: { asset_id: string; ts: string; old_tick_size: number | null; new_tick_size: number }[]
}

export type Trader = {
  proxy_wallet: string
  name: string | null
  pseudonym: string | null
  profile_image: string | null
  pnl: number | null
  value: number | null
  volume: number
  trades: number
}

export type Activity = {
  ts: string
  activity_type: string
  condition_id: string
  title: string | null
  side: string | null
  outcome: string | null
  size: number
  usdc_size: number
  price: number
  transaction_hash: string
}

export type ActivityFeedRow = Activity & { proxy_wallet: string; name: string | null; pseudonym: string | null }

export type TraderDetail = {
  trader: Trader
  profile: { bio: string | null; verified: boolean | null }
  pnl: { ts: string; pnl: number }[]
  values: { ts: string; value: number }[]
  positions: {
    asset: string
    condition_id: string
    has_market: boolean
    question: string | null
    outcome: string | null
    size: number
    avg_price: number
    cur_price: number
    initial_value: number
    current_value: number
    total_bought: number
    cash_pnl: number
    percent_pnl: number
    realized_pnl: number
    redeemable: boolean
    mergeable: boolean
    end_date: string | null
  }[]
  activity: Activity[]
}
