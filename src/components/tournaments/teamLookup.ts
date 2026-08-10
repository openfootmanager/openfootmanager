/**
 * How a tournaments view names a team and opens it.
 *
 * A competition's table can hold clubs or national teams, and the two are named
 * differently — clubs by their stored name, nations through a translated
 * template. Every row, table and fixture in this folder needs the same four
 * answers, so they travel together rather than as four repeated props.
 */
export interface TournamentsTeamLookup {
  /** The club being managed, so its row can be picked out. Null when unemployed. */
  userTeamId: string | null;
  /** National teams have no club page to open. */
  isClubTeam: (id: string) => boolean;
  resolveTeamName: (id: string) => string;
  onSelectTeam: (id: string) => void;
}
