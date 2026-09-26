import type { ActivityFeedRow, Activity, Market, MarketDetail, Trade, TraderDetail } from '../api'

export const WALLET = '0xabcdef0123456789abcdef0123456789abcd1234'
export const CONDITION = '0xc0nd1710n'
export const TOKEN_YES = '71321045679252212594626385532706912750332728571942532289631379312455583992563'
export const TOKEN_NO = '52114319501245915516055106046884209969926127482827954674443846427813813222426'
export const TX = '0x5eed000000000000000000000000000000000000000000000000000000000001'

export function market(over: Partial<Market> = {}): Market {
  return {
    id: '512345',
    question: 'Will it rain in London tomorrow?',
    slug: 'will-it-rain-in-london-tomorrow',
    condition_id: CONDITION,
    category: 'Weather',
    closed: false,
    end_date: '2026-12-31T12:00:00Z',
    volume: 2_345_678,
    liquidity: 45_600,
    best_bid: 0.61,
    best_ask: 0.63,
    last_trade_price: 0.62,
    spread: 0.02,
    outcomes: ['Yes', 'No'],
    outcome_prices: [0.62, 0.38],
    token_ids: [TOKEN_YES, TOKEN_NO],
    scraped_at: '2026-09-25T11:59:00Z',
    icon: null,
    one_day_change: null,
    ...over,
  }
}

export function trade(over: Partial<Trade> = {}): Trade {
  return {
    ts: '2026-09-25T11:00:00Z',
    proxy_wallet: WALLET,
    name: 'alice',
    pseudonym: 'Brave-Otter',
    condition_id: CONDITION,
    title: 'Will it rain in London tomorrow?',
    side: 'BUY',
    outcome: 'Yes',
    size: 100,
    price: 0.62,
    transaction_hash: TX,
    ...over,
  }
}

export function marketDetail(over: Partial<MarketDetail> = {}): MarketDetail {
  return {
    market: market(),
    info: {
      description: 'Resolves Yes if the Met Office records rain.',
      image: null,
      resolution_source: 'https://www.metoffice.gov.uk',
      start_date: '2026-09-01T12:00:00Z',
      active: true,
      accepting_orders: true,
      tick_size: 0.01,
      min_order_size: 5,
      one_week_change: null,
      one_month_change: null,
    },
    history: [],
    spread_history: [],
    quotes: [],
    books: [],
    book: [],
    holders: [],
    trades: [],
    ws_trades: [],
    price_changes: [],
    tick_changes: [],
    ...over,
  }
}

type Position = TraderDetail['positions'][number]

export function position(over: Partial<Position> = {}): Position {
  return {
    asset: TOKEN_YES,
    condition_id: CONDITION,
    has_market: true,
    question: 'Will it rain in London tomorrow?',
    outcome: 'Yes',
    size: 200,
    avg_price: 0.5,
    cur_price: 0.62,
    initial_value: 100,
    current_value: 124,
    total_bought: 200,
    cash_pnl: 24,
    percent_pnl: 24,
    realized_pnl: 0,
    redeemable: false,
    mergeable: false,
    end_date: '2026-12-31T12:00:00Z',
    ...over,
  }
}

export function activity(over: Partial<Activity> = {}): Activity {
  return {
    ts: '2026-09-25T11:00:00Z',
    activity_type: 'TRADE',
    condition_id: CONDITION,
    title: 'Will it rain in London tomorrow?',
    side: 'BUY',
    outcome: 'Yes',
    size: 100,
    usdc_size: 62,
    price: 0.62,
    transaction_hash: TX,
    ...over,
  }
}

export function activityRow(over: Partial<ActivityFeedRow> = {}): ActivityFeedRow {
  return { ...activity(), proxy_wallet: WALLET, name: 'alice', pseudonym: null, ...over }
}

export function traderDetail(over: Partial<TraderDetail> = {}): TraderDetail {
  return {
    trader: {
      proxy_wallet: WALLET,
      name: 'alice',
      pseudonym: 'Brave-Otter',
      profile_image: null,
      pnl: 12_500,
      value: 3_400,
      volume: 98_000,
      trades: 42,
    },
    profile: { bio: 'Weather markets only.', verified: true },
    pnl: [],
    values: [],
    positions: [],
    activity: [],
    ...over,
  }
}
