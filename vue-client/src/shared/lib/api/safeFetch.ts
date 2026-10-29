// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

import { z } from 'zod'

export type FetchResult<T> = { data: T; error: null } | { data: null; error: string | z.ZodError }

/**
 * A fetch wrapper that validates all payloads at runtime. This will help unify runtime and "compiletime" issues.
 */
export async function safeFetch<T>(
  input: RequestInfo,
  init: RequestInit,
  schema: z.ZodType<T>,
): Promise<FetchResult<T>> {
  let res: Response
  try {
    res = await fetch(input, init)
  } catch (err) {
    return { data: null, error: `Error occured when fetching: ${err}` }
  }

  let body: unknown
  const contentType = res.headers.get('content-type') ?? ''

  // Binary or non-json/text case
  if (!contentType.includes('application/json') && !contentType.includes('text/')) {
    if (!res.ok) return { data: null, error: `HTTP ${res.status}` }
    return { data: (await res.arrayBuffer()) as T, error: null } //TODO: Evaluate this case for non-json/text payloads
  }

  // Read the body once as text so it is still available as a fallback if JSON parsing fails.
  let text: string
  try {
    text = await res.text()
  } catch (err) {
    return { data: null, error: `Failed to read response body with error: ${err}` }
  }

  if (contentType.includes('application/json')) {
    try {
      body = JSON.parse(text)
    } catch {
      return { data: null, error: `Request failed with message: ${text}` }
    }
  } else {
    body = text
  }

  if (!res.ok) {
    return { data: null, error: `HTTP ${res.status}: ${JSON.stringify(body)}` }
  }

  const parsed = schema.safeParse(body)
  if (!parsed.success) {
    return { data: null, error: parsed.error }
  }

  return { data: parsed.data, error: null } as { data: T; error: null }
}

/**
 * A utility function that will flatten FetchResult<T>[] to FetchResult<T[]>.
 * If propogate error is true, any error will bubble up and cancel the entire flattening.
 */
export function flattenResultArray<T>(
  data: FetchResult<T>[],
  options: { propogateError: boolean } = { propogateError: true },
): FetchResult<T[]> {
  if (options.propogateError) {
    const hasError = data.findIndex((x) => !!x.error)
    if (hasError !== -1) {
      return {
        data: null,
        error: `Atleast one result threw an error: ${data[hasError]?.error}`,
      }
    }
  }
  const flat = data.reduce(
    (acc, cur) => ({ data: cur.data ? [...acc.data, cur.data] : acc.data, error: null }),
    {
      data: [] as T[],
      error: null,
    },
  )
  return flat
}

/*
  TODO: Something like this:

  enum SafeFetchError {
    Network(String),
    Http(String)...
  }
*/
export enum SafeFetchError {
  Network = 'NETWORK_ERROR',
  Http = 'HTTP_ERROR',
  InvalidJson = 'INVALID_JSON',
  InvalidText = 'INVALID_TEXT',
  Validation = 'VALIDATION_ERROR',
  Unknown = '',
}
