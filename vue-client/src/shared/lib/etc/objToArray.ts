// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

export function objToArray<T extends Object, K extends string | number | symbol>(
  object: Record<K, T>,
  identifier: string,
) {
  return Object.entries(object).map((x) => {
    const [k, v] = x

    const newV = structuredClone(v) as T
    ;(newV as any)[identifier] = k

    return newV as T & { [identifier]: string }
  })
}
