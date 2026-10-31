import type { PlayersPageQuery } from "../../services/playersService";

/**
 * The query the Players page opens with: every player in the world, best first, 30 to a page.
 * The page harness's fixture answers exactly this query (page_fixture.rs builds the same JSON by
 * hand, since Rust can't import it), and fixtureAnswers.test.ts holds the two equal, so a change on
 * either side fails a test instead of leaving the harness's Players page unanswered.
 */
export const PLAYERS_OPENING_QUERY: PlayersPageQuery = {
  search: null,
  position: null,
  positions: [],
  age_min: null,
  age_max: null,
  ovr_min: null,
  ovr_max: null,
  value_min: null,
  value_max: null,
  in_budget: false,
  scope: "all",
  team_id: null,
  status: "all",
  sort_key: "ovr",
  sort_asc: false,
  page: 1,
  page_size: 30,
};
