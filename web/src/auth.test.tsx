import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { UNAUTHORIZED_EVENT } from './api'
import { AuthProvider } from './auth'
import { json, mockApi } from './test/fetch'
import { useAuth } from './useAuth'

function Who() {
  const { user, login, logout } = useAuth()
  if (user === undefined) return <p>checking</p>
  return (
    <>
      <p>{user ? `signed in as ${user.email}` : 'signed out'}</p>
      <button onClick={() => login('ana@example.com', 'hunter22').catch(() => {})}>log in</button>
      <button onClick={() => logout()}>log out</button>
    </>
  )
}

const renderAuth = () =>
  render(
    <AuthProvider>
      <Who />
    </AuthProvider>,
  )

describe('AuthProvider', () => {
  it('loads the current user from /api/auth/me', async () => {
    mockApi({ '/api/auth/me': { id: 1, email: 'ana@example.com' } })
    renderAuth()
    expect(screen.getByText('checking')).toBeInTheDocument()
    expect(await screen.findByText('signed in as ana@example.com')).toBeInTheDocument()
  })

  it('treats a failed /api/auth/me as signed out', async () => {
    mockApi({ '/api/auth/me': () => json({ error: 'not signed in' }, 401) })
    renderAuth()
    expect(await screen.findByText('signed out')).toBeInTheDocument()
  })

  it('signs out when any data request comes back 401', async () => {
    mockApi({ '/api/auth/me': { id: 1, email: 'ana@example.com' } })
    renderAuth()
    await screen.findByText('signed in as ana@example.com')
    act(() => {
      window.dispatchEvent(new Event(UNAUTHORIZED_EVENT))
    })
    expect(screen.getByText('signed out')).toBeInTheDocument()
  })

  it('logs in and out through the API', async () => {
    const fetchMock = mockApi({
      '/api/auth/me': () => json({ error: 'not signed in' }, 401),
      '/api/auth/login': { id: 1, email: 'ana@example.com' },
      '/api/auth/logout': () => new Response(null, { status: 204 }),
    })
    renderAuth()
    await userEvent.click(await screen.findByRole('button', { name: 'log in' }))
    expect(await screen.findByText('signed in as ana@example.com')).toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/auth/login',
      expect.objectContaining({ method: 'POST', body: '{"email":"ana@example.com","password":"hunter22"}' }),
    )

    await userEvent.click(screen.getByRole('button', { name: 'log out' }))
    expect(await screen.findByText('signed out')).toBeInTheDocument()
  })
})
