export interface NormalizedError {
  message: string;
  stack?: string;
  cause?: unknown;
}

export function normalizeError(err: unknown): NormalizedError {
  if (err instanceof Error) {
    return { message: err.message, stack: err.stack, cause: err.cause };
  }

  if (typeof err === "string") {
    return { message: err };
  }

  return { message: "Unknown error", cause: err };
}
