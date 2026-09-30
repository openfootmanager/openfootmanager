//! What the extra fields on an authored player mean.
//!
//! `package.rs` declares the fields; this says what they resolve to. The contract
//! interval is the part with rules worth having in one place, because two things
//! have to agree about it: generation, which turns it into a player, and
//! validation, which tells the author when it cannot be. Both call
//! [`resolve_authored_contract`], so what counts as a valid interval is decided
//! once.

use super::MIN_OPENING_YEAR;
use super::package::{PackageError, PlayerDef, UNKNOWN_TEAM};
use crate::contracts::{MAX_CONTRACT_YEARS, parse_contract_date};
use chrono::{Months, NaiveDate};
use domain::player::{WEAK_FOOT_MAX, WEAK_FOOT_MIN};
use std::collections::HashSet;

/// The interval an author wrote, resolved against the year the career opens in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct AuthoredContract {
    /// `None` when the author gave none. It is given when the career opens, from
    /// the club's own season, not here: the opening date is not known yet.
    pub start: Option<NaiveDate>,
    /// `None` when the author wrote no end and no length, and generation's own
    /// roll should stand.
    pub end: Option<NaiveDate>,
}

/// Why an authored contract cannot be resolved. Each variant is something the
/// author can fix, and each has its own translated message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContractError {
    /// Not a real `YYYY-MM-DD` date. Carries the JSON field it was in.
    InvalidDate(&'static str),
    /// Both `contractEnd` and `contractLength`, which say the same thing two ways.
    EndAndLength,
    /// A length outside `1..=MAX_CONTRACT_YEARS`.
    LengthOutOfRange,
    /// An end on or before the start.
    EndNotAfterStart,
}

/// A date exactly as it is stored: `YYYY-MM-DD` and nothing looser. The shared
/// parse accepts `2026-7-1`, which is fine for reading a save but would let an
/// author write a date the rest of the tooling does not produce.
fn parse_authored_date(field: &'static str, value: &str) -> Result<NaiveDate, ContractError> {
    if value.len() != 10 {
        return Err(ContractError::InvalidDate(field));
    }
    parse_contract_date(value).ok_or(ContractError::InvalidDate(field))
}

fn add_years(
    start: NaiveDate,
    years: u32,
    field: &'static str,
) -> Result<NaiveDate, ContractError> {
    start
        .checked_add_months(Months::new(years.saturating_mul(12)))
        .ok_or(ContractError::InvalidDate(field))
}

/// Resolve what an author wrote about a player's contract.
///
/// - An end date is kept exactly. It is correct for the period it was written for
///   and nowhere else, which is the author's choice to make.
/// - A length counts whole years from the start when there is one, and otherwise
///   from the year the career opens in, ending on 30 June as every generated
///   contract does. That is what lets one package be played in any era.
/// - A start on its own is half an interval. Rather than invent an end, the length
///   generation would have rolled (`rolled_years`) is counted from it.
/// - Nothing at all is `Ok(AuthoredContract::default())`: generation decides.
pub(super) fn resolve_authored_contract(
    def: &PlayerDef,
    opening_year: u32,
    rolled_years: u32,
) -> Result<AuthoredContract, ContractError> {
    let start = def
        .contract_start
        .as_deref()
        .map(|value| parse_authored_date("contractStart", value))
        .transpose()?;
    let end = def
        .contract_end
        .as_deref()
        .map(|value| parse_authored_date("contractEnd", value))
        .transpose()?;

    if end.is_some() && def.contract_length.is_some() {
        return Err(ContractError::EndAndLength);
    }

    let end = match (end, def.contract_length, start) {
        (Some(end), _, _) => Some(end),
        (None, Some(length), start) => {
            if !(1..=MAX_CONTRACT_YEARS).contains(&length) {
                return Err(ContractError::LengthOutOfRange);
            }
            Some(match start {
                Some(start) => add_years(start, length, "contractStart")?,
                None => NaiveDate::from_ymd_opt(opening_year.saturating_add(length) as i32, 6, 30)
                    .ok_or(ContractError::InvalidDate("contractLength"))?,
            })
        }
        (None, None, Some(start)) => Some(add_years(start, rolled_years, "contractStart")?),
        (None, None, None) => None,
    };

    if let (Some(start), Some(end)) = (start, end)
        && end <= start
    {
        return Err(ContractError::EndNotAfterStart);
    }

    Ok(AuthoredContract { start, end })
}

// One code per message, because `IssueList` renders `t(code, params)` and each has to
// be its own translatable sentence. `field` carries the JSON key the author wrote
// (`weakFoot`): their own identifier, not prose in any language.
const INVALID_DATE: &str = "be.error.package.invalidDate";
const CONTRACT_END_AND_LENGTH: &str = "be.error.package.contractEndAndLength";
const CONTRACT_LENGTH_OUT_OF_RANGE: &str = "be.error.package.contractLengthOutOfRange";
const CONTRACT_END_NOT_AFTER_START: &str = "be.error.package.contractEndNotAfterStart";
const PLAYER_FIELD_OUT_OF_RANGE: &str = "be.error.package.playerFieldOutOfRange";
const ALTERNATE_POSITION_INVALID: &str = "be.error.package.alternatePositionInvalid";
const IDENTITY_NEEDS_SPECIFIC_POSITION: &str = "be.error.package.identityNeedsSpecificPosition";
const CAREER_ENTRY_NEEDS_CLUB: &str = "be.error.package.careerEntryNeedsClub";

/// The largest `value` an author may write: the largest integer a JavaScript number
/// holds exactly (2^53 - 1). The editor would already have rounded anything larger,
/// and it sits comfortably inside the signed 64-bit column the game stores it in.
const MAX_AUTHORED_VALUE: u64 = 9_007_199_254_740_991;

/// Everything wrong with an authored player's contract, status, identity and career
/// fields. All of it is reported, not just the first, so an author fixes their file
/// in one pass.
pub(super) fn authored_player_errors(
    player: &PlayerDef,
    source: &str,
    team_ids: &HashSet<&str>,
) -> Vec<PackageError> {
    let mut errors = Vec::new();
    let error = |code: &str| PackageError::new(code, source).with("entity", &player.id);
    let out_of_range = |field: &str, min: u64, max: u64| {
        error(PLAYER_FIELD_OUT_OF_RANGE)
            .with("field", field)
            .with("min", min.to_string())
            .with("max", max.to_string())
    };

    // Whether an interval is valid depends on where an end would land, never on which
    // year a career opens in or what length generation rolls, so representative
    // values are enough to ask the question the generator answers for real.
    if let Err(problem) = resolve_authored_contract(player, MIN_OPENING_YEAR, 1) {
        let text = |value: &Option<String>| value.clone().unwrap_or_default();
        errors.push(match problem {
            ContractError::InvalidDate(field) => error(INVALID_DATE).with("field", field).with(
                "value",
                match field {
                    "contractStart" => text(&player.contract_start),
                    "contractEnd" => text(&player.contract_end),
                    _ => String::new(),
                },
            ),
            ContractError::EndAndLength => error(CONTRACT_END_AND_LENGTH),
            ContractError::LengthOutOfRange => error(CONTRACT_LENGTH_OUT_OF_RANGE)
                .with(
                    "length",
                    player.contract_length.unwrap_or_default().to_string(),
                )
                .with("max", MAX_CONTRACT_YEARS.to_string()),
            ContractError::EndNotAfterStart => error(CONTRACT_END_NOT_AFTER_START)
                .with("start", text(&player.contract_start))
                .with("end", text(&player.contract_end)),
        });
    }

    if player
        .weak_foot
        .is_some_and(|value| !(WEAK_FOOT_MIN..=WEAK_FOOT_MAX).contains(&value))
    {
        errors.push(out_of_range(
            "weakFoot",
            WEAK_FOOT_MIN.into(),
            WEAK_FOOT_MAX.into(),
        ));
    }
    if player.condition.is_some_and(|value| value > 100) {
        errors.push(out_of_range("condition", 0, 100));
    }
    if player.morale.is_some_and(|value| value > 100) {
        errors.push(out_of_range("morale", 0, 100));
    }
    if player.value.is_some_and(|value| value > MAX_AUTHORED_VALUE) {
        errors.push(out_of_range("value", 0, MAX_AUTHORED_VALUE));
    }

    // A general group (`Midfielder`, `Forward`, …) has its weak foot and alternates
    // re-worked from attributes when a career opens, so anything written against one
    // would be accepted and then replaced. Say so once, rather than also picking at
    // each alternate of a player whose position is the real problem.
    if player.position.is_legacy_bucket() {
        if player.weak_foot.is_some() || !player.alternate_positions.is_empty() {
            errors.push(
                error(IDENTITY_NEEDS_SPECIFIC_POSITION)
                    .with("position", format!("{:?}", player.position)),
            );
        }
    } else {
        let mut seen = HashSet::new();
        for alternate in &player.alternate_positions {
            if alternate.is_legacy_bucket()
                || *alternate == player.position
                || !seen.insert(alternate)
            {
                errors.push(
                    error(ALTERNATE_POSITION_INVALID).with("position", format!("{alternate:?}")),
                );
            }
        }
    }

    for (index, entry) in player.career_history.iter().enumerate() {
        if entry.team_name.trim().is_empty() {
            errors.push(error(CAREER_ENTRY_NEEDS_CLUB).with("row", (index + 1).to_string()));
        }
        // The name is free text, so a club the package does not define is fine. An id
        // is a reference, and is held to the package.
        if let Some(team_id) = entry.team_id.as_deref().filter(|id| !id.is_empty())
            && !team_ids.contains(team_id)
        {
            errors.push(error(UNKNOWN_TEAM).with("team", team_id));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(extra: serde_json::Value) -> PlayerDef {
        let mut base = serde_json::json!({ "id": "p", "name": "P" });
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        serde_json::from_value(base).expect("the fixture deserializes")
    }

    fn day(year: i32, month: u32, date: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, date).unwrap()
    }

    fn resolve(extra: serde_json::Value) -> Result<AuthoredContract, ContractError> {
        resolve_authored_contract(&def(extra), 2026, 3)
    }

    #[test]
    fn nothing_authored_leaves_the_roll_to_generation() {
        assert_eq!(
            resolve(serde_json::json!({})),
            Ok(AuthoredContract::default())
        );
    }

    #[test]
    fn an_end_date_is_kept_exactly() {
        let resolved = resolve(serde_json::json!({ "contractEnd": "2031-03-15" })).unwrap();
        assert_eq!(resolved.end, Some(day(2031, 3, 15)));
        assert_eq!(resolved.start, None);
    }

    #[test]
    fn a_length_with_no_start_ends_on_30_june_of_the_opening_year_plus_length() {
        let resolved = resolve(serde_json::json!({ "contractLength": 3 })).unwrap();
        assert_eq!(resolved.end, Some(day(2029, 6, 30)));
        assert_eq!(resolved.start, None);
    }

    #[test]
    fn a_length_with_a_start_counts_whole_years_from_it() {
        let resolved =
            resolve(serde_json::json!({ "contractStart": "2024-01-15", "contractLength": 2 }))
                .unwrap();
        assert_eq!(resolved.start, Some(day(2024, 1, 15)));
        assert_eq!(resolved.end, Some(day(2026, 1, 15)));
    }

    /// 29 February plus a year has no 29 February to land on. chrono clamps to the
    /// 28th, which is the only sensible answer and worth pinning.
    #[test]
    fn a_leap_day_start_lands_on_the_28th_a_year_later() {
        let resolved =
            resolve(serde_json::json!({ "contractStart": "2024-02-29", "contractLength": 1 }))
                .unwrap();
        assert_eq!(resolved.end, Some(day(2025, 2, 28)));
    }

    #[test]
    fn a_start_alone_rolls_the_generated_length_from_it() {
        // `resolve` passes a rolled length of 3.
        let resolved = resolve(serde_json::json!({ "contractStart": "2024-01-15" })).unwrap();
        assert_eq!(resolved.start, Some(day(2024, 1, 15)));
        assert_eq!(resolved.end, Some(day(2027, 1, 15)));
    }

    #[test]
    fn an_end_and_a_length_together_are_refused() {
        let resolved =
            resolve(serde_json::json!({ "contractEnd": "2030-06-30", "contractLength": 2 }));
        assert_eq!(resolved, Err(ContractError::EndAndLength));
    }

    #[test]
    fn a_length_is_held_to_the_same_limit_as_negotiation() {
        for length in [0, MAX_CONTRACT_YEARS + 1, 100] {
            assert_eq!(
                resolve(serde_json::json!({ "contractLength": length })),
                Err(ContractError::LengthOutOfRange),
                "length {length}"
            );
        }
        for length in 1..=MAX_CONTRACT_YEARS {
            assert!(
                resolve(serde_json::json!({ "contractLength": length })).is_ok(),
                "length {length}"
            );
        }
    }

    #[test]
    fn an_end_on_or_before_the_start_is_refused() {
        for end in ["2025-01-01", "2026-01-01"] {
            let resolved =
                resolve(serde_json::json!({ "contractStart": "2026-01-01", "contractEnd": end }));
            assert_eq!(resolved, Err(ContractError::EndNotAfterStart), "end {end}");
        }
    }

    #[test]
    fn a_date_that_is_not_yyyy_mm_dd_is_refused_and_named() {
        for bad in ["2026-13-45", "2026-7-1", "tomorrow", "", "26-01-01"] {
            assert_eq!(
                resolve(serde_json::json!({ "contractEnd": bad })),
                Err(ContractError::InvalidDate("contractEnd")),
                "contractEnd {bad:?}"
            );
            assert_eq!(
                resolve(serde_json::json!({ "contractStart": bad })),
                Err(ContractError::InvalidDate("contractStart")),
                "contractStart {bad:?}"
            );
        }
    }

    // -- validation, through the real validator -----------------------------
    //
    // Each asserts on the error *code and params*, because those are what the editor
    // and the CLI render. Written against `validate_references` as it stood, so they
    // fail on behaviour: the fields already parse, and nothing yet checks them.

    use super::super::package::{PackageError, WorldPackage, validate_references};

    /// One real team, so a career entry can name a team that exists.
    fn validate(extra: serde_json::Value) -> Vec<PackageError> {
        let mut package = WorldPackage::default();
        package.teams.push(
            serde_json::from_value(serde_json::json!({
                "id": "real-madrid", "name": "Real Madrid", "shortName": "RMA",
                "city": "Madrid", "country": "ENG",
                "colors": { "primary": "#ffffff", "secondary": "#000000" },
            }))
            .expect("the team fixture deserializes"),
        );
        let mut player = serde_json::json!({ "id": "p", "name": "P", "position": "Striker" });
        player
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        package
            .players
            .push(serde_json::from_value(player).expect("the player fixture deserializes"));
        validate_references(&package)
            .into_iter()
            .filter(|error| param(error, "entity") == Some("p"))
            .collect()
    }

    fn param<'a>(error: &'a PackageError, key: &str) -> Option<&'a str> {
        error
            .params
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    fn codes(errors: &[PackageError]) -> Vec<&str> {
        errors.iter().map(|error| error.code.as_str()).collect()
    }

    #[test]
    fn a_fully_authored_player_is_valid() {
        let errors = validate(serde_json::json!({
            "contractStart": "2024-01-15",
            "contractLength": 2,
            "wage": 12_345,
            "value": 9_000_000,
            "weakFoot": 4,
            "alternatePositions": ["LeftWinger"],
            "condition": 64,
            "morale": 51,
            "careerHistory": [
                { "season": 2019, "teamName": "Juventus", "appearances": 30 },
                { "season": 2020, "teamId": "real-madrid", "teamName": "Real Madrid" },
            ],
        }));
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn a_player_with_none_of_the_new_fields_is_still_valid() {
        // The back-compat guarantee: a general-group position with no weak foot and no
        // alternates is what every package written before these fields looks like.
        let errors = validate(serde_json::json!({ "position": "Midfielder" }));
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn an_end_and_a_length_together_are_reported() {
        let errors = validate(serde_json::json!({
            "contractEnd": "2030-06-30", "contractLength": 2,
        }));
        assert_eq!(codes(&errors), ["be.error.package.contractEndAndLength"]);
    }

    #[test]
    fn a_length_outside_the_limit_is_reported_with_the_limit() {
        for length in [0, 6] {
            let errors = validate(serde_json::json!({ "contractLength": length }));
            assert_eq!(
                codes(&errors),
                ["be.error.package.contractLengthOutOfRange"],
                "length {length}"
            );
            assert_eq!(
                param(&errors[0], "length"),
                Some(length.to_string().as_str())
            );
            assert_eq!(param(&errors[0], "max"), Some("5"));
        }
    }

    #[test]
    fn an_end_on_or_before_the_start_is_reported_with_both_dates() {
        let errors = validate(serde_json::json!({
            "contractStart": "2026-01-01", "contractEnd": "2025-01-01",
        }));
        assert_eq!(
            codes(&errors),
            ["be.error.package.contractEndNotAfterStart"]
        );
        assert_eq!(param(&errors[0], "start"), Some("2026-01-01"));
        assert_eq!(param(&errors[0], "end"), Some("2025-01-01"));
    }

    #[test]
    fn a_bad_date_is_reported_with_the_field_it_was_in() {
        for field in ["contractStart", "contractEnd"] {
            let errors = validate(serde_json::json!({ field: "2026-13-45" }));
            assert_eq!(codes(&errors), ["be.error.package.invalidDate"], "{field}");
            assert_eq!(param(&errors[0], "field"), Some(field));
        }
    }

    #[test]
    fn a_number_outside_its_range_is_reported_with_the_range() {
        let cases = [
            ("weakFoot", 0_u64, "1", "5"),
            ("weakFoot", 6, "1", "5"),
            ("condition", 101, "0", "100"),
            ("morale", 101, "0", "100"),
            // One past what a JavaScript number holds exactly: the editor would
            // already have rounded it.
            ("value", 9_007_199_254_740_992, "0", "9007199254740991"),
        ];
        for (field, value, min, max) in cases {
            let errors = validate(serde_json::json!({ field: value }));
            assert_eq!(
                codes(&errors),
                ["be.error.package.playerFieldOutOfRange"],
                "{field} = {value}"
            );
            assert_eq!(param(&errors[0], "field"), Some(field));
            assert_eq!(param(&errors[0], "min"), Some(min));
            assert_eq!(param(&errors[0], "max"), Some(max));
        }
    }

    #[test]
    fn the_edges_of_each_range_are_accepted() {
        for extra in [
            serde_json::json!({ "weakFoot": 1 }),
            serde_json::json!({ "weakFoot": 5 }),
            serde_json::json!({ "condition": 0 }),
            serde_json::json!({ "condition": 100 }),
            serde_json::json!({ "morale": 0 }),
            serde_json::json!({ "value": 0 }),
            serde_json::json!({ "value": 9_007_199_254_740_991_u64 }),
            serde_json::json!({ "wage": 0 }),
            serde_json::json!({ "contractLength": 1 }),
            serde_json::json!({ "contractLength": 5 }),
        ] {
            let errors = validate(extra.clone());
            assert!(errors.is_empty(), "{extra}: {errors:?}");
        }
    }

    #[test]
    fn an_alternate_that_repeats_or_is_the_players_own_or_is_a_group_is_reported() {
        let cases = [
            (
                serde_json::json!(["LeftWinger", "LeftWinger"]),
                "LeftWinger",
            ),
            (serde_json::json!(["Striker"]), "Striker"),
            (serde_json::json!(["Midfielder"]), "Midfielder"),
        ];
        for (alternates, offender) in cases {
            let errors = validate(serde_json::json!({ "alternatePositions": alternates }));
            assert_eq!(
                codes(&errors),
                ["be.error.package.alternatePositionInvalid"],
                "{alternates}"
            );
            assert_eq!(param(&errors[0], "position"), Some(offender));
        }
    }

    /// Goalkeeper, Defender, Midfielder and Forward are re-worked from attributes when
    /// a career opens, so a weak foot or alternates written against one would be
    /// accepted and then silently replaced. The author is told instead.
    #[test]
    fn a_weak_foot_or_alternates_on_a_general_group_position_are_refused() {
        for position in ["Goalkeeper", "Defender", "Midfielder", "Forward"] {
            for extra in [
                serde_json::json!({ "position": position, "weakFoot": 4 }),
                serde_json::json!({ "position": position, "alternatePositions": ["LeftWinger"] }),
            ] {
                let errors = validate(extra.clone());
                assert_eq!(
                    codes(&errors),
                    ["be.error.package.identityNeedsSpecificPosition"],
                    "{extra}"
                );
                assert_eq!(param(&errors[0], "position"), Some(position));
            }
        }
    }

    #[test]
    fn a_player_with_no_position_is_a_goalkeeper_and_is_told_so() {
        // Omitting `position` defaults to Goalkeeper, so a weak foot here is refused
        // for the same reason, and the message names the position the author got.
        let mut package = WorldPackage::default();
        package.players.push(
            serde_json::from_value(serde_json::json!({ "id": "p", "weakFoot": 3 }))
                .expect("the fixture deserializes"),
        );
        let errors: Vec<_> = validate_references(&package)
            .into_iter()
            .filter(|error| param(error, "entity") == Some("p"))
            .collect();
        assert_eq!(
            codes(&errors),
            ["be.error.package.identityNeedsSpecificPosition"]
        );
        assert_eq!(param(&errors[0], "position"), Some("Goalkeeper"));
    }

    #[test]
    fn a_career_entry_needs_a_club_name() {
        let errors = validate(serde_json::json!({
            "careerHistory": [
                { "season": 2019, "teamName": "Juventus" },
                { "season": 2020, "teamName": "  " },
            ],
        }));
        assert_eq!(codes(&errors), ["be.error.package.careerEntryNeedsClub"]);
        assert_eq!(
            param(&errors[0], "row"),
            Some("2"),
            "rows are numbered from 1"
        );
    }

    /// The point of the free-text club: a package that defines only Real Madrid can
    /// still record a spell at Juventus. A `teamId`, when given, is held to the
    /// package, because it is a reference.
    #[test]
    fn a_career_team_id_must_name_a_team_in_the_package_but_a_name_alone_need_not() {
        let name_only = validate(serde_json::json!({
            "careerHistory": [{ "season": 2000, "teamName": "Juventus" }],
        }));
        assert!(name_only.is_empty(), "{name_only:?}");

        let known = validate(serde_json::json!({
            "careerHistory": [{ "season": 2001, "teamId": "real-madrid", "teamName": "Real Madrid" }],
        }));
        assert!(known.is_empty(), "{known:?}");

        let unknown = validate(serde_json::json!({
            "careerHistory": [{ "season": 2002, "teamId": "ghost-fc", "teamName": "Ghost FC" }],
        }));
        assert_eq!(codes(&unknown), ["be.error.package.unknownTeam"]);
        assert_eq!(param(&unknown[0], "team"), Some("ghost-fc"));
    }

    #[test]
    fn every_problem_is_reported_not_just_the_first() {
        let errors = validate(serde_json::json!({
            "contractEnd": "2030-06-30", "contractLength": 2,
            "weakFoot": 9,
            "careerHistory": [{ "season": 2000, "teamName": "" }],
        }));
        let mut found = codes(&errors);
        found.sort();
        assert_eq!(
            found,
            [
                "be.error.package.careerEntryNeedsClub",
                "be.error.package.contractEndAndLength",
                "be.error.package.playerFieldOutOfRange",
            ]
        );
    }
}
