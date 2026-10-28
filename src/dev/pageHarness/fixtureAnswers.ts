import type { SupportedLanguageCode } from "../../i18n";
import type { SettingsResponse } from "../../store/settingsStore";
import type { StyleGuideOptions } from "../styleGuide/styleGuideOptions";
import * as fixture from "./fixture.generated";
import { type CommandHandlers, UNANSWERED } from "./harnessIpc";

/** The fixture holds one club, the user's: anything asked about another club is declined. */
const USER_CLUB = fixture.gameState.manager.team_id;

function forUserClub(teamId: unknown, answer: () => unknown): unknown {
  return teamId === USER_CLUB ? answer() : UNANSWERED;
}

/** The settings the page sees, matching what the URL put on the document. */
export function harnessSettings(
  options: StyleGuideOptions,
  language: SupportedLanguageCode,
): SettingsResponse {
  const currency = { code: "EUR", symbol: "€", exchange_rate: 1 } as const;
  return {
    settings: {
      theme: options.theme,
      ui_scale: options.scale,
      high_contrast: options.highContrast,
      language,
    },
    currency,
    supported_currencies: [currency],
  };
}

/**
 * The answers to every command the dashboard shell, Home and Squad ask for on mount, from the
 * generated career. A page that needs another one shows it in the harness's unanswered list.
 */
export function fixtureHandlers(
  options: StyleGuideOptions,
  language: SupportedLanguageCode,
): CommandHandlers {
  return {
    get_settings: () => harnessSettings(options, language),
    get_active_game: () => fixture.gameState,
    get_active_save_id: () => "page-harness",
    get_career_stage: () => fixture.careerStage,
    // No team id means the manager's club, as in the Rust command.
    get_finance_snapshot: ({ teamId }) =>
      forUserClub(teamId ?? USER_CLUB, () => ({
        snapshot: fixture.financeSnapshot,
        previews: fixture.financePreviews,
      })),
    get_club_league_standing: ({ teamId }) => forUserClub(teamId, () => fixture.clubLeagueStanding),
    get_squad: ({ teamId }) => forUserClub(teamId, () => fixture.squad),
    // The user's club only, as in the Rust command.
    get_squad_fitness: () => fixture.squadFitness,
    get_available_jobs: () => fixture.availableJobs,
    // Only the players asked about; Rust leaves out an id no player has.
    get_contract_warning_stages: ({ playerIds }) => {
      const asked = new Set(Array.isArray(playerIds) ? playerIds : []);
      return Object.fromEntries(
        Object.entries(fixture.contractWarningStages).filter(([id]) => asked.has(id)),
      );
    },
  };
}
