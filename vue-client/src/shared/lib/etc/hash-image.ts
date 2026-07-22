import { createSHA256, type IDataType } from 'hash-wasm'

const CHUNK_SIZE = 20 * 1024 * 1024 // 20MB Chunk Size, arbitrary for the time being

// We are using this WASM hash over the web native, as we want to stream the file in, rather than loading a large image in browser memory
export async function fileToSHA256HashHex(
  file: File,
  options?: {
    updatePercentageCallback?: (x: number) => void
    checkForCancelationCallback?: () => boolean
  },
): Promise<string | null> {
  const sha256 = await createSHA256()
  sha256.init()

  const chunks = Math.ceil(file.size / CHUNK_SIZE)
  let position = 0

  for (let i = 0; i < chunks; i++) {
    const chunk = file.slice(position, position + CHUNK_SIZE)
    const buffer = await chunk.arrayBuffer()

    // Wrap the ArrayBuffer in a Uint8Array so it matches ITypedArray
    const typed: IDataType = new Uint8Array(buffer)
    position += CHUNK_SIZE

    if (options?.updatePercentageCallback) {
      options.updatePercentageCallback(Math.round((i * 100) / chunks))
    }

    if (options?.checkForCancelationCallback) {
      const shouldCancel = options.checkForCancelationCallback()
      if (shouldCancel) {
        return null
      }
    }

    sha256.update(typed)
  }

  return sha256.digest('hex')
}
