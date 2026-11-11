/**
 * The text an unknown thrown value carries: a string as it is, an Error's or an `{ message }`
 * object's message. Anything else gives `fallback`, so a caller that must show something passes
 * `String(error)` and one that only parses a key leaves it empty.
 */
export function extractErrorMessage(error: unknown, fallback = ""): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;

  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }

  return fallback;
}
