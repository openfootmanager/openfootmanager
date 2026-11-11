import { SUPPORTED_LANGUAGES, type SupportedLanguageCode } from "../../i18n";

/** `?lang=de` renders the page in that language (German has the longest labels); English otherwise. */
export function readHarnessLanguage(search: string): SupportedLanguageCode {
  const lang = new URLSearchParams(search).get("lang");
  return SUPPORTED_LANGUAGES.find((language) => language.code === lang)?.code ?? "en";
}

/** The pages the harness can mount, by the Dashboard tab each opens on. */
export const HARNESS_PAGES = {
  home: "Home",
  squad: "Squad",
  players: "Players",
  teams: "Teams",
  managers: "Managers",
  staff: "Staff",
  // A player's profile opens over the Players tab, as it does from a list.
  profile: "Players",
  tactics: "Tactics",
} as const;
export type HarnessPage = keyof typeof HARNESS_PAGES;

function isHarnessPage(page: string | null): page is HarnessPage {
  return page !== null && Object.keys(HARNESS_PAGES).includes(page);
}

/** `?page=squad` and the like; a page the harness doesn't know is the style guide itself. */
export function readHarnessPage(search: string): HarnessPage | null {
  const page = new URLSearchParams(search).get("page");
  return isHarnessPage(page) ? page : null;
}

/**
 * The states a player's profile is in, by how the club knows him (`ofm_core::player_knowledge`):
 * its own player, a rival in its division, one a scout reported on, one it knows nothing of, a free
 * agent whose last contract ran out, and one who has retired. Each shows different facts, so each is a case of the layout check.
 */
export const HARNESS_PROFILES = [
  "own",
  "keeper",
  "injured",
  "loaned",
  "rival",
  "scouted",
  "unknown",
  "freeAgent",
  "retired",
] as const;
export type HarnessProfile = (typeof HARNESS_PROFILES)[number];

/** `?page=profile&player=rival` and the like; the club's own player when it names none. */
export function readHarnessProfile(search: string): HarnessProfile {
  const player = new URLSearchParams(search).get("player");
  return HARNESS_PROFILES.find((profile) => profile === player) ?? "own";
}

/**
 * The two states of the World Editor's library the harness mounts: a library with worlds in it, and a
 * new editor with none (which says how to start).
 */
export const HARNESS_WORLD_LIBRARIES = ["populated", "empty"] as const;
export type HarnessWorldLibrary = (typeof HARNESS_WORLD_LIBRARIES)[number];

/**
 * `?page=worlds&library=empty` and the like: the World Editor's home, which is a route of its own and
 * not a dashboard tab. Null for any other page; `populated` when it names no library.
 */
export function readHarnessWorlds(search: string): HarnessWorldLibrary | null {
  const params = new URLSearchParams(search);
  if (params.get("page") !== "worlds") return null;
  return (
    HARNESS_WORLD_LIBRARIES.find((library) => library === params.get("library")) ?? "populated"
  );
}
