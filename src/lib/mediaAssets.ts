const URI_SCHEME = /^[a-z][a-z0-9+.-]*:/i;

export function resolveLocalMediaPath(path: string | null | undefined): string | null {
  if (!path) return null;
  const trimmed = path.trim();
  if (!trimmed || URI_SCHEME.test(trimmed) || trimmed.startsWith("//")) {
    return null;
  }
  return trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
}
