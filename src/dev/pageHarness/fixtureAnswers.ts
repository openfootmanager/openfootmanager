import type { SupportedLanguageCode } from "../../i18n";
import type { SettingsResponse } from "../../store/settingsStore";
import type { StyleGuideOptions } from "../styleGuide/styleGuideOptions";
import * as fixture from "./fixture.generated";
import type { CommandHandlers } from "./harnessIpc";

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
    get_finance_snapshot: () => ({
      snapshot: fixture.financeSnapshot,
      previews: fixture.financePreviews,
    }),
    get_club_league_standing: () => fixture.clubLeagueStanding,
    get_squad: () => fixture.squad,
    get_available_jobs: () => fixture.availableJobs,
    get_contract_warning_stages: () => fixture.contractWarningStages,
  };
}
