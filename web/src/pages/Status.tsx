import { useState } from 'react'
import { Tile } from '../components/ui'
import { ago, duration } from '../format'
import { useApi } from '../useApi'
import { useNow } from '../useNow'

type Stage = {
  name: string
  done: number
  total: number
  errors: number
  started_at: string
  finished_at: string | null
}

type Live = {
  process_started_at: string
  cycle: number
  cycles_completed: number
  cycle_started_at: string | null
  last_cycle_ms: number | null
  last_cycle_finished_at: string | null
  markets: number
  live_markets: number
  tokens: number
  wallets_discovered: number
  wallets_this_cycle: number
  stages: Stage[]
  errors_total: number
  recent_errors: { at: string; msg: string }[]
}

type StatusData = {
  now: string
  live: Live
  tables: { name: string; rows: number }[]
}

const POLL_MS = 2000

const fmt = (n: number) => n.toLocaleString()

export function Status() {
  const { data: status, error } = useApi<StatusData>('/api/status', POLL_MS)
  const now = useNow()

  const [baseline, setBaseline] = useState<Map<string, number> | null>(null)
  if (status && !baseline) setBaseline(new Map(status.tables.map((t) => [t.name, t.rows])))

  return (
    <>
      <div className="page-head">
        <h1>Indexer</h1>
        {status && <span className="muted">up {duration(now - Date.parse(status.live.process_started_at))}</span>}
        {error && <span className="pill bad">unreachable</span>}
      </div>
      {error && <div className="error-box">{error}</div>}
      {status && <Body status={status} now={now} baseline={baseline ?? new Map()} />}
    </>
  )
}

function Body({ status, now, baseline }: { status: StatusData; now: number; baseline: Map<string, number> }) {
  const l = status.live
  const running = l.cycle_started_at !== null &&
    (l.last_cycle_finished_at === null || Date.parse(l.cycle_started_at) > Date.parse(l.last_cycle_finished_at))
  const current = running ? l.stages.at(-1) : undefined

  return (
    <>
      <div className="tiles">
        <Tile label="Cycle" value={`#${l.cycle}`} sub={running ? `running · ${current?.name ?? 'starting'}` : 'idle'} />
        <Tile label="Cycles completed" value={fmt(l.cycles_completed)} sub={`last ${ago(l.last_cycle_finished_at, now)}`} />
        <Tile label="Last cycle took" value={l.last_cycle_ms === null ? '—' : duration(l.last_cycle_ms)} />
        <Tile label="Markets fetched" value={fmt(l.markets)} sub={`${fmt(l.live_markets)} live`} />
        <Tile label="Tokens" value={fmt(l.tokens)} />
        <Tile label="Wallets" value={fmt(l.wallets_discovered)} sub={`${fmt(l.wallets_this_cycle)} this cycle`} />
        <Tile label="Errors" value={fmt(l.errors_total)} />
      </div>

      <h2>Stages, cycle #{l.cycle}</h2>
      <div className="scroll">
        <table>
          <thead>
            <tr>
              <th>Stage</th>
              <th>Progress</th>
              <th className="num">Done</th>
              <th className="num">Total</th>
              <th className="num">Errors</th>
              <th className="num">Time</th>
            </tr>
          </thead>
          <tbody>
            {l.stages.length === 0 && (
              <tr>
                <td colSpan={6} className="muted">No stage started yet.</td>
              </tr>
            )}
            {l.stages.map((s) => {
              const end = s.finished_at ? Date.parse(s.finished_at) : now
              const pct = s.total > 0 ? Math.min(100, (s.done / s.total) * 100) : s.finished_at ? 100 : 0
              return (
                <tr key={s.name}>
                  <td>{s.name}{!s.finished_at && running && <span className="muted"> · running</span>}</td>
                  <td>
                    <div className="progress"><span style={{ width: `${pct}%` }} /></div>
                  </td>
                  <td className="num">{fmt(s.done)}</td>
                  <td className="num">{s.total > 0 ? fmt(s.total) : '—'}</td>
                  <td className="num">{s.errors > 0 ? fmt(s.errors) : ''}</td>
                  <td className="num">{duration(end - Date.parse(s.started_at))}</td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>

      <h2>Rows per table</h2>
      <div className="scroll">
        <table>
          <thead>
            <tr>
              <th>Table</th>
              <th className="num">Rows (≈)</th>
              <th className="num">Since page opened</th>
            </tr>
          </thead>
          <tbody>
            {status.tables.map((t) => {
              const delta = t.rows - (baseline.get(t.name) ?? t.rows)
              return (
                <tr key={t.name}>
                  <td>{t.name}</td>
                  {}
                  <td key={t.rows} className="num flash">{fmt(t.rows)}</td>
                  <td className="num delta">{delta !== 0 ? `${delta > 0 ? '+' : ''}${fmt(delta)}` : ''}</td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>

      {l.recent_errors.length > 0 && (
        <>
          <h2>Recent errors</h2>
          <div className="errors">
            {[...l.recent_errors].reverse().map((e, i) => (
              <div key={i}>
                <time>{new Date(e.at).toLocaleTimeString()}</time>
                {e.msg}
              </div>
            ))}
          </div>
        </>
      )}
    </>
  )
}
