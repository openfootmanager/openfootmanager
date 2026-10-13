# S3: the calendar reconcile engine

As a career player, I need no club to play twice inside the rest window, whatever competitions it is in,
so a full rest follows every match. The league gives way to cups, cups to continental competitions, and
everything to international windows, and only the clashing match ever moves.

Refs #553 and #654. This slice adds the pure engine (`ofm_core::calendar_reconcile`), the named rest rule
(`calendar_booking::MIN_GAP_DAYS = 3`, replacing the merged P3 two-day rule and its unused ledger API) and the
one knockout date function (`knockout_dates::knockout_round_date`). No generator, day loop or loader calls
it yet: S4a, S5 and S6 wire it in. Layers, lowest first: friendly, league, domestic cup, continental
(club and Club World Cup), international. Same-layer clashes: the larger `priority` number yields.

| Named scenario | Given | When | Then |
| --- | --- | --- | --- |
| the_minimum_rest_is_three_days | A match on Saturday | Sunday to Monday and Tuesday are checked | The first three are too close; Tuesday is legal |
| the_rule_looks_backwards_as_well_as_forwards / the_rule_crosses_the_new_year | A match on March 8, or on December 31 | The days before, or January 1 to 3, are checked | The window is symmetric and ignores the year |
| a_league_match_moves_to_the_earliest_day_free_for_both_clubs | A cup tie and a league match of one club on one day | The calendar is reconciled | The league match moves to the first day three days clear, reason cup tie |
| only_the_clashing_match_moves | A league round with one clashing match | Reconciled | Only that match moves |
| a_cup_tie_never_moves_for_a_league_fixture | A cup and a league match clash | Reconciled | The cup keeps its date |
| a_domestic_cup_yields_to_a_continental_competition | A continental tie and a cup tie clash | Reconciled | The cup tie moves, reason continental tie |
| the_lower_priority_cup_moves_when_two_cups_collide / the_state_championship_with_the_larger_priority_yields | Two cups, or two leagues, clash with ids that sort against their priorities | Reconciled | The larger priority number yields, whatever the ids |
| a_friendly_yields_to_every_competitive_fixture | A friendly clashes with a league match | Reconciled | The friendly is dropped |
| a_played_or_live_fixture_never_moves / a_fixture_dated_today_never_moves | Played, in-progress or same-day fixtures | Reconciled | They never move, and still block their neighbours |
| a_league_match_moves_off_an_international_window | A blackout covers a league match | Reconciled | It moves off, reason international window |
| national_team_fixtures_are_ignored | A national-team fixture shares a date with a club match | Reconciled | Neither moves; nations never enter club occupancy |
| a_postponed_match_may_land_after_the_last_round | The only free day lies after the league's last round | Reconciled | The match lands there |
| a_final_round_match_that_clashes_moves_alone | A final round with one clash | Reconciled | The round is not moved as a block |
| a_dormant_cup_draw_displaces_an_active_league_fixture | A cup tie of a dormant competition and an active league match clash | Reconciled | The league match moves |
| a_shadow_date_is_never_a_displacement_target | A knockout whose next round is undrawn | A league match must move | It never lands on a date the next round could claim for the clubs still in |
| a_shadow_alone_never_moves_a_fixture | A league match on a date a cup draw could claim, with no firm clash | Reconciled | It stays |
| an_eliminated_club_frees_its_shadow_dates | A cup semifinal; one league match of a survivor and one of an eliminated club must move | Reconciled | Only the survivor avoids the final's dates |
| a_seeded_round_is_never_dated_before_its_feeding_round | A feeding round pushed past its successor by a window | Reconciled | The successor moves after the last feeding match |
| a_round_follows_the_actual_end_of_the_previous_one / the_grid_date_holds_when_the_previous_round_was_early / a_round_moves_off_a_blackout / a_round_keeps_the_rest_window_from_another_cups_round | `knockout_round_date` inputs | A round is dated | It follows the real end of the previous round plus the gap, not the grid date, and clears blackouts and other cups |
| a_blocked_calendar_returns_none_instead_of_hanging / a_search_past_the_cap_fails_loudly_instead_of_hanging | No legal day within 730 days | Searched / reconciled | A typed bug error, never a hang or a silent spill |
| an_unreadable_date_is_an_error_not_a_guess | A fixture with an unreadable date | Reconciled | An error names the fixture |
| the_sweep_changes_nothing_on_a_clean_calendar / the_result_does_not_depend_on_input_order / moves_are_forward_only | Any world | Reconciled twice, or in another order | The second sweep moves nothing, the result is identical, and every move goes forward |
| postponements_are_recorded_only_when_asked_and_keep_the_first_date | A moved fixture | Applied with and without marking | Generation records nothing; a published move records the original date once |
| the_sweep_keeps_every_invariant (property) | Random worlds up to 120 clubs: leagues, cups, a continental cup, windows, partial knockouts | Reconciled | No two fixtures of a club closer than the rest window (history aside), valid dates, immovables unchanged, nothing lost but friendlies, forward moves, rounds after their feeders, idempotent |
| the_result_does_not_depend_on_order (property) | The same random world in another competition order | Reconciled | Identical output |
| the_generator_produces_clashes_for_the_sweep_to_resolve | 40 seeds | Generated | At least 30 start with clashes and the sweep moves hundreds of fixtures: the properties are not passing on empty worlds |

## Open questions and limits
- A group-and-knockout cup has no shadows until its bracket exists.
- Real organisers sometimes move matches earlier; this engine only moves forward, so the real-case tests assert the
  invariants, not the real new dates.
- Timing is not asserted here; it is measured by hand when each wiring slice lands.
