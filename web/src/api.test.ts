import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api, ApiError, qs, UNAUTHORIZED_EVENT } from './api'

describe('qs', () => {
  it('returns an empty string when nothing is set', () => {
    expect(qs({})).toBe('')
    expect(qs({ a: undefined, b: '' })).toBe('')
  })

  it('drops undefined and empty values, keeps zero', () => {
    expect(qs({ q: 'rain', status: undefined, sort: '', limit: 50, offset: 0 })).toBe('?q=rain&limit=50&offset=0')
  })

  it('encodes values', () => {
    expect(qs({ q: 'a b&c' })).toBe('?q=a+b%26c')
  })
})

describe('api', () => {
  const fetchMock = vi.fn<typeof fetch>()
  const onUnauthorized = vi.fn()

  beforeEach(() => {
    fetchMock.mockReset()
    vi.stubGlobal('fetch', fetchMock)
    window.addEventListener(UNAUTHORIZED_EVENT, onUnauthorized)
  })

  afterEach(() => {
    window.removeEventListener(UNAUTHORIZED_EVENT, onUnauthorized)
    onUnauthorized.mockReset()
  })

  it('GETs same-origin and parses JSON', async () => {
    fetchMock.mockResolvedValue(new Response('{"total":1,"rows":[]}'))
    await expect(api('/api/markets?limit=1')).resolves.toEqual({ total: 1, rows: [] })
    expect(fetchMock).toHaveBeenCalledWith('/api/markets?limit=1', {
      method: 'GET',
      credentials: 'same-origin',
      headers: undefined,
      body: undefined,
    })
  })

  it('sends a JSON body with its content type', async () => {
    fetchMock.mockResolvedValue(new Response('{"id":1,"email":"a@b.c"}'))
    await api('/api/auth/login', { method: 'POST', body: { email: 'a@b.c', password: 'pw' } })
    expect(fetchMock).toHaveBeenCalledWith('/api/auth/login', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json' },
      body: '{"email":"a@b.c","password":"pw"}',
    })
  })

  it('returns undefined for an empty body', async () => {
    fetchMock.mockResolvedValueOnce(new Response(''))
    await expect(api('/api/auth/logout', { method: 'POST', body: {} })).resolves.toBeUndefined()
    fetchMock.mockResolvedValueOnce(new Response(null, { status: 204 }))
    await expect(api('/api/auth/logout', { method: 'POST', body: {} })).resolves.toBeUndefined()
  })

  it('throws ApiError with the server error message and status', async () => {
    fetchMock.mockResolvedValue(new Response('{"error":"market not found"}', { status: 404, statusText: 'Not Found' }))
    const err = await api('/api/markets/nope').catch((e: unknown) => e)
    expect(err).toBeInstanceOf(ApiError)
    expect(err).toMatchObject({ status: 404, message: 'market not found' })
  })

  it('falls back to the status line when the error body is not JSON', async () => {
    fetchMock.mockResolvedValue(new Response('<html>oops</html>', { status: 502, statusText: 'Bad Gateway' }))
    await expect(api('/api/overview')).rejects.toMatchObject({ status: 502, message: '502 Bad Gateway' })
  })

  it('falls back to the status line when the JSON body has no error field', async () => {
    fetchMock.mockResolvedValue(new Response('{}', { status: 500, statusText: 'Internal Server Error' }))
    await expect(api('/api/overview')).rejects.toMatchObject({ status: 500, message: '500 Internal Server Error' })
  })

  it('dispatches the unauthorized event on 401 from a data path', async () => {
    fetchMock.mockResolvedValue(new Response('{"error":"not signed in"}', { status: 401 }))
    await expect(api('/api/markets')).rejects.toMatchObject({ status: 401, message: 'not signed in' })
    expect(onUnauthorized).toHaveBeenCalledTimes(1)
  })

  it('does not dispatch the unauthorized event on 401 from /api/auth/*', async () => {
    // A Response body reads once; hand each call a fresh one.
    fetchMock.mockImplementation(async () => new Response('{"error":"bad credentials"}', { status: 401 }))
    await expect(api('/api/auth/me')).rejects.toMatchObject({ status: 401 })
    await expect(api('/api/auth/login', { method: 'POST', body: {} })).rejects.toMatchObject({
      status: 401,
      message: 'bad credentials',
    })
    expect(onUnauthorized).not.toHaveBeenCalled()
  })

  it('does not dispatch the unauthorized event on other errors', async () => {
    fetchMock.mockResolvedValue(new Response('{"error":"forbidden"}', { status: 403 }))
    await expect(api('/api/markets')).rejects.toMatchObject({ status: 403 })
    expect(onUnauthorized).not.toHaveBeenCalled()
  })
})
