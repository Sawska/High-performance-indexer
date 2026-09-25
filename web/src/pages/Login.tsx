import { useState, type FormEvent } from 'react'
import { Link, Navigate, useLocation } from 'react-router'
import { useAuth } from '../useAuth'

export function Login({ mode }: { mode: 'login' | 'register' }) {
  const { user, login, register } = useAuth()
  const location = useLocation()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const from = (location.state as { from?: string } | null)?.from ?? '/'
  if (user) return <Navigate to={from} replace />

  const isRegister = mode === 'register'

  async function submit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    if (isRegister && password !== confirm) {
      setError('Passwords do not match')
      return
    }
    setBusy(true)
    try {
      await (isRegister ? register : login)(email, password)
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="auth-page">
      <form className="auth-card" onSubmit={submit}>
        <div className="brand">Polymarket Indexer</div>
        <h1>{isRegister ? 'Create account' : 'Sign in'}</h1>

        <label>
          Email
          <input type="email" autoComplete="email" required value={email} onChange={(e) => setEmail(e.target.value)} autoFocus />
        </label>
        <label>
          Password
          <input
            type="password"
            autoComplete={isRegister ? 'new-password' : 'current-password'}
            required
            minLength={isRegister ? 8 : undefined}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        {isRegister && (
          <label>
            Confirm password
            <input type="password" autoComplete="new-password" required value={confirm} onChange={(e) => setConfirm(e.target.value)} />
          </label>
        )}
        {isRegister && <div className="hint">At least 8 characters.</div>}

        {error && <div className="error-box">{error}</div>}

        <button className="primary" type="submit" disabled={busy}>
          {busy ? '…' : isRegister ? 'Create account' : 'Sign in'}
        </button>

        <div className="switch">
          {isRegister ? (
            <>Have an account? <Link to="/login" state={location.state}>Sign in</Link></>
          ) : (
            <>No account? <Link to="/register" state={location.state}>Create one</Link></>
          )}
        </div>
      </form>
    </div>
  )
}
