/** The pages the harness can mount, by the Dashboard tab each opens on. */
export const HARNESS_PAGES = { home: "Home", squad: "Squad" } as const;
export type HarnessPage = keyof typeof HARNESS_PAGES;

/** `?page=home` or `?page=squad`; anything else is the style guide itself. */
export function readHarnessPage(search: string): HarnessPage | null {
  const page = new URLSearchParams(search).get("page");
  return page === "home" || page === "squad" ? page : null;
}
