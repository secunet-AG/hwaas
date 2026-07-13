// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

interface BaseError {}

export class AppError extends Error implements BaseError {
  constructor(message: string) {
    super(message)
    this.name = 'AppError'
  }
}
