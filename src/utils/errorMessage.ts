import { resolveBackendError } from "./backendI18n";

type TranslateFn = (key: string, params?: Record<string, string | number>) => string;

function extractErrorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }

  return String(error);
}

export function getErrorMessage(error: unknown): string {
  return extractErrorMessage(error);
}

/**
 * A backend error as a banner shows it: the one resolver (`resolveBackendError`) translates the key
 * and its params (a param that is itself a key, a money amount). A message that is not a known key
 * is shown as it was. The second argument is the caller's `t`, which this no longer reads: the
 * resolver uses the app's own i18n, so a second resolver cannot disagree with the first. Callers
 * can drop it.
 */
export function resolveTranslatedErrorMessage(error: unknown, _t?: TranslateFn): string {
  return resolveBackendError(error) || extractErrorMessage(error);
}
