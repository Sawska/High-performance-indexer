import { vi } from 'vitest'

type Route = unknown | ((url: URL) => unknown)

export const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } })

export function mockApi(routes: Record<string, Route>) {
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
    const url = new URL(String(input), 'http://localhost')
    if (!(url.pathname in routes)) return json({ error: `no fixture for ${url.pathname}` }, 404)
    const route = routes[url.pathname]
    const body = typeof route === 'function' ? (route as (u: URL) => unknown)(url) : route
    return body instanceof Response ? body : json(body)
  })
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

export function requests(fetchMock: ReturnType<typeof mockApi>, pathname: string): URL[] {
  return fetchMock.mock.calls
    .map(([input]) => new URL(String(input), 'http://localhost'))
    .filter((u) => u.pathname === pathname)
}

export function lastQuery(fetchMock: ReturnType<typeof mockApi>, pathname: string): Record<string, string> {
  const all = requests(fetchMock, pathname)
  if (all.length === 0) throw new Error(`no request for ${pathname}`)
  return Object.fromEntries(all[all.length - 1].searchParams)
}
