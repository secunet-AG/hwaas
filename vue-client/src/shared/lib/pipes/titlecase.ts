export const toTitleCase = (item: string) =>
  item
    .toLowerCase()
    .split(' ')
    .map((x) => x.charAt(0).toUpperCase() + x.slice(1))
    .join(' ')
