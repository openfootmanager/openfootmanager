use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CompetitionType {
    #[default]
    League,
    Cup,
    ContinentalClub,
    InternationalClub,
    InternationalNation,
    FriendlyCup,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CompetitionScope {
    #[default]
    Domestic,
    Regional,
    Continental,
    International,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CompetitionFormat {
    #[default]
    LeagueTable,
    Knockout,
    GroupAndKnockout,
}

/// Smallest supported group; authored values below this are rejected.
pub const MIN_GROUP_SIZE: u32 = 2;

/// Legacy and built-in group size when no authored override is supplied.
pub const DEFAULT_GROUP_SIZE: u32 = 4;

fn default_group_size() -> u32 {
    DEFAULT_GROUP_SIZE
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CompetitionRules {
    pub format: CompetitionFormat,
    pub counts_in_season_flow: bool,
    /// Maximum clubs per snake-seeded group; uneven fields are balanced.
    #[serde(default = "default_group_size")]
    pub group_size: u32,
    /// Group-and-knockout only: clubs advancing from each group.
    pub group_qualifiers_per_group: u32,
    /// Group-and-knockout only: additional best next-placed finishers across
    /// all groups that also advance (the 2026 World Cup's "best thirds").
    pub group_best_third_qualifiers: u32,
    /// Round-robin legs played inside each group.
    pub group_stage_legs: u8,
    /// Days between group-stage matchdays.
    pub group_matchday_gap_days: u32,
    /// Days between knockout rounds.
    pub knockout_round_gap_days: u32,
    /// Maximum fixtures scheduled on the same day within a single knockout round.
    /// Defaults to 1 (each match on its own day). Set higher for large tournaments
    /// like the World Cup where multiple matches happen on the same day.
    #[serde(default = "default_knockout_matches_per_day")]
    pub knockout_matches_per_day: u32,
}

fn default_knockout_matches_per_day() -> u32 {
    1
}

impl Default for CompetitionRules {
    fn default() -> Self {
        Self {
            format: CompetitionFormat::LeagueTable,
            counts_in_season_flow: true,
            group_size: DEFAULT_GROUP_SIZE,
            group_qualifiers_per_group: 2,
            group_best_third_qualifiers: 0,
            group_stage_legs: 2,
            group_matchday_gap_days: 7,
            knockout_round_gap_days: 14,
            knockout_matches_per_day: 1,
        }
    }
}

/// One group of a group-and-knockout competition: a mini league table whose
/// top finishers advance to the knockout rounds.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GroupState {
    pub id: String,
    /// Short label ("A", "B", …); the UI renders it as "Group A".
    pub name: String,
    pub team_ids: Vec<String>,
    pub standings: Vec<StandingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct KnockoutRoundState {
    pub id: String,
    pub name: String,
    pub fixture_ids: Vec<String>,
    /// Teams that advance from this round without playing (byes), used when the
    /// entrant count is not a power of two.
    pub bye_team_ids: Vec<String>,
    pub completed: bool,
}

/// One qualification berth a competition awards into another, based on this
/// season's results (e.g. "league finishers 1–4 enter the Champions Cup").
/// Authored on competition definitions; carried on the runtime competition so
/// it can be evaluated at season rollover.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Berth {
    /// Competition id the qualifying club(s) enter.
    pub target: String,
    /// How the qualifying club(s) are chosen from this competition's results.
    pub rule: BerthRule,
    /// Optional cascade: when a chosen club has already taken a higher-priority
    /// berth, its place passes to this competition instead (e.g. cup winner
    /// already in the Champions Cup → their cup berth drops to the Europa Cup).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_to: Option<String>,
}

/// How a [`Berth`] selects qualifying clubs from the source competition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BerthRule {
    /// League finishers in the inclusive 1-based range `[from, to]`.
    PositionRange { from: u32, to: u32 },
    /// The winner of this (knockout/group-and-knockout) competition.
    CupWinner,
    /// The winner of a playoff contested by league finishers in the inclusive
    /// 1-based range `[from, to]`.
    PlayoffWinner { from: u32, to: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct League {
    pub id: String,
    pub name: String,
    pub kind: CompetitionType,
    pub scope: CompetitionScope,
    pub season: u32,
    pub region_id: Option<String>,
    pub country_id: Option<String>,
    #[serde(default)]
    pub required_region_ids: Vec<String>,
    pub participant_ids: Vec<String>,
    pub rules: CompetitionRules,
    pub fixtures: Vec<Fixture>,
    pub standings: Vec<StandingEntry>,
    #[serde(default)]
    pub groups: Vec<GroupState>,
    pub knockout_rounds: Vec<KnockoutRoundState>,
    #[serde(default)]
    pub transfer_log: Vec<CompletedTransfer>,
    #[serde(default)]
    pub transfer_rumours: Vec<TransferRumour>,
    #[serde(default)]
    pub priority: u32,
    /// Qualification berths this competition awards, evaluated at rollover.
    #[serde(default)]
    pub berths: Vec<Berth>,
    /// Calendar month the season starts (1–12). Stored so rollover can compute
    /// the correct next-season start date without re-reading the definition.
    #[serde(default = "default_season_start_month")]
    pub season_start_month: u8,
    /// Day of month the season starts (1–31).
    #[serde(default = "default_season_start_day")]
    pub season_start_day: u8,
    /// Optional i18n key for the competition name. When set, the frontend
    /// translates via `t(name_key, { year })` instead of displaying `name` raw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_key: Option<String>,
    /// Durable authoring identity and edition provenance. Absent in legacy saves.
    #[serde(default)]
    pub calendar: Option<crate::competition_calendar::CalendarMetadata>,
}

fn default_season_start_month() -> u8 {
    8
}
fn default_season_start_day() -> u8 {
    1
}

impl Default for League {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            kind: CompetitionType::League,
            scope: CompetitionScope::Domestic,
            season: 0,
            region_id: None,
            country_id: None,
            required_region_ids: Vec::new(),
            participant_ids: Vec::new(),
            rules: CompetitionRules::default(),
            fixtures: Vec::new(),
            standings: Vec::new(),
            groups: Vec::new(),
            knockout_rounds: Vec::new(),
            transfer_log: Vec::new(),
            transfer_rumours: Vec::new(),
            priority: 0,
            berths: Vec::new(),
            season_start_month: default_season_start_month(),
            season_start_day: default_season_start_day(),
            name_key: None,
            calendar: None,
        }
    }
}

pub type CompetitionState = League;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletedTransfer {
    pub date: String,
    pub from_team_id: String,
    pub to_team_id: String,
    pub player_id: String,
    pub fee: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferRumour {
    pub id: String,
    pub date: String,
    pub player_id: String,
    pub player_name: String,
    pub team_id: String,
    pub team_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FixtureCompetition {
    #[default]
    League,
    Cup,
    ContinentalClub,
    InternationalClub,
    InternationalNation,
    Friendly,
    FriendlyCup,
    PreseasonTournament,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Fixture {
    pub id: String,
    pub competition_id: String,
    pub matchday: u32,
    pub date: String, // ISO 8601 date
    pub home_team_id: String,
    pub away_team_id: String,
    pub competition: FixtureCompetition,
    pub status: FixtureStatus,
    pub result: Option<MatchResult>,
    /// Set when the match was moved off a date the player had already been shown.
    #[serde(default)]
    pub postponement: Option<Postponement>,
}

/// Written by hand, not derived, to add `competitive` to what the screens receive. The answer is
/// computed from `competition`, not a field; but every serializer of a fixture, including the
/// save's JSON, writes it, and `Deserialize` ignores it on the way back in (a saved fixture loads
/// with or without it). The destructuring below makes a new `Fixture` field a compile error here,
/// so it cannot be dropped from every save unnoticed.
impl Serialize for Fixture {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let Fixture {
            id,
            competition_id,
            matchday,
            date,
            home_team_id,
            away_team_id,
            competition,
            status,
            result,
            postponement,
        } = self;
        let mut fixture = serializer.serialize_struct("Fixture", 11)?;
        fixture.serialize_field("id", id)?;
        fixture.serialize_field("competition_id", competition_id)?;
        fixture.serialize_field("matchday", matchday)?;
        fixture.serialize_field("date", date)?;
        fixture.serialize_field("home_team_id", home_team_id)?;
        fixture.serialize_field("away_team_id", away_team_id)?;
        fixture.serialize_field("competition", competition)?;
        fixture.serialize_field("status", status)?;
        fixture.serialize_field("result", result)?;
        fixture.serialize_field("postponement", postponement)?;
        fixture.serialize_field("competitive", &self.is_competitive())?;
        fixture.end()
    }
}

/// Where a moved match was first scheduled and why it moved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Postponement {
    pub original_date: String,
    pub reason: RescheduleReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RescheduleReason {
    InternationalWindow,
    CupTie,
    ContinentalTie,
    LeagueClash,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FixtureStatus {
    Scheduled,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchResult {
    pub home_goals: u8,
    pub away_goals: u8,
    pub home_scorers: Vec<GoalEvent>,
    pub away_scorers: Vec<GoalEvent>,
    #[serde(default)]
    pub report: Option<CompactMatchReport>,
    /// Penalty-shootout score when a knockout tie is level after extra time.
    /// `None` for matches that never went to a shootout (the default). When set,
    /// it — not the regulation goals — decides who advances.
    #[serde(default)]
    pub home_penalties: Option<u8>,
    #[serde(default)]
    pub away_penalties: Option<u8>,
}

impl MatchResult {
    /// Whether the home side advances from a knockout tie: the penalty
    /// shootout decides it when the tie went to one, otherwise the goals do
    /// (a level result with no shootout still favours home, as before).
    pub fn advancing_is_home(&self) -> bool {
        match (self.home_penalties, self.away_penalties) {
            // A shootout only decides a tie that was level after regulation (and
            // extra time). Guarding on equal goals keeps a malformed or
            // mis-deserialized result — penalties set on a non-level score — from
            // flipping the rightful winner.
            (Some(home), Some(away)) if self.home_goals == self.away_goals => home >= away,
            _ => self.home_goals >= self.away_goals,
        }
    }
}

#[cfg(test)]
mod match_result_tests {
    use super::MatchResult;

    fn result(home: u8, away: u8, pens: Option<(u8, u8)>) -> MatchResult {
        MatchResult {
            home_goals: home,
            away_goals: away,
            home_penalties: pens.map(|(h, _)| h),
            away_penalties: pens.map(|(_, a)| a),
            ..Default::default()
        }
    }

    #[test]
    fn shootout_decides_a_level_knockout_not_the_home_side() {
        // 1-1, away wins the shootout 4-2 → away advances, not home.
        assert!(!result(1, 1, Some((2, 4))).advancing_is_home());
        // 1-1, home wins the shootout → home advances.
        assert!(result(1, 1, Some((5, 4))).advancing_is_home());
        // Decisive in regulation: goals decide, shootout untouched.
        assert!(result(2, 1, None).advancing_is_home());
        assert!(!result(0, 2, None).advancing_is_home());
    }

    #[test]
    fn penalties_only_decide_a_level_score() {
        // Malformed data: penalties present on a decisive 2-1. The goals must
        // win — the (lower) penalty tally cannot flip the rightful winner.
        assert!(result(2, 1, Some((1, 5))).advancing_is_home());
        assert!(!result(1, 2, Some((5, 1))).advancing_is_home());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalEvent {
    pub player_id: String,
    pub minute: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactMatchReport {
    pub total_minutes: u8,
    pub home_stats: CompactTeamMatchStats,
    pub away_stats: CompactTeamMatchStats,
    pub events: Vec<CompactMatchEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactTeamMatchStats {
    pub possession_pct: u8,
    pub shots: u16,
    pub shots_on_target: u16,
    pub fouls: u16,
    pub corners: u16,
    pub yellow_cards: u8,
    pub red_cards: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactMatchEvent {
    pub minute: u8,
    pub event_type: String,
    pub side: String,
    pub player_id: Option<String>,
    pub secondary_player_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandingEntry {
    pub team_id: String,
    pub played: u32,
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
    pub goals_for: u32,
    pub goals_against: u32,
    pub points: u32,
}

impl StandingEntry {
    pub fn new(team_id: String) -> Self {
        Self {
            team_id,
            played: 0,
            won: 0,
            drawn: 0,
            lost: 0,
            goals_for: 0,
            goals_against: 0,
            points: 0,
        }
    }

    pub fn goal_difference(&self) -> i32 {
        self.goals_for as i32 - self.goals_against as i32
    }

    pub fn record_result(&mut self, goals_for: u8, goals_against: u8) {
        self.played += 1;
        self.goals_for += goals_for as u32;
        self.goals_against += goals_against as u32;
        if goals_for > goals_against {
            self.won += 1;
            self.points += 3;
        } else if goals_for == goals_against {
            self.drawn += 1;
            self.points += 1;
        } else {
            self.lost += 1;
        }
    }
}

impl Fixture {
    pub fn counts_for_league_standings(&self) -> bool {
        matches!(self.competition, FixtureCompetition::League)
    }

    /// Whether the match is played for something: everything but a friendly, a friendly cup and a
    /// preseason tournament. The one statement of it (the cup summaries, the completion proof and
    /// the screens all ask it).
    pub fn is_competitive(&self) -> bool {
        !matches!(
            self.competition,
            FixtureCompetition::Friendly
                | FixtureCompetition::FriendlyCup
                | FixtureCompetition::PreseasonTournament
        )
    }

    /// The team advancing from this knockout fixture once a result is
    /// recorded: the winner on goals, or on penalties after a drawn tie.
    pub fn advancing_team_id(&self) -> Option<&str> {
        let result = self.result.as_ref()?;
        Some(if result.advancing_is_home() {
            self.home_team_id.as_str()
        } else {
            self.away_team_id.as_str()
        })
    }

    pub fn generates_match_report_news(&self) -> bool {
        matches!(
            self.competition,
            FixtureCompetition::League
                | FixtureCompetition::Cup
                | FixtureCompetition::ContinentalClub
                | FixtureCompetition::InternationalClub
                | FixtureCompetition::InternationalNation
                | FixtureCompetition::Friendly
                | FixtureCompetition::FriendlyCup
                | FixtureCompetition::PreseasonTournament
        )
    }
}

impl League {
    pub fn new(id: String, name: String, season: u32, team_ids: &[String]) -> Self {
        let standings = team_ids
            .iter()
            .map(|tid| StandingEntry::new(tid.clone()))
            .collect();

        Self {
            id,
            name,
            kind: CompetitionType::League,
            scope: CompetitionScope::Domestic,
            season,
            region_id: None,
            country_id: None,
            required_region_ids: Vec::new(),
            participant_ids: team_ids.to_vec(),
            rules: CompetitionRules::default(),
            fixtures: Vec::new(),
            standings,
            groups: Vec::new(),
            knockout_rounds: Vec::new(),
            transfer_log: Vec::new(),
            transfer_rumours: Vec::new(),
            priority: 0,
            berths: Vec::new(),
            season_start_month: default_season_start_month(),
            season_start_day: default_season_start_day(),
            name_key: None,
            calendar: None,
        }
    }

    /// Whether `fixture_id` belongs to one of this competition's knockout
    /// rounds — a tie that must produce a winner. Knockout pairings are
    /// single-leg (the schedule generator never creates two-legged ties).
    pub fn is_knockout_fixture(&self, fixture_id: &str) -> bool {
        self.knockout_rounds
            .iter()
            .any(|round| round.fixture_ids.iter().any(|id| id == fixture_id))
    }

    pub fn sorted_standings(&self) -> Vec<StandingEntry> {
        sort_standings(self.standings.clone())
    }
}

/// Orders a table: points, then goal difference, then goals scored. The one ordering of a league
/// table, for a live table and for a snapshot of one alike.
pub fn sort_standings(mut standings: Vec<StandingEntry>) -> Vec<StandingEntry> {
    standings.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.goal_difference().cmp(&a.goal_difference()))
            .then(b.goals_for.cmp(&a.goals_for))
    });
    standings
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            id: String::new(),
            competition_id: String::new(),
            matchday: 0,
            date: String::new(),
            home_team_id: String::new(),
            away_team_id: String::new(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            result: None,
            postponement: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given a fixture of each competition kind
    /// When it is asked whether it is competitive
    /// Then a friendly, a friendly cup and a preseason tournament are not, and every other kind is.
    #[test]
    fn only_friendlies_and_preseason_tournaments_are_not_competitive() {
        let kinds = [
            (FixtureCompetition::League, true),
            (FixtureCompetition::Cup, true),
            (FixtureCompetition::ContinentalClub, true),
            (FixtureCompetition::InternationalClub, true),
            (FixtureCompetition::InternationalNation, true),
            (FixtureCompetition::Friendly, false),
            (FixtureCompetition::FriendlyCup, false),
            (FixtureCompetition::PreseasonTournament, false),
        ];
        for (competition, expected) in kinds {
            let kind = format!("{competition:?}");
            let fixture = Fixture {
                competition,
                ..Default::default()
            };
            assert_eq!(fixture.is_competitive(), expected, "{kind}");
        }
    }

    /// Given a friendly cup fixture and a league fixture
    /// When they are serialised for the screens
    /// Then each carries `competitive`, the backend's answer, so no screen decides it again.
    #[test]
    fn a_serialised_fixture_says_whether_it_is_competitive() {
        let friendly_cup = Fixture {
            competition: FixtureCompetition::FriendlyCup,
            ..Default::default()
        };

        assert_eq!(
            serde_json::to_value(&friendly_cup).unwrap()["competitive"],
            false
        );
        assert_eq!(
            serde_json::to_value(Fixture::default()).unwrap()["competitive"],
            true
        );
    }

    /// Given a fixture saved before the field existed, or carrying it
    /// When it is read back
    /// Then it loads either way: the answer is computed, never stored.
    #[test]
    fn a_fixture_loads_with_or_without_the_competitive_field() {
        let written = serde_json::to_string(&Fixture::default()).unwrap();
        let mut old: serde_json::Value = serde_json::from_str(&written).unwrap();
        old.as_object_mut().unwrap().remove("competitive");

        assert!(serde_json::from_str::<Fixture>(&written).is_ok());
        assert!(serde_json::from_value::<Fixture>(old).is_ok());
    }

    #[test]
    fn old_competition_rules_default_to_four_clubs_per_group() {
        let rules: CompetitionRules = serde_json::from_str(
            r#"{"format":"GroupAndKnockout","group_qualifiers_per_group":2,"group_stage_legs":1}"#,
        )
        .unwrap();
        let json = serde_json::to_value(&rules).unwrap();
        assert_eq!(json["group_size"], 4);
        assert_eq!(rules.group_stage_legs, 1);
        assert_eq!(
            serde_json::to_value(CompetitionRules::default()).unwrap()["group_size"],
            4
        );
    }

    fn entry(team: &str, wins: u32, goals_for: u8, goals_against: u8) -> StandingEntry {
        let mut entry = StandingEntry::new(team.to_string());
        for _ in 0..wins {
            entry.record_result(goals_for, goals_against);
        }
        entry
    }

    /// Given a table in no order, when it is sorted, then points decide first, then goal
    /// difference, then goals scored.
    #[test]
    fn a_table_is_ordered_by_points_then_goal_difference_then_goals_scored() {
        let unsorted = vec![
            entry("lowest", 1, 1, 0),
            entry("more_goals", 2, 3, 1),
            entry("best_difference", 2, 4, 1),
            entry("fewer_goals", 2, 2, 0),
        ];

        let sorted: Vec<_> = sort_standings(unsorted)
            .into_iter()
            .map(|e| e.team_id)
            .collect();

        assert_eq!(
            sorted,
            ["best_difference", "more_goals", "fewer_goals", "lowest"]
        );
    }
}

#[cfg(test)]
mod postponement_tests {
    use super::*;
    use serde_json::json;

    /// Given a fixture saved before postponements existed, when it is loaded, then it has none.
    #[test]
    fn a_fixture_saved_before_postponements_loads_without_one() {
        let fixture: Fixture = serde_json::from_value(json!({
            "id": "f1", "competition_id": "c1", "matchday": 3, "date": "2033-03-01",
            "home_team_id": "a", "away_team_id": "b", "competition": "League",
            "status": "Scheduled", "result": null
        }))
        .unwrap();
        assert_eq!(fixture.postponement, None);
    }

    /// Given a postponed fixture, when serialized and loaded, then the original date and reason survive.
    #[test]
    fn a_postponement_round_trips_with_its_original_date_and_reason() {
        let fixture = Fixture {
            postponement: Some(Postponement {
                original_date: "2033-03-01".into(),
                reason: RescheduleReason::CupTie,
            }),
            ..Default::default()
        };
        let loaded: Fixture =
            serde_json::from_value(serde_json::to_value(&fixture).unwrap()).unwrap();
        assert_eq!(loaded.postponement, fixture.postponement);
    }

    /// Given a reason without an original date, when loaded, then it is rejected: the two travel together.
    #[test]
    fn a_reason_without_an_original_date_is_rejected() {
        let loaded = serde_json::from_value::<Fixture>(json!({
            "postponement": { "reason": "CupTie" }
        }));
        assert!(loaded.is_err());
    }
}
