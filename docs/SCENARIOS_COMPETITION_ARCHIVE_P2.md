# P2: immutable completed-edition archive

As a career player, I need the final table, bracket and champion of every finished competition
edition frozen once, so later slices can renew a competition on its own calendar without losing
the result that promotion, awards and qualification read.

Refs #654. Stacked on P1 (#710). This slice proves completion, records one immutable archive entry per
`(competition, season)` and persists it. It does not renew a competition, advance an edition, move
clubs or change any rollover outcome. Tables are archivable only when P1 recorded their authored legs;
older saves without them stay unarchived rather than guessing two legs. Each scenario maps to a named
test, seen failing first and mutation-checked.

The completion validator is shared for AI, user and dormant results. It reads the saved competition
rather than the manager, clock, name or simulation route.

## Completion proof
| Named scenario | Given | When | Then |
| --- | --- | --- | --- |
| completed_tables_respect_authored_legs_and_odd_fields | Completed three/four-club one/two-leg tables | Completion is verified | Each full authored schedule has a proof with its actual final date |
| missing_table_specification_cannot_invent_two_legs | A completed table without verified legs | Completion is verified | Missing specification blocks archiving |
| unfinished_table_blocks_even_when_a_result_is_present | A final Scheduled or InProgress fixture containing a stale result | Completion is verified | The table remains incomplete |
| duplicate_pair_cannot_replace_a_missing_table_match | The expected total fixture count but a duplicated pairing | Completion is verified | Invalid table shape blocks archiving |
| malformed_fixture_cannot_supply_completion_proof | Completed fixtures with a missing result, bad date, duplicate ID, wrong competition, self-pair or unknown club | Completion is verified | Invalid fixture data blocks archiving |
| empty_edition_cannot_supply_completion_proof | No competitive fixtures | Completion is verified | The edition remains incomplete |
| optional_friendlies_do_not_block_a_completed_table | A complete table and an unplayed friendly | Completion is verified | The competitive proof ignores the friendly |
| terminal_knockout_proof_uses_the_shared_champion_rule | Completed generated cups, including odd fields with byes and a final shootout | Completion is verified | Only the terminal bracket yields its actual champion |
| unfinished_knockout_cannot_be_archived | Played quarterfinals followed by Scheduled semifinals | Completion is verified | The bracket remains incomplete; no edition changes |
| missing_terminal_round_cannot_masquerade_as_a_final | A completed nonterminal round with its successor missing | Completion is verified | Missing terminal progression blocks archiving |
| corrupt_bracket_references_cannot_supply_a_champion | A completed cup with missing/duplicate references, invalid byes, or a forged progression | Completion is verified | Invalid bracket data blocks archiving |
| completed_groups_wait_for_the_terminal_knockout | Completed groups that seed an unfinished bracket | Completion is verified before and after the final | Only the completed terminal knockout yields a proof |
| missing_group_match_cannot_be_hidden_by_a_completed_final | A completed group cup with a missing or duplicated group match | Completion is verified | Invalid group shape blocks archiving |
| stale_standings_block_a_completed_fixture_set | Completed fixtures whose table/group results have not all been applied | Completion is verified | No final table is certified before result application |
| loaded_completion_proof_preserves_the_live_sibling | A JSON-loaded completed half and an InProgress sibling | The completed half is verified repeatedly | Proofs agree and both saved competitions remain byte-equivalent |

## Archive record and persistence

| Named scenario | Given | When | Then |
| --- | --- | --- | --- |
| finished_league_is_archived_with_its_final_table | A finished league with authored legs | Archiving runs | One record holds edition key, ordered final table, champion, clubs and fixtures |
| league_with_a_scheduled_fixture_is_not_archived | A Scheduled competitive fixture | Archiving runs | Nothing is recorded |
| league_without_authored_legs_is_not_archived | A finished table with no authored legs | Archiving runs | Blocked, not guessed |
| finished_knockout_archives_bracket_and_champion | A finished cup | Archiving runs | Bracket and the final's winner are recorded |
| finished_group_cup_archives_groups_and_bracket | A finished group cup | Archiving runs | Groups and bracket are recorded |
| unfinished_knockout_is_not_archived | An unplayed final | Archiving runs | Nothing is recorded |
| archiving_twice_keeps_the_first_record | An archived edition | Archiving again | One unchanged record |
| archive_is_immutable_after_the_live_competition_resets | An archived edition whose live competition then resets | Archiving again | The freeze holds |
| archive_keeps_sibling_halves_separate | Finished Apertura, unplayed Clausura | Archiving runs | Only Apertura recorded; Clausura byte-identical |
| archiving_does_not_mutate_the_competition | Any finished edition | Archiving runs | Competition byte-identical afterwards |
| sweep_archives_only_finished_editions | One finished, one unfinished competition | Every edition is swept | Only the finished one is recorded |
| rollover_freezes_the_finished_edition_before_regenerating_it | A finished edition at the user's rollover | Competitions regenerate | The pre-reset table is archived first |
| editions_survive_sqlite_reload | Non-default editions | Saved and reloaded | Every field survives |
| replacing_competitions_does_not_delete_the_archive | An archived edition | Competitions are replaced | The archive remains |
| a_recorded_edition_is_never_rewritten | A recorded key | A different record claims it | The first wins |
| corrupt_archive_record_is_not_silently_dropped | A malformed record | Loaded | The translated load error, not an empty archive |
| edition_archive_survives_save_load_and_competition_restart | A saved career, competition restarted, saved again | Reloaded | The record is byte-identical |
| edition_archive_v049_upgrades_a_v048_save_without_touching_competitions | A v048 save | Migrated twice | Empty archive table, competitions unchanged |

Not in this slice: player statistics in the record, capture on the live post-result paths (only the
existing rollover captures), and any use of the archive by awards or promotion (P7).
