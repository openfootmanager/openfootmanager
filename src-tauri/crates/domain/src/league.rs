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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CompetitionRules {
    pub format: CompetitionFormat,
    pub counts_in_season_flow: bool,
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

fn default_knockout_matches_per_day() -> u32 { 1 }

impl Default for CompetitionRules {
    fn default() -> Self {
        Self {
            format: CompetitionFormat::LeagueTable,
            counts_in_season_flow: true,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let mut sorted = self.standings.clone();
        sorted.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(b.goal_difference().cmp(&a.goal_difference()))
                .then(b.goals_for.cmp(&a.goals_for))
        });
        sorted
    }
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
        }
    }
}
