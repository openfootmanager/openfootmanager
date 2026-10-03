# P1: durable competition/calendar identity

As a career player or package author, I need each competition to retain its authored calendar
and edition provenance through construction and save/load, so later lifecycle slices can renew it
on its own calendar.

Refs #654. This first slice records identity and calendar specification; it does not activate
independent renewal, change fixture dates, resolve rest conflicts, or claim the season skip fixed.
Each scenario below maps to a named test, checked with red regressions and targeted mutations. AI and managed
competitions share the definition/resolver and persistence routes. Baseline guard scenarios are
mutation-checked. The approved never-affiliated retirement calendar remains a later P6b concern.

| Named scenario | Given | When | Then |
| --- | --- | --- | --- |
| authored_league_calendar_retains_identity_and_single_leg_shape | An authored one-leg domestic opening phase, family/tier and June 30 window | Explicit competition construction runs | Stable definition ID, edition provenance, legs, existing weekly cadence and declared grouping/window are retained without changing fixtures |
| selector_calendar_uses_the_same_metadata_rule | A selector-authored ordinary league with declared calendar | Definition resolution runs for AI/user clubs | It receives the same metadata as the explicit route |
| generated_calendar_halves_declare_shared_family_and_distinct_windows | Generated Argentine tiers | The world competition plan is built | Each Apertura/Clausura pair declares the same family/tier, distinct roles, June 30/January 31 ends and authored February 1/July 1 starts |
| cup_calendar_keeps_its_opener_without_inventing_league_shape | A March 8 knockout/group cup | Construction runs | Its calendar identity survives, and ordinary league legs/cadence/division are not invented |
| leap_opener_keeps_authored_day_and_existing_clamp | An authored February 29 opener | A non-leap edition is constructed | February 29 remains authored, its existing February 28 fixture-date clamp survives, and no extra edition is generated |
| future_edition_identity_does_not_follow_the_clock | A declared calendar edition ahead of its fixture clock | Metadata is constructed/loaded | The declared edition identity is retained and fixtures are not rewritten |
| calendar_metadata_survives_core_serialization | A non-default explicit calendar | A Game-compatible League is serialized/deserialized | Every metadata field survives |
| missing_calendar_json_remains_backward_compatible | Old League/definition JSON without the new fields | Deserialization runs | Old data loads; no phase grouping is guessed from names or shared rosters |
| invalid_calendar_definition_is_rejected_before_building | Blank family, zero tier or impossible end date | Definition validation runs | A structured occurrence-localized error is returned; no calendar/fixture plan is published |
| sqlite_calendar_metadata_survives_replace_and_reload | A non-default authored calendar | Normal and replacement competition writes reload | Metadata persists with exact identity, legs, windows and unchanged fixture IDs/results |
| legacy_ordinal_uses_verified_opener_provenance | Season counter 5 and competitive fixtures opening August 1, 2030 | Old SQLite metadata is backfilled | Edition basis records counter/year provenance without changing season 5, fixture IDs/dates or results |
| ambiguous_legacy_edition_is_recorded_as_unresolved | A counter with no reliable authored opener or invalid dates | Legacy metadata is backfilled | The basis remains unresolved; today's clock cannot invent a year |
| legacy_same_roster_does_not_invent_phase_identity | Two old leagues sharing clubs/names | Legacy metadata is backfilled | No family/tier/phase is inferred from roster equality or names |
| migrated_calendar_is_resaved_by_its_own_flag | A settled save with only calendar metadata missing | SaveManager loads it | The metadata reaches disk; a second load neither changes it nor rewrites for this backfill |
| corrupt_calendar_metadata_is_not_silently_discarded | Malformed new calendar JSON | SQLite load runs | Load fails through its existing translated error contract rather than pretending metadata never existed |
| generated_annual_calendar_declares_tiers_without_split_phase | Generated English tiers | The world competition plan is built | Each ordinary tier has an explicit family/tier and annual role, keeps August 1 and receives no invented split-season end |
| every_template_offers_every_field_its_definition_serializes | The CLI competition scaffold and the populated definition schema | The existing schema contract test runs | The scaffold exposes the new calendar field rather than hiding it from authors |
| regenerated_edition_keeps_calendar_identity_and_updates_label_basis | An authored edition with legacy counter 5 and verified 2030 provenance | Each existing league/knockout/group-cup regeneration writer stamps counter 6, year 2031 or an unrepresentable label | Its basis reflects the new verified counter/year, calendar year or unresolved state and definition identity, phase/window and original ordinary-table specification survive; existing scheduling policy is unchanged |
| unrepresentable_edition_label_does_not_invent_calendar_provenance | Zero or an unrepresentable season label | Explicit construction runs | The label and fixture clock survive while the edition basis remains unresolved, without inventing a legacy anchor |
| the_annotated_schema_documents_every_field_the_template_scaffolds | The updated competition scaffold | The existing annotated CLI schema contract runs | Its calendar declaration is documented |
| the_schema_reference_documents_every_field_the_template_scaffolds | The updated competition scaffold | The existing package reference contract runs | Calendar fields, defaults and grouping constraints are documented |
| calendar_v048_upgrades_a_seeded_v047_save_without_reinterpreting_it | A v047 database with save format 8, either World Cup draw flag, and competition data | Calendar migration v048 runs and runs again | Schema becomes 48, the calendar column is added once, and format, draw policy and competition identity remain intact |
