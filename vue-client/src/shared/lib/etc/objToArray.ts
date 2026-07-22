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
