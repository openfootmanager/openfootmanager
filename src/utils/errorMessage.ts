import { resolveBackendError } from "./backendI18n";
import { extractErrorMessage } from "./errorText";

export function getErrorMessage(error: unknown): string {
  return extractErrorMessage(error, String(error));
}

/**
 * A backend error as a banner shows it: the one resolver (`resolveBackendError`) translates the key
 * and its params (a param that is itself a key, a money amount). A message that is not a known key
 * is shown as it was.
 */
export function resolveTranslatedErrorMessage(error: unknown): string {
  return resolveBackendError(error) || getErrorMessage(error);
}
