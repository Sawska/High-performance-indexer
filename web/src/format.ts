const compact = new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 1 })
const whole = new Intl.NumberFormat()

export const num = (n: number | null | undefined) => (n == null ? '—' : whole.format(Math.round(n)))

export function usd(n: number | null | undefined, opts?: { compact?: boolean; signed?: boolean }): string {
  if (n == null) return '—'
  const sign = n < 0 ? '−' : opts?.signed && n > 0 ? '+' : ''
  const abs = Math.abs(n)
  const body = opts?.compact && abs >= 10_000 ? compact.format(abs) : abs.toLocaleString(undefined, { maximumFractionDigits: abs < 100 ? 2 : 0 })
  return `${sign}$${body}`
}

export const cents = (p: number | null | undefined) => (p == null ? '—' : `${(p * 100).toFixed(p < 0.01 || p > 0.99 ? 1 : 0)}¢`)

/** A price move in price units, as signed cents. */
export function centsDelta(d: number | null | undefined): string {
  if (d == null) return '—'
  const c = d * 100
  if (Math.abs(c) < 0.05) return '0¢'
  return `${c > 0 ? '+' : '−'}${Math.abs(c).toFixed(Math.abs(c) < 1 ? 1 : 0)}¢`
}

export const txUrl = (hash: string) => `https://polygonscan.com/tx/${hash}`

export const shares = (n: number | null | undefined) =>
  n == null ? '—' : n.toLocaleString(undefined, { maximumFractionDigits: 0 })

export const pct = (p: number | null | undefined) => (p == null ? '—' : `${p.toFixed(1)}%`)

export function ago(iso: string | null, now = Date.now()): string {
  if (!iso) return '—'
  return duration(now - Date.parse(iso)) + ' ago'
}

export function duration(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000))
  if (s < 60) return `${s}s`
  const m = Math.floor(s / 60)
  if (m < 60) return `${m}m ${s % 60}s`
  const h = Math.floor(m / 60)
  if (h < 48) return `${h}h ${m % 60}m`
  return `${Math.floor(h / 24)}d`
}

export function shortDate(iso: string | null): string {
  if (!iso) return '—'
  const d = new Date(iso)
  // data-api fills unknown dates with the epoch.
  if (d.getTime() === 0) return '—'
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
}

export const shortWallet = (w: string) => `${w.slice(0, 6)}…${w.slice(-4)}`

export const traderName = (t: { name: string | null; pseudonym: string | null; proxy_wallet: string }) =>
  t.name || t.pseudonym || shortWallet(t.proxy_wallet)
