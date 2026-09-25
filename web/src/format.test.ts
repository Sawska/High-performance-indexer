import { describe, expect, it } from 'vitest'
import { ago, cents, centsDelta, duration, num, pct, shares, shortDate, shortWallet, traderName, txUrl, usd } from './format'

const MINUS = '−'
const SEC = 1000
const MIN = 60 * SEC
const HOUR = 60 * MIN

it('runs under the locale and time zone pinned in vite.config.ts', () => {
  expect(Intl.NumberFormat().resolvedOptions().locale).toBe('en-US')
  expect(Intl.DateTimeFormat().resolvedOptions().timeZone).toBe('UTC')
})

describe('num', () => {
  it('shows a dash for missing values', () => {
    expect(num(null)).toBe('—')
    expect(num(undefined)).toBe('—')
  })

  it('rounds to a whole number with grouping', () => {
    expect(num(0)).toBe('0')
    expect(num(1234.6)).toBe('1,235')
    expect(num(1_000_000)).toBe('1,000,000')
  })
})

describe('usd', () => {
  it('shows a dash for missing values', () => {
    expect(usd(null)).toBe('—')
    expect(usd(undefined)).toBe('—')
    expect(usd(null, { compact: true, signed: true })).toBe('—')
  })

  it('keeps cents below $100 and drops them from $100', () => {
    expect(usd(0)).toBe('$0')
    expect(usd(42.126)).toBe('$42.13')
    expect(usd(99.5)).toBe('$99.5')
    expect(usd(1234.56)).toBe('$1,235')
  })

  it('prefixes negatives with a U+2212 minus sign, never a hyphen', () => {
    expect(usd(-50)).toBe(`${MINUS}$50`)
    expect(usd(-1234.56)).toBe(`${MINUS}$1,235`)
    expect(usd(-50)).not.toContain('-')
  })

  it('adds + only when signed and positive', () => {
    expect(usd(50)).toBe('$50')
    expect(usd(50, { signed: true })).toBe('+$50')
    expect(usd(0, { signed: true })).toBe('$0')
    expect(usd(-50, { signed: true })).toBe(`${MINUS}$50`)
  })

  it('compacts only from $10k', () => {
    expect(usd(9_999, { compact: true })).toBe('$9,999')
    expect(usd(10_000, { compact: true })).toBe('$10K')
    expect(usd(12_500, { compact: true })).toBe('$12.5K')
    expect(usd(2_345_678, { compact: true })).toBe('$2.3M')
    expect(usd(12_500)).toBe('$12,500')
  })

  it('combines compact and signed', () => {
    expect(usd(1_500_000, { compact: true, signed: true })).toBe('+$1.5M')
    expect(usd(-12_500, { compact: true, signed: true })).toBe(`${MINUS}$12.5K`)
    expect(usd(-12_500, { compact: true })).toBe(`${MINUS}$12.5K`)
  })
})

describe('cents', () => {
  it('shows a dash for missing values', () => {
    expect(cents(null)).toBe('—')
    expect(cents(undefined)).toBe('—')
  })

  it('shows whole cents between 1¢ and 99¢', () => {
    expect(cents(0.62)).toBe('62¢')
    expect(cents(0.01)).toBe('1¢')
    expect(cents(0.99)).toBe('99¢')
    expect(cents(0.5)).toBe('50¢')
  })

  it('shows one decimal below 1¢ and above 99¢', () => {
    expect(cents(0.005)).toBe('0.5¢')
    expect(cents(0)).toBe('0.0¢')
    expect(cents(0.995)).toBe('99.5¢')
    expect(cents(1)).toBe('100.0¢')
  })
})

describe('centsDelta', () => {
  it('shows a dash for missing values', () => {
    expect(centsDelta(null)).toBe('—')
    expect(centsDelta(undefined)).toBe('—')
  })

  it('signs moves with + and U+2212', () => {
    expect(centsDelta(0.034)).toBe('+3¢')
    expect(centsDelta(0.036)).toBe('+4¢')
    expect(centsDelta(-0.05)).toBe(`${MINUS}5¢`)
    expect(centsDelta(-0.12)).toBe(`${MINUS}12¢`)
  })

  it('keeps one decimal for moves under a cent', () => {
    expect(centsDelta(0.004)).toBe('+0.4¢')
    expect(centsDelta(-0.0072)).toBe(`${MINUS}0.7¢`)
  })

  it('shows 0¢ for moves that would round to nothing', () => {
    expect(centsDelta(0)).toBe('0¢')
    expect(centsDelta(0.0004)).toBe('0¢')
    expect(centsDelta(-0.0003)).toBe('0¢')
  })
})

describe('duration', () => {
  it('shows seconds under a minute', () => {
    expect(duration(0)).toBe('0s')
    expect(duration(59 * SEC)).toBe('59s')
    expect(duration(400)).toBe('0s')
  })

  it('clamps negative spans to 0s', () => {
    expect(duration(-5 * MIN)).toBe('0s')
  })

  it('shows minutes and seconds under an hour', () => {
    expect(duration(59_600)).toBe('1m 0s')
    expect(duration(61 * SEC)).toBe('1m 1s')
    expect(duration(59 * MIN + 59 * SEC)).toBe('59m 59s')
  })

  it('shows hours and minutes under two days', () => {
    expect(duration(HOUR)).toBe('1h 0m')
    expect(duration(5 * HOUR + 7 * MIN + 30 * SEC)).toBe('5h 7m')
    expect(duration(47 * HOUR + 59 * MIN)).toBe('47h 59m')
  })

  it('shows whole days from two days', () => {
    expect(duration(48 * HOUR)).toBe('2d')
    expect(duration(10 * 24 * HOUR + 5 * HOUR)).toBe('10d')
  })
})

describe('ago', () => {
  const now = Date.parse('2026-09-25T12:00:00Z')

  it('shows a dash without a timestamp', () => {
    expect(ago(null, now)).toBe('—')
    expect(ago('', now)).toBe('—')
  })

  it('formats the span to now', () => {
    expect(ago('2026-09-25T11:59:30Z', now)).toBe('30s ago')
    expect(ago('2026-09-25T11:58:30Z', now)).toBe('1m 30s ago')
    expect(ago('2026-09-25T09:15:00Z', now)).toBe('2h 45m ago')
    expect(ago('2026-09-20T12:00:00Z', now)).toBe('5d ago')
  })

  it('treats future timestamps as just now', () => {
    expect(ago('2026-09-25T12:05:00Z', now)).toBe('0s ago')
  })
})

describe('shortDate', () => {
  it('shows a dash for missing dates', () => {
    expect(shortDate(null)).toBe('—')
    expect(shortDate('')).toBe('—')
  })

  it('shows a dash for the epoch placeholder data-api uses for unknown dates', () => {
    expect(shortDate('1970-01-01')).toBe('—')
    expect(shortDate('1970-01-01T00:00:00Z')).toBe('—')
  })

  it('returns unparseable input unchanged', () => {
    expect(shortDate('not a date')).toBe('not a date')
  })

  it('formats as month, day, year', () => {
    expect(shortDate('2026-03-15T12:00:00Z')).toBe('Mar 15, 2026')
    expect(shortDate('2025-12-31')).toBe('Dec 31, 2025')
  })
})

describe('shortWallet', () => {
  it('keeps the first 6 and last 4 characters', () => {
    expect(shortWallet('0xabcdef0123456789abcdef0123456789abcd1234')).toBe('0xabcd…1234')
  })
})

describe('traderName', () => {
  const wallet = '0xabcdef0123456789abcdef0123456789abcd1234'

  it('prefers the name', () => {
    expect(traderName({ name: 'alice', pseudonym: 'Brave-Otter', proxy_wallet: wallet })).toBe('alice')
  })

  it('falls back to the pseudonym', () => {
    expect(traderName({ name: null, pseudonym: 'Brave-Otter', proxy_wallet: wallet })).toBe('Brave-Otter')
    expect(traderName({ name: '', pseudonym: 'Brave-Otter', proxy_wallet: wallet })).toBe('Brave-Otter')
  })

  it('falls back to the short wallet', () => {
    expect(traderName({ name: null, pseudonym: null, proxy_wallet: wallet })).toBe('0xabcd…1234')
    expect(traderName({ name: '', pseudonym: '', proxy_wallet: wallet })).toBe('0xabcd…1234')
  })
})

describe('txUrl', () => {
  it('links to polygonscan', () => {
    expect(txUrl('0xdeadbeef')).toBe('https://polygonscan.com/tx/0xdeadbeef')
  })
})

describe('shares', () => {
  it('shows a dash for missing values', () => {
    expect(shares(null)).toBe('—')
    expect(shares(undefined)).toBe('—')
  })

  it('rounds to whole shares with grouping', () => {
    expect(shares(12)).toBe('12')
    expect(shares(12.4)).toBe('12')
    expect(shares(1234.6)).toBe('1,235')
  })
})

describe('pct', () => {
  it('shows a dash for missing values', () => {
    expect(pct(null)).toBe('—')
    expect(pct(undefined)).toBe('—')
  })

  it('shows one decimal', () => {
    expect(pct(24)).toBe('24.0%')
    expect(pct(12.345)).toBe('12.3%')
    expect(pct(-5)).toBe('-5.0%')
  })
})
