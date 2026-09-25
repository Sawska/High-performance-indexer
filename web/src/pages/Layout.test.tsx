import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes, useLocation } from 'react-router'
import { describe, expect, it, vi } from 'vitest'
import { AuthContext, type Auth } from '../useAuth'
import { Layout } from './Layout'

function LoginProbe() {
  const location = useLocation()
  return <div>login page, from {(location.state as { from?: string } | null)?.from}</div>
}

function renderLayout(auth: Partial<Auth>, url = '/markets?status=all') {
  const value: Auth = { user: undefined, login: vi.fn(), register: vi.fn(), logout: vi.fn(async () => {}), ...auth }
  render(
    <AuthContext.Provider value={value}>
      <MemoryRouter initialEntries={[url]}>
        <Routes>
          <Route path="/login" element={<LoginProbe />} />
          <Route element={<Layout />}>
            <Route index element={<div>overview page</div>} />
            <Route path="markets" element={<div>markets page</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </AuthContext.Provider>,
  )
  return value
}

describe('Layout', () => {
  it('waits while the session is being checked', () => {
    renderLayout({ user: undefined })
    expect(screen.getByText('Loading…')).toBeInTheDocument()
    expect(screen.queryByText('markets page')).toBeNull()
  })

  it('redirects to /login, remembering where the user was going', () => {
    renderLayout({ user: null })
    expect(screen.getByText('login page, from /markets?status=all')).toBeInTheDocument()
  })

  it('renders the nav, the account and the page for a signed-in user', async () => {
    const auth = renderLayout({ user: { id: 1, email: 'ana@example.com' } })
    expect(screen.getByText('markets page')).toBeInTheDocument()
    expect(screen.getByText('ana@example.com')).toBeInTheDocument()
    const nav = screen.getByRole('navigation')
    expect([...nav.querySelectorAll('a')].map((a) => [a.textContent, a.getAttribute('href')])).toEqual([
      ['Overview', '/'],
      ['Markets', '/markets'],
      ['Traders', '/traders'],
      ['Trades', '/trades'],
      ['Activity', '/activity'],
      ['Indexer', '/indexer'],
    ])
    expect(screen.getByRole('link', { name: 'Markets' })).toHaveAttribute('aria-current', 'page')
    expect(screen.getByRole('link', { name: 'Overview' })).not.toHaveAttribute('aria-current')

    await userEvent.click(screen.getByRole('button', { name: 'Sign out' }))
    expect(auth.logout).toHaveBeenCalledTimes(1)
  })
})
