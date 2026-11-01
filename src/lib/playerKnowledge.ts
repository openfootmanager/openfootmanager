import type { TOptions } from "i18next";

import type { PlayerKnowledge, VisibleWage } from "../services/scoutingService";
import type { PotentialOpinion } from "../store/generated/PotentialOpinion";

type TranslateFn = (key: string, options?: TOptions) => string;

/** A player's rating and wage as the club may see them, from the backend's projection of a list. */
export interface VisibleFigures {
  ovr: number | null;
  wage: VisibleWage;
}

/** What a screen shows of a player it has no projection for: nothing private. */
export const NOTHING_VISIBLE: VisibleFigures = { ovr: null, wage: { kind: "hidden" } };

/**
 * A wage the club may see, written with `format`: as it is, as "about …", or "–". The one place a
 * visible wage is spelled, so every screen hides and rounds it alike.
 */
export function visibleWageText(
  wage: VisibleWage,
  format: (weeklyWage: number) => string,
  t: TranslateFn,
): string {
  if (wage.kind === "exact") return format(wage.weekly);
  if (wage.kind === "approximate") {
    return t("playerProfile.approximateWage", { wage: format(wage.weekly) });
  }
  return "–";
}

/** What the club's knowledge of players, and the wages it may see, are asked again on. */
export interface KnowledgeRevisionInputs {
  /** The career this game was installed as: another career's answers are never kept. */
  careerToken?: number;
  date: string;
  /** The club the players are at: the profile's player's, or the roster's. */
  playerClubId?: string | null;
  managerClubId?: string | null;
  /** The season's start: a rollover moves clubs between divisions, and so who is known, without
   *  moving the date. */
  seasonStart?: string | null;
  /** A profile's player's wage, last, so a renewal moves the key; none for a roster. */
  wage?: number;
}

/** The key the club's knowledge and wage answers are asked again on when it changes. */
export function knowledgeRevision(inputs: KnowledgeRevisionInputs): string {
  // JSON keeps the fields apart: a package's club ids may hold any character, a separator too.
  return JSON.stringify([
    inputs.careerToken ?? 0,
    inputs.date,
    inputs.playerClubId ?? "",
    inputs.managerClubId ?? "",
    inputs.seasonStart ?? "",
    inputs.wage ?? 0,
  ]);
}

/** The parts of the game that what the club knows moves with. */
export interface KnowledgeGame {
  career_token?: number;
  clock: { current_date: string };
  manager: { team_id?: string | null };
  season_context?: { season_start?: string | null } | null;
}

/**
 * The knowledge revision of the game a screen shows: its career, date, manager's club and season
 * start, picked here once for every screen, plus a screen's own inputs (a profile's player's club
 * and wage).
 */
export function gameKnowledgeRevision(
  game: KnowledgeGame,
  screenInputs: Pick<KnowledgeRevisionInputs, "playerClubId" | "wage"> = {},
): string {
  return knowledgeRevision({
    careerToken: game.career_token,
    date: game.clock.current_date,
    managerClubId: game.manager.team_id,
    seasonStart: game.season_context?.season_start,
    ...screenInputs,
  });
}

/** What `ownLineupRevision` reads of a game: the manager's club and every team's lineup. */
export interface LineupGame {
  manager: { team_id?: string | null };
  teams: { id: string; formation: string; starting_xi_ids: string[] }[];
}

/**
 * The manager's own lineup, as a revision: his team's formation and starting XI. A player's
 * position ratings mark the slot he is played in, which no knowledge revision follows, so the
 * profile asks for them again when this moves (benching him, or starting him elsewhere).
 */
export function ownLineupRevision(game: LineupGame): string {
  const team = game.teams.find((candidate) => candidate.id === game.manager.team_id);
  return team ? JSON.stringify([team.formation, team.starting_xi_ids]) : "no-team";
}

/**
 * A rating as a list or a card writes it: the number when the club knows it, "–" when it does not.
 * The one place a hidden rating is spelled, so every screen hides it alike.
 */
export function visibleRatingText(rating: number | null): string {
  return rating === null ? "–" : String(rating);
}

/** The club sees his live rating and attributes: he is its own, or plays in its division. */
export function knowsPlayerRating(knowledge: PlayerKnowledge): boolean {
  return knowledge.kind === "own" || knowledge.kind === "known";
}

/** Condition, morale and the exact wage are his club's business: only the club's own player shows them. */
export function knowsPrivateState(knowledge: PlayerKnowledge): boolean {
  return knowledge.kind === "own";
}

// The one trait the backend derives from potential rather than from attributes (domain
// `PlayerTrait`, `ofm_core::player_rating::qualifies_for_wonderkid`): the player's own copy is
// read from his true potential, so it tells what no club can see.
const WONDERKID = "Wonderkid";

/**
 * The traits the club may see. Wonderkid is the club's verdict, not the player's truth: it shows
 * when the club's opinion of his potential calls him one (`PotentialOpinion.wonderkid`), and the
 * player's own tag is never shown. No opinion, no tag.
 */
export function visibleTraits(traits: string[], opinion: PotentialOpinion | null): string[] {
  const attributeTraits = traits.filter((trait) => trait !== WONDERKID);
  return opinion?.wonderkid ? [...attributeTraits, WONDERKID] : attributeTraits;
}
