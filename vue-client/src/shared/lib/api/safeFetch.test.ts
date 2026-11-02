// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, it, vi } from 'vitest'
import { z } from 'zod'
import { safeFetch } from './safeFetch'

function mockResponse(body: string | null, status: number, contentType?: string) {
  const headers = contentType ? { 'content-type': contentType } : undefined
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(body, { status, headers })))
}

describe('safeFetch', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('parses and validates a JSON body', async () => {
    mockResponse('{"a":1}', 200, 'application/json')
    const res = await safeFetch('/x', {}, z.object({ a: z.number() }))
    expect(res).toEqual({ data: { a: 1 }, error: null })
  })

  it('reports the raw body when JSON is malformed', async () => {
    mockResponse('not json', 500, 'application/json')
    const res = await safeFetch('/x', {}, z.unknown())
    expect(res.error).toBe('Request failed with message: not json')
  })

  it('reports an error for an empty body with a failing status', async () => {
    mockResponse(null, 500)
    const res = await safeFetch('/x', {}, z.unknown())
    expect(res.error).toBe('HTTP 500')
  })

  it('treats an empty body with a successful status as success', async () => {
    mockResponse(null, 200)
    const res = await safeFetch('/x', {}, z.unknown())
    expect(res.error).toBeNull()
  })

  it('includes the body in the error for a failing JSON response', async () => {
    mockResponse('["failed to add tag"]', 500, 'application/json')
    const res = await safeFetch('/x', {}, z.unknown())
    expect(res.error).toBe('HTTP 500: ["failed to add tag"]')
  })
})
