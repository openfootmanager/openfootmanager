# P3: one world-wide booking rule

As a career player, I need every club's matches to be at least two dates apart across every league,
cup and continental competition, so a full rest day always separates two fixtures of the same club.

Refs #553 and #654. This slice adds one pure rule and planner in `ofm_core::calendar_booking`. No fixture
writer, kick-off route or save loader calls it yet: P4 adopts it at every writer and P5 repairs loaded
saves. The rule reads fixtures only, so manager, country, scope and competition order cannot change it.
Not covered here: the 20-club full-window fit (16) and four-season play (65), which need the P4 writers.

| Named scenario | Given | When | Then |
| --- | --- | --- | --- |
| league_and_continental_same_day_plan_is_rejected | A league match on Tuesday | A continental tie is proposed the same day | The batch fails naming both fixtures and the club |
| league_and_continental_consecutive_day_plan_is_rejected | A league match on Tuesday | A tie is proposed on Wednesday or Monday | The batch is rejected, never shifted |
| one_full_rest_day_allows_a_cross_competition_match | A league match on Tuesday | A tie is proposed on Thursday or the Saturday before | It is accepted |
| rest_rule_does_not_depend_on_the_kind_of_competition | A booked club | A fixture of any competition kind, including national-team fixtures, is proposed beside it | Every kind is rejected alike |
| both_participants_and_both_neighbours_are_checked | Home club free, away club booked the day before or after | A fixture is proposed | It fails for the away club |
| rest_rule_crosses_december_and_january | A club booked on December 31 | January 1 and January 2 are proposed | Only January 2 is legal |
| calendar_mirrors_do_not_create_or_hide_real_conflicts | A fixture in a competition and its legacy mirror, plus a distinct same-opponent cup tie | The ledger is built | The mirror counts once and the cup tie counts separately |
| ledger_does_not_depend_on_competition_order | The same fixtures in a different competition order | The ledger is built | The ledgers are identical |
| unreadable_fixture_date_is_unresolved_not_empty | A fixture with an unreadable date | The ledger is built | An error names the fixture; it is not an empty booking |
| played_and_live_fixtures_keep_their_reservation | Completed or InProgress fixtures | A neighbouring day is proposed | Both still reserve their dates |
| members_of_one_batch_conflict_with_each_other | Proposals in one batch sharing a club on consecutive days | The batch is validated | The batch is rejected |
| a_fixture_proposed_twice_in_one_batch_is_rejected | One fixture proposed twice, even on different dates or with different participants | The batch is validated | Rejected as a repeated fixture, independent of the rest rule |
| planning_rejects_a_flexible_match_that_repeats_a_fixed_fixture | A flexible match sharing a fixed fixture's identity | Planning is attempted | It fails and places nothing |
| mirrors_that_disagree_about_a_fixture_are_inconsistent | A competition and its mirror listing one fixture on different dates | The ledger is built | It fails instead of reserving both dates |
| incompatible_fixed_openers_refuse_a_calendar_plan | A club forced into two hard openers on consecutive dates | Planning is attempted | It fails and moves no opener |
| flexible_match_takes_the_earliest_legal_day_in_its_window | No commitments | A flexible match is planned | It takes its window's first day |
| flexible_match_skips_the_rest_days_around_a_commitment | A commitment at the window start | A flexible match is planned | It lands two days clear |
| impossible_capacity_rejects_the_whole_batch | Two matches of one club in a two-day window | They are planned | The batch fails and nothing is placed |
| plan_does_not_depend_on_input_order | The same flexible matches in any order | They are planned | The plan is identical |
| paired_half_openers_reserve_the_previous_half_rest_day | A club opening the next half on July 1 | The previous half's last matchday is fitted | It ends by June 29; a June-30-only window is refused |
| a_failed_plan_leaves_the_ledger_untouched | A failing plan | The ledger is read afterwards | It is unchanged |
