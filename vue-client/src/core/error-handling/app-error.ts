interface BaseError {}

export class AppError extends Error implements BaseError {
  constructor(message: string) {
    super(message)
    this.name = 'AppError'
  }
}
