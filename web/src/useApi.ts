import { useEffect, useState } from 'react'
import { api } from './api'

type State<T> = { data: T | null; error: string | null; loading: boolean }

export function useApi<T>(path: string | null, refreshMs?: number): State<T> {
  const [state, setState] = useState<State<T>>({ data: null, error: null, loading: path !== null })

  useEffect(() => {
    if (path === null) return
    let stop = false
    const load = async () => {
      setState((s) => ({ ...s, loading: true }))
      try {
        const data = await api<T>(path)
        if (!stop) setState({ data, error: null, loading: false })
      } catch (e) {
        if (!stop) setState((s) => ({ ...s, error: (e as Error).message, loading: false }))
      }
    }
    load()
    const id = refreshMs ? setInterval(load, refreshMs) : undefined
    return () => {
      stop = true
      clearInterval(id)
    }
  }, [path, refreshMs])

  return state
}
