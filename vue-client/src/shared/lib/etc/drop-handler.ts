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
