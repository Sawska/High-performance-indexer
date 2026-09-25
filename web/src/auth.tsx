import { useCallback, useEffect, useState, type ReactNode } from 'react'
import { api, UNAUTHORIZED_EVENT, type User } from './api'
import { AuthContext } from './useAuth'

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null | undefined>(undefined)

  useEffect(() => {
    api<User>('/api/auth/me').then(setUser, () => setUser(null))
    const drop = () => setUser(null)
    window.addEventListener(UNAUTHORIZED_EVENT, drop)
    return () => window.removeEventListener(UNAUTHORIZED_EVENT, drop)
  }, [])

  const login = useCallback(async (email: string, password: string) => {
    setUser(await api<User>('/api/auth/login', { method: 'POST', body: { email, password } }))
  }, [])

  const register = useCallback(async (email: string, password: string) => {
    setUser(await api<User>('/api/auth/register', { method: 'POST', body: { email, password } }))
  }, [])

  const logout = useCallback(async () => {
    await api('/api/auth/logout', { method: 'POST', body: {} })
    setUser(null)
  }, [])

  return <AuthContext.Provider value={{ user, login, register, logout }}>{children}</AuthContext.Provider>
}
