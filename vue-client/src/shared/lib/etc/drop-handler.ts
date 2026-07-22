// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

export function dropHandler(e: DragEvent): File[] {
  e.preventDefault()

  if (!e.dataTransfer) return []

  if (e.dataTransfer?.items) {
    return [...e.dataTransfer.items].reduce((acc: File[], cur: DataTransferItem) => {
      if (cur.kind !== 'file') return [...acc]
      const file = cur.getAsFile()
      return file ? [...acc, file] : [...acc]
    }, [])
  }

  return [...(e.dataTransfer.files ?? [])]
}
