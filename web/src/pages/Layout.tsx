import { NavLink, Navigate, Outlet, useLocation } from 'react-router'
import { useAuth } from '../useAuth'

const NAV = [
  { to: '/', label: 'Overview', end: true },
  { to: '/markets', label: 'Markets' },
  { to: '/traders', label: 'Traders' },
  { to: '/trades', label: 'Trades' },
  { to: '/activity', label: 'Activity' },
  { to: '/indexer', label: 'Indexer' },
]

export function Layout() {
  const { user, logout } = useAuth()
  const location = useLocation()

  if (user === undefined) return <div className="empty">Loading…</div>
  if (user === null) return <Navigate to="/login" replace state={{ from: location.pathname + location.search }} />

  return (
    <>
      <nav className="topbar">
        <div className="topbar-inner">
          <span className="brand">Polymarket Indexer</span>
          <div className="links">
            {NAV.map((n) => (
              <NavLink key={n.to} to={n.to} end={n.end}>
                {n.label}
              </NavLink>
            ))}
          </div>
          <div className="account">
            <span className="muted">{user.email}</span>
            <button onClick={() => logout()}>Sign out</button>
          </div>
        </div>
      </nav>
      <main>
        <Outlet />
      </main>
    </>
  )
}
