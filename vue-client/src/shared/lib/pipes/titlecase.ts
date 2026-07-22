// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

export const toTitleCase = (item: string) =>
  item
    .toLowerCase()
    .split(' ')
    .map((x) => x.charAt(0).toUpperCase() + x.slice(1))
    .join(' ')
