import type { SupportedLanguageCode } from "../../i18n";
import type { SettingsResponse } from "../../store/settingsStore";
import type { StyleGuideOptions } from "../styleGuide/styleGuideOptions";
import * as fixture from "./fixture.generated";
import type { HarnessProfile } from "./harnessPage";
import { type CommandHandlers, UNANSWERED } from "./harnessIpc";
import { profileCase } from "./profileCases";

/** The fixture holds one club, the user's: anything asked about another club is declined. */
const USER_CLUB = fixture.gameState.manager.team_id;

function forUserClub(teamId: unknown, answer: () => unknown): unknown {
  return teamId === USER_CLUB ? answer() : UNANSWERED;
}

/**
 * Whether a list of ids names exactly the players of `expected`, each once, in whatever order: the
 * same length and every id there is not enough, as one player named eleven times has both.
 */
function sameIds(ids: unknown, expected: readonly string[]): boolean {
  if (!Array.isArray(ids)) return false;
  const given = new Set<unknown>(ids);
  const wanted = new Set(expected);
  return (
    given.size === ids.length &&
    given.size === wanted.size &&
    ids.every((id) => typeof id === "string" && wanted.has(id))
  );
}

/** The same JSON whatever the key order: the page builds its query by spreading, Rust by serde. */
function canonicalJson(value: unknown): string {
  return JSON.stringify(value, (_key, inner: unknown) =>
    inner !== null && typeof inner === "object" && !Array.isArray(inner)
      ? Object.fromEntries(Object.entries(inner).sort(([a], [b]) => a.localeCompare(b)))
      : inner,
  );
}

/**
 * The fixture holds the answer to one query: the one it was generated for. Any other (a search, a
 * filter, the next page) is declined, so the page can't show the opening page as its result.
 */
function forQuery(query: unknown, generatedFor: unknown, answer: () => unknown): unknown {
  return canonicalJson(query) === canonicalJson(generatedFor) ? answer() : UNANSWERED;
}

/** The game with the case's patch laid over its player: a state the generated career has none of. */
function withPlayerPatch(game: typeof fixture.gameState, patch: ReturnType<typeof profileCase>) {
  if (!patch.playerPatch) return game;
  return {
    ...game,
    players: game.players.map((player) =>
      player.id === patch.playerId ? { ...player, ...patch.playerPatch } : player,
    ),
  };
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
      list_view: options.listView,
    },
    currency,
    supported_currencies: [currency],
  };
}

/**
 * What a player's profile asks the backend about him, answered for the one player the harness opened
 * it on and only for him: the knowledge rule's answers (`profileCase`), and his season.
 */
function profileHandlers(profile: HarnessProfile): CommandHandlers {
  const answer = profileCase(profile);
  const about = (playerId: unknown, value: unknown) =>
    playerId === answer.playerId ? value : UNANSWERED;
  return {
    // Rust answers every requested player in the world and leaves out an id no player has (the
    // retiree's). The fixture knows one player: a request for another active one is declined, not
    // answered with a gap the page would read as "unknown".
    get_player_knowledge: ({ playerIds }) => {
      const asked = Array.isArray(playerIds) ? playerIds : [];
      const activeIds = new Set(fixture.gameState.players.map((player) => player.id));
      if (asked.some((id) => activeIds.has(id) && id !== answer.playerId)) return UNANSWERED;
      return answer.knowledge && asked.includes(answer.playerId)
        ? { [answer.playerId]: answer.knowledge }
        : {};
    },
    get_visible_wage: ({ playerId }) => about(playerId, answer.wage),
    get_potential_opinion: ({ playerId }) => about(playerId, answer.opinion),
    get_position_ratings: ({ playerId }) => about(playerId, answer.positionRatings),
    get_scouted_report: ({ playerId }) => about(playerId, answer.scoutedReport),
    // The previews the Contract tab loads: the backend answers them for a contract the club holds
    // (the release one not for a player on loan) and refuses them otherwise.
    preview_contract_termination: ({ playerId }) => {
      const termination = answer.contractPreviews?.termination;
      return playerId === answer.playerId && termination ? { preview: termination } : UNANSWERED;
    },
    preview_renewal_financial_impact: ({ playerId }) => {
      const projection = answer.contractPreviews?.projection;
      return playerId === answer.playerId && projection ? { projection } : UNANSWERED;
    },
    get_retired_player: ({ playerId }) => about(playerId, answer.retiredPlayer),
    get_player_stats_overview: ({ playerId }) => about(playerId, answer.seasonStats),
    get_player_career: ({ playerId }) => about(playerId, answer.career),
    // The page asks for its last `limit` matches, latest first.
    get_player_match_history: ({ playerId, limit }) =>
      about(
        playerId,
        typeof limit === "number" ? answer.recentMatches.slice(0, limit) : answer.recentMatches,
      ),
  };
}

/**
 * The answers to every command the dashboard shell and each harness page ask for on mount, from
 * the generated career. A page that needs another one shows it in the harness's unanswered list.
 */
export function fixtureHandlers(
  options: StyleGuideOptions,
  language: SupportedLanguageCode,
  profile: HarnessProfile | null = null,
): CommandHandlers {
  const patch = profile ? profileCase(profile) : null;
  return {
    ...(profile ? profileHandlers(profile) : {}),
    get_settings: () => harnessSettings(options, language),
    get_active_game: () => (patch ? withPlayerPatch(fixture.gameState, patch) : fixture.gameState),
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
    get_rating_tiers: () => fixture.ratingTiers,
    get_position_groups: () => fixture.positionGroups,
    // The user's club only, as in the Rust command: the grade the Squad and Tactics pages show.
    get_familiarity: () => fixture.familiarity,
    // Only the players asked about; Rust leaves out an id no player has.
    get_contract_warning_stages: ({ playerIds }) => {
      const asked = new Set(Array.isArray(playerIds) ? playerIds : []);
      return Object.fromEntries(
        Object.entries(fixture.contractWarningStages).filter(([id]) => asked.has(id)),
      );
    },
    get_players_page: ({ query }) =>
      forQuery(query, fixture.playersPageQuery, () => fixture.playersPage),
    get_teams_directory: ({ query }) =>
      forQuery(query, { search: null }, () => fixture.teamsDirectory),
    get_staff: ({ teamId }) => forUserClub(teamId, () => fixture.staff),
    // A new career has no saved tactics of its own, and nothing from an older build to import.
    list_custom_tactics: () => [],
    legacy_tactics_import_pending: () => false,
    // The side the club would field, as the backend projects it: the Tactics page's table and pitch.
    get_tactics_lineup: () => fixture.tacticsLineup,
    get_tactics_blueprint_status: () => fixture.tacticsBlueprintStatus,
    // The takers the backend picked for the club's eleven: answered for that eleven only, in any order.
    auto_select_set_pieces: ({ playerIds }) =>
      sameIds(playerIds, fixture.autoSetPiecesFor) ? fixture.autoSetPieces : UNANSWERED,
  };
}
