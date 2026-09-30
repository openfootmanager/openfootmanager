import { GITHUB_REPO_URL } from "./communityLinks";
import { bugReportTemplateFor } from "./issueTemplates";

/** What the player typed, plus what the app knows about their machine. */
export interface BugReportFields {
  whatHappened: string;
  expected: string;
  steps: string;
  gameContext: string;
  appVersion: string;
  os: string;
  resolution: string;
}

/**
 * Browsers and servers stop honouring very long URLs, and the limit is not standardised. 8 000 is
 * comfortably under every real one; the evidence lives in the bundle the player attaches, so
 * trimming prose here costs nothing that matters.
 */
const MAX_URL_LENGTH = 8_000;

/** Field ids from `.github/ISSUE_TEMPLATE/bug_report*.yml`, in the order the form shows them. */
const FIELD_IDS: Array<[keyof BugReportFields, string]> = [
  ["whatHappened", "what-happened"],
  ["expected", "expected-behavior"],
  ["steps", "steps-to-reproduce"],
  ["gameContext", "game-context"],
  ["appVersion", "version"],
  ["os", "os"],
  ["resolution", "resolution"],
];

/**
 * A GitHub issue-form URL with the fields already filled in.
 *
 * `frequency` is deliberately never prefilled. It is a dropdown whose *options* are translated per
 * template, so a value chosen in English matches nothing in the German form — and GitHub drops an
 * option it cannot match without reporting it, which would look like the app silently losing an
 * answer the player gave.
 */
export function buildBugReportUrl(locale: string, fields: BugReportFields): string {
  const build = (limit: number): string => {
    const params = new URLSearchParams();
    params.set("template", bugReportTemplateFor(locale));
    for (const [key, id] of FIELD_IDS) {
      const value = fields[key].trim();
      // An empty parameter would overwrite the template's placeholder with nothing, leaving a box
      // that looks answered and is not.
      if (value === "") continue;
      params.set(id, value.length > limit ? `${value.slice(0, limit)}…` : value);
    }
    return `${GITHUB_REPO_URL}/issues/new?${params.toString()}`;
  };

  let url = build(Number.POSITIVE_INFINITY);
  if (url.length <= MAX_URL_LENGTH) return url;

  // Shrink the per-field allowance until the whole URL fits. Halving converges in a handful of
  // passes and keeps every field represented, rather than dropping the later ones entirely.
  let limit = 2_000;
  while (limit > 32) {
    url = build(limit);
    if (url.length <= MAX_URL_LENGTH) return url;
    limit = Math.floor(limit / 2);
  }
  return build(32);
}
