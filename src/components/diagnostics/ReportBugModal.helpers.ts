import type { GameStateData } from "../../store/types";
import type { BundleSummary, DiagnosticsReport } from "../../services/reportService";

export type Frequency = "everyTime" | "sometimes" | "once";

export interface ReportDraft {
  whatHappened: string;
  expected: string;
  steps: string;
  frequency: Frequency;
}

export const EMPTY_DRAFT: ReportDraft = {
  whatHappened: "",
  expected: "",
  steps: "",
  frequency: "everyTime",
};

/**
 * Which required fields are still blank.
 *
 * Returned as a list rather than a boolean so the form can mark the fields themselves; a Continue
 * button that is simply disabled tells the player nothing about why.
 */
export function missingRequiredFields(draft: ReportDraft): Array<keyof ReportDraft> {
  return (["whatHappened", "expected"] as const).filter((field) => draft[field].trim() === "");
}

/** Human-readable size. Used on the preview, where the whole point is that nothing is hidden. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  // One decimal below 10 so "1.4 MB" does not round to "1 MB", none above, where it is noise.
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

/**
 * The one-line machine summary that goes in the issue form's `os` field.
 *
 * The GitHub form asks for an operating system in prose ("Windows 11"); what the app can actually
 * prove is the platform triple and the webview, which is what decides whether a rendering bug is
 * reproducible. Both go in.
 */
export function describeMachine(diagnostics: DiagnosticsReport | null): string {
  if (!diagnostics) return "";
  return `${diagnostics.os} ${diagnostics.arch} · webview ${diagnostics.webview_version}`;
}

/** The current screen, for the issue form's `resolution` field. */
export function describeResolution(screen: { width: number; height: number }): string {
  if (!screen.width || !screen.height) return "";
  return `${screen.width}x${screen.height}`;
}

/**
 * What the player said, folded into the single `game-context` field the form offers.
 *
 * Frequency is carried here as text rather than prefilling the form's dropdown: its options are
 * translated per template, so a value picked in one language matches nothing in another's form.
 */
export function describeContext(frequencyLabel: string, careerLine: string): string {
  return [careerLine, frequencyLabel].filter((part) => part.trim() !== "").join("\n");
}

/**
 * The player's own words, as the bundle stores them.
 *
 * The GitHub link has a length limit and trims what will not fit, so without this the trimmed
 * version would be the only copy — and a description in a script that percent-encodes to several
 * bytes per character reaches that limit quickly. This file is what survives.
 */
export function composeReportText(
  draft: ReportDraft,
  labels: {
    whatHappened: string;
    expected: string;
    steps: string;
    frequency: string;
    career: string;
  },
  careerLine = "",
): string {
  const sections: string[] = [
    `## ${labels.whatHappened}\n\n${draft.whatHappened.trim()}`,
    `## ${labels.expected}\n\n${draft.expected.trim()}`,
  ];
  if (draft.steps.trim() !== "") {
    sections.push(`## ${labels.steps}\n\n${draft.steps.trim()}`);
  }
  sections.push(`## ${labels.frequency}`);
  if (careerLine !== "") {
    sections.push(`## ${labels.career}\n\n${careerLine}`);
  }
  return `${sections.join("\n\n")}\n`;
}

/**
 * Where the player was when it broke: club, competition, in-game date, and the packages the world
 * was built from.
 *
 * Untranslated on purpose, like `describeMachine`. This is data a triager reads to reproduce the
 * bug, not prose — and the package ids in particular are the difference between "cannot reproduce"
 * and loading the same world. The manager's own name is deliberately absent: it is the one field
 * in the save that is usually the player's real one.
 */
export function describeCareer(gameState: GameStateData | null): string {
  if (!gameState) return "";
  const club = gameState.teams.find((team) => team.id === gameState.manager?.team_id)?.name;
  const league = gameState.league;
  const packages = (gameState.package_lockfile ?? [])
    .map((entry) => `${entry.id}@${entry.version}`)
    .join(", ");

  return [
    club,
    league === null ? undefined : `${league.name} ${league.season}`,
    gameState.clock?.current_date,
    packages === "" ? undefined : `packages: ${packages}`,
  ]
    .filter((part): part is string => typeof part === "string" && part.trim() !== "")
    .join(" · ");
}

/** A sentence naming where the bundle landed and how big it turned out. */
export function describeBundle(summary: BundleSummary): string {
  return `${summary.path} (${formatBytes(summary.bytes)})`;
}
