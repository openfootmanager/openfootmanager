# P2: immutable completed-edition archive (local work)

This fresh branch starts at upstream/develop `8f6659f5`. P1 (#710) is held
behind #668. This first local increment implements only the pure completion
proof needed before archiving. It activates no renewal or rollover change and
adds no persisted field or migration. Table legs are supplied explicitly until
P1's durable calendar specification lands; missing provenance must block.

The validator is shared for AI, user and dormant results. It reads the saved
competition rather than the manager, clock, name, or simulation route. Existing
rollover predicates remain compatibility behavior until the planned cutover;
their permissive cup predicate cannot be used as an archive proof.

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

Remaining P2 work, after P1 integration: the immutable table/bracket/player-stat
record, edition-keyed idempotent receipt, capture at safe post-result boundaries,
and real SQLite normal/replacement/dirty-save/load tests. The full design's
archive scenarios 4, 5, 22, 29 and 33 remain acceptance requirements; this pure
validator increment does not claim those complete lifecycle routes are done.

## Local evidence and integration hold

The clean develop baseline passed all 29 scoped `end_of_season` tests. All 15
new named scenarios failed against the initial `NotVerified` service scaffold
before implementation, then passed with the validator. A mutation returning
success without validation made 11 guard scenarios fail (four positive/read-only
scenarios passed); the real implementation was restored afterward.

Group qualification and knockout bye sizing are extracted from their existing
writers and reused, and final champion selection uses `world_cup_champion`.
No existing public signature/field/variant is removed or renamed. The small
extractions overlap files touched by #704; this branch remains local and must
integrate the landed versions plus P1 before its eventual review/push. P1's
provisional calendar migration becomes v048 only after #668's v047 is merged.
This P2 increment contains no DB changes. CodeRabbit and full PR preflight are
reserved for the completed P2 branch before its first push.

Final local checks for this increment:

- `cargo test --locked --manifest-path src-tauri/Cargo.toml -p ofm_core --lib --jobs 2`: 895 passed, including all 15 new scenarios and the existing group/knockout writer tests.
- `cargo clippy --locked --manifest-path src-tauri/Cargo.toml -p ofm_core --all-targets --jobs 2 -- -D warnings`: passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` and `git diff --check`: passed.

No full workspace/frontend preflight, CodeRabbit run, push or PR opening is
claimed for this unfinished P2 slice.
