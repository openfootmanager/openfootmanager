//! Results of the read-only information tools.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool_results;

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "Stats available".to_string())
}

/// The whole game, as the save format serializes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub game: Value,
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", pretty(&self.game))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeagueSnapshot {
    pub position: usize,
    pub points: u32,
    pub goal_difference: i64,
    /// Last five results, oldest first, joined by "-".
    pub form: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextMatch {
    pub opponent: String,
    pub at_home: bool,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSummary {
    pub date: String,
    pub manager_first_name: String,
    pub manager_last_name: String,
    pub team_name: String,
    pub season_phase: String,
    pub transfer_window: String,
    /// `None` before the season has a league (pre-season).
    pub league: Option<LeagueSnapshot>,
    pub balance: i64,
    pub wage_budget: i64,
    pub transfer_budget: i64,
    pub avg_condition: f64,
    pub avg_ovr: f64,
    pub injured: usize,
    pub squad_size: usize,
    pub next_match: Option<NextMatch>,
    pub unread_messages: usize,
}

impl fmt::Display for GameSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let league = match &self.league {
            Some(league) => format!(
                "**League Position**: {} | **Points**: {} | **GD**: {:+}\n**Form**: {}",
                league.position, league.points, league.goal_difference, league.form
            ),
            None => "**League**: No league yet (pre-season)".to_string(),
        };
        let next = match &self.next_match {
            Some(next) => format!(
                "vs {} ({}) — {}",
                next.opponent,
                if next.at_home { "H" } else { "A" },
                next.date
            ),
            None => "No upcoming match".to_string(),
        };
        write!(
            f,
            "## Game Summary — {}\n\n\
             **Manager**: {} {} | **Team**: {}\n\
             **Season Phase**: {} | **Transfer Window**: {}\n\n\
             ### Position & Form\n{}\n\n\
             ### Finances\n\
             **Balance**: €{} | **Wage Budget**: €{}/wk | **Transfer Budget**: €{}\n\n\
             ### Squad Health\n\
             **Avg Condition**: {:.0}% | **Avg OVR**: {:.0} | **Injured**: {} | **Squad Size**: {}\n\n\
             ### Next Match\n{}\n\n\
             ### Unread Messages: {}",
            self.date,
            self.manager_first_name,
            self.manager_last_name,
            self.team_name,
            self.season_phase,
            self.transfer_window,
            league,
            self.balance,
            self.wage_budget,
            self.transfer_budget,
            self.avg_condition,
            self.avg_ovr,
            self.injured,
            self.squad_size,
            next,
            self.unread_messages,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingRow {
    pub position: usize,
    pub team: String,
    pub is_your_team: bool,
    pub played: u32,
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
    pub goal_difference: i64,
    pub points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standings {
    pub competition: String,
    pub season: u32,
    pub rows: Vec<StandingRow>,
}

impl fmt::Display for Standings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## {} — Season {}\n\n| # | Team | P | W | D | L | GD | Pts |\n|---|------|---|---|---|---|-----|-----|\n",
            self.competition, self.season
        )?;
        for row in &self.rows {
            let team = if row.is_your_team {
                format!("{} ←", row.team)
            } else {
                row.team.clone()
            };
            writeln!(
                f,
                "| {} | {} | {} | {} | {} | {} | {:+} | {} |",
                row.position,
                team,
                row.played,
                row.won,
                row.drawn,
                row.lost,
                row.goal_difference,
                row.points
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GameStatus {
    ManagerFired {},
    AllFixturesCompleted {},
    InProgress { remaining_fixtures: usize },
    NoLeagueYet {},
}

impl fmt::Display for GameStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManagerFired {} => {
                write!(
                    f,
                    "## Game Status: Finished\n\n**Reason**: Manager was fired."
                )
            }
            Self::AllFixturesCompleted {} => {
                write!(
                    f,
                    "## Game Status: Finished\n\n**Reason**: All fixtures completed."
                )
            }
            Self::InProgress { remaining_fixtures } => write!(
                f,
                "## Game Status: In Progress\n\n**Remaining fixtures**: {remaining_fixtures}"
            ),
            Self::NoLeagueYet {} => {
                write!(f, "## Game Status: In Progress\n\nNo league active yet.")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureRow {
    pub date: String,
    /// "<home> - <away>" goals, or `None` for a fixture not yet played.
    pub score: Option<String>,
    pub matchup: String,
    pub matchday: u32,
}

impl FixtureRow {
    fn write_row(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "| {} | {} | {} | MD{} |",
            self.date,
            self.score.as_deref().unwrap_or("-"),
            self.matchup,
            self.matchday
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fixtures {
    pub upcoming: Vec<FixtureRow>,
    /// The last five results, newest first.
    pub recent: Vec<FixtureRow>,
}

impl fmt::Display for Fixtures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const HEADER: &str = "| Date | Score | Match | MD |\n|------|-------|-------|----|\n";
        if !self.upcoming.is_empty() {
            write!(f, "### Upcoming Fixtures\n\n{HEADER}")?;
            for row in &self.upcoming {
                row.write_row(f)?;
            }
        }
        if !self.recent.is_empty() {
            write!(f, "\n### Recent Results (last 5)\n\n{HEADER}")?;
            for row in &self.recent {
                row.write_row(f)?;
            }
        }
        if self.upcoming.is_empty() && self.recent.is_empty() {
            write!(f, "No fixtures found for your team.")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub value: u8,
}

/// What the profile shows depends on whose player it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "visibility", rename_all = "snake_case")]
pub enum PlayerDetail {
    Own {
        ovr: u8,
        condition: u8,
        morale: u8,
        fitness: u8,
        wage: u32,
        contract_end: Option<String>,
        injury: Option<String>,
        attributes: Vec<Attribute>,
    },
    Other {
        ovr: u8,
        /// Shown to the agent as "Form".
        condition: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub id: String,
    pub match_name: String,
    pub full_name: String,
    pub position: String,
    pub age: String,
    pub nationality: String,
    pub team: String,
    pub detail: PlayerDetail,
}

impl fmt::Display for PlayerProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## {} — {}\n\n\
             | Field | Value |\n|-------|-------|\n\
             | ID | {} |\n\
             | Full Name | {} |\n\
             | Position | {} |\n\
             | Age | {} |\n\
             | Nationality | {} |\n\
             | Team | {} |\n",
            self.match_name,
            self.position,
            self.id,
            self.full_name,
            self.position,
            self.age,
            self.nationality,
            self.team,
        )?;
        match &self.detail {
            PlayerDetail::Own {
                ovr,
                condition,
                morale,
                fitness,
                wage,
                contract_end,
                injury,
                attributes,
            } => {
                write!(
                    f,
                    "| OVR | {ovr} |\n| Condition | {condition}% |\n| Morale | {morale}% |\n| Fitness | {fitness}% |\n| Wage | {wage} |\n| Contract End | {} |\n",
                    contract_end.as_deref().unwrap_or("-")
                )?;
                if let Some(injury) = injury {
                    writeln!(f, "| Injury | ⚠️ {injury} |")?;
                }
                write!(
                    f,
                    "\n### Attributes\n\n| Attr | Val | Attr | Val | Attr | Val |\n|------|-----|------|-----|------|-----|\n"
                )?;
                // Three attributes a row; a short last row is left out, as it always was.
                for row in attributes.chunks_exact(3) {
                    let cells: Vec<String> = row
                        .iter()
                        .flat_map(|a| [a.name.clone(), a.value.to_string()])
                        .collect();
                    writeln!(f, "| {} |", cells.join(" | "))?;
                }
                Ok(())
            }
            PlayerDetail::Other { ovr, condition } => write!(
                f,
                "| OVR | {ovr} |\n| Form | {condition} |\n\n*Use `scout_send` for detailed attributes.*"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinanceFigures {
    pub weekly_wage_spend: i64,
    pub weekly_wage_budget: i64,
    pub weekly_recurring_income: i64,
    pub weekly_sponsor_income: i64,
    pub projected_weekly_net: i64,
    pub wage_budget_usage_percent: u32,
    pub cash_runway_weeks: Option<i64>,
    pub in_debt: bool,
    pub over_budget: bool,
    pub wage_budget_status: String,
    pub runway_status: String,
    pub overall_status: String,
}

impl FinanceFigures {
    fn runway(&self) -> String {
        self.cash_runway_weeks
            .map_or_else(|| "N/A".to_string(), |w| format!("{w} weeks"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinancialOverview {
    #[serde(flatten)]
    pub figures: FinanceFigures,
}

impl fmt::Display for FinancialOverview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = &self.figures;
        write!(
            f,
            "## Financial Overview\n\n\
             | Item | Amount |\n|------|--------|\n\
             | Weekly Wage Spend | {} |\n\
             | Weekly Wage Budget | {} |\n\
             | Weekly Recurring Income | {} |\n\
             | Weekly Sponsor Income | {} |\n\
             | Projected Weekly Net | {} |\n\
             | Wage Budget Usage | {}% |\n\
             | Cash Runway | {} |\n\
             | In Debt | {} |\n\
             | Over Budget | {} |\n\
             | Overall Status | {} |",
            s.weekly_wage_spend,
            s.weekly_wage_budget,
            s.weekly_recurring_income,
            s.weekly_sponsor_income,
            s.projected_weekly_net,
            s.wage_budget_usage_percent,
            s.runway(),
            s.in_debt,
            s.over_budget,
            s.overall_status,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailedFinancialSnapshot {
    #[serde(flatten)]
    pub figures: FinanceFigures,
}

impl fmt::Display for DetailedFinancialSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = &self.figures;
        write!(
            f,
            "## Detailed Financial Snapshot\n\n\
             | Metric | Value |\n|--------|-------|\n\
             | Weekly Wage Spend | {} |\n\
             | Weekly Wage Budget | {} |\n\
             | Weekly Recurring Income | {} |\n\
             | Weekly Sponsor Income | {} |\n\
             | Projected Weekly Net | {} |\n\
             | Cash Runway | {} |\n\
             | Wage Budget Usage | {}% |\n\
             | In Debt | {} |\n\
             | Over Budget | {} |\n\
             | Budget Status | {} |\n\
             | Runway Status | {} |\n\
             | Overall Status | {} |",
            s.weekly_wage_spend,
            s.weekly_wage_budget,
            s.weekly_recurring_income,
            s.weekly_sponsor_income,
            s.projected_weekly_net,
            s.runway(),
            s.wage_budget_usage_percent,
            s.in_debt,
            s.over_budget,
            s.wage_budget_status,
            s.runway_status,
            s.overall_status,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeasonContext {
    pub phase: String,
    pub transfer_window_open: bool,
    /// 0 when there is no league.
    pub season: u32,
}

impl fmt::Display for SeasonContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Season Context\n\n\
             | Field | Value |\n|-------|-------|\n\
             | Phase | {} |\n\
             | Transfer Window | {} |\n\
             | Season | {} |",
            self.phase,
            if self.transfer_window_open {
                "Open"
            } else {
                "Closed"
            },
            self.season,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewsItem {
    pub headline: String,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentNews {
    pub articles: Vec<NewsItem>,
}

impl fmt::Display for RecentNews {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.articles.is_empty() {
            return write!(f, "## News\n\nNo recent news.");
        }
        write!(
            f,
            "## Recent News\n\n| # | Headline | Date |\n|---|----------|------|\n"
        )?;
        for (i, article) in self.articles.iter().enumerate() {
            writeln!(f, "| {} | {} | {} |", i + 1, article.headline, article.date)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "fixture", rename_all = "snake_case")]
pub enum MatchPreview {
    NoUpcomingFixtures {},
    Upcoming {
        opponent: String,
        at_home: bool,
        date: String,
        matchday: u32,
        /// 0 when the opponent is not in the table.
        opponent_position: usize,
        /// The opponent's last five results, newest first, space separated.
        opponent_form: String,
    },
}

impl fmt::Display for MatchPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoUpcomingFixtures {} => write!(f, "## Match Preview\n\nNo upcoming fixtures."),
            Self::Upcoming {
                opponent,
                at_home,
                date,
                matchday,
                opponent_position,
                opponent_form,
            } => write!(
                f,
                "## Match Preview\n\n\
                 | Field | Value |\n|-------|-------|\n\
                 | Opponent | {} |\n\
                 | Venue | {} |\n\
                 | Date | {} |\n\
                 | Matchday | {} |\n\
                 | Opponent Position | {} |\n\
                 | Opponent Form | {} |",
                opponent,
                if *at_home { "Home" } else { "Away" },
                date,
                matchday,
                opponent_position,
                opponent_form,
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerStats {
    pub player_name: String,
    pub stats: Value,
}

impl fmt::Display for PlayerStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Player Stats: {}\n\n{}",
            self.player_name,
            pretty(&self.stats)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAppearance {
    pub date: String,
    pub opponent: String,
    pub minutes_played: u8,
    pub goals: u8,
    pub assists: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerMatchHistory {
    pub player_name: String,
    pub matches: Vec<PlayerAppearance>,
}

impl fmt::Display for PlayerMatchHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.matches.is_empty() {
            return write!(
                f,
                "## Match History: {}\n\nNo match data available.",
                self.player_name
            );
        }
        write!(
            f,
            "## Match History: {} ({} matches)\n\n| # | Date | Opponent | Mins | Goals | Assists |\n|---|------|----------|--------|-------|--------|\n",
            self.player_name,
            self.matches.len()
        )?;
        for (i, m) in self.matches.iter().enumerate() {
            writeln!(
                f,
                "| {} | {} | {} | {} | {} | {} |",
                i + 1,
                m.date,
                m.opponent,
                m.minutes_played,
                m.goals,
                m.assists
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamLeagueFigures {
    pub position: usize,
    pub points: u32,
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
    pub goal_difference: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamFinances {
    pub weekly_wage_spend: i64,
    pub weekly_wage_budget: i64,
    pub projected_weekly_net: i64,
    pub in_debt: bool,
    pub overall_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamProfile {
    pub id: String,
    pub name: String,
    pub formation: String,
    pub play_style: String,
    pub squad_size: usize,
    pub training_focus: String,
    pub training_intensity: String,
    pub league: Option<TeamLeagueFigures>,
    /// Last five results, newest first, space separated; `None` when none were played.
    pub recent_form: Option<String>,
    /// Only for the manager's own team.
    pub finances: Option<TeamFinances>,
}

impl fmt::Display for TeamProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## {} — Team Profile\n\n\
             | Field | Value |\n|-------|-------|\n\
             | ID | {} |\n\
             | Formation | {} |\n\
             | Play Style | {} |\n\
             | Squad Size | {} |\n\
             | Training | {} / {} |\n",
            self.name,
            self.id,
            self.formation,
            self.play_style,
            self.squad_size,
            self.training_focus,
            self.training_intensity,
        )?;
        if let Some(league) = &self.league {
            write!(
                f,
                "| League Position | {} |\n| Points | {} ({}/{}/{}) |\n| Goal Difference | {:+} |\n",
                league.position,
                league.points,
                league.won,
                league.drawn,
                league.lost,
                league.goal_difference
            )?;
        }
        if let Some(form) = &self.recent_form {
            writeln!(f, "| Recent Form | {form} |")?;
        }
        if let Some(finances) = &self.finances {
            write!(
                f,
                "\n### Finances\n\n\
                 | Item | Value |\n|------|-------|\n\
                 | Weekly Wage Spend | {} |\n\
                 | Weekly Wage Budget | {} |\n\
                 | Projected Weekly Net | {} |\n\
                 | In Debt | {} |\n\
                 | Overall Status | {} |",
                finances.weekly_wage_spend,
                finances.weekly_wage_budget,
                finances.projected_weekly_net,
                finances.in_debt,
                finances.overall_status,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamStats {
    pub team_name: String,
    /// `None` before the team has played.
    pub stats: Option<Value>,
}

impl fmt::Display for TeamStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.stats {
            Some(stats) => write!(f, "## Team Stats: {}\n\n{}", self.team_name, pretty(stats)),
            None => write!(
                f,
                "## Team Stats: {}\n\nNo stats available yet.",
                self.team_name
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamResult {
    pub date: String,
    pub opponent: String,
    pub goals_for: u8,
    pub goals_against: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamMatchHistory {
    pub team_name: String,
    pub matches: Vec<TeamResult>,
}

impl fmt::Display for TeamMatchHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.matches.is_empty() {
            return write!(
                f,
                "## Match History: {}\n\nNo match data available.",
                self.team_name
            );
        }
        write!(
            f,
            "## Match History: {} ({} matches)\n\n| # | Date | Opponent | Score |\n|---|------|----------|-------|\n",
            self.team_name,
            self.matches.len()
        )?;
        for (i, m) in self.matches.iter().enumerate() {
            writeln!(
                f,
                "| {} | {} | {} | {}-{} |",
                i + 1,
                m.date,
                m.opponent,
                m.goals_for,
                m.goals_against
            )?;
        }
        Ok(())
    }
}

tool_results!(
    GameState,
    GameSummary,
    Standings,
    GameStatus,
    Fixtures,
    PlayerProfile,
    FinancialOverview,
    DetailedFinancialSnapshot,
    SeasonContext,
    RecentNews,
    MatchPreview,
    PlayerStats,
    PlayerMatchHistory,
    TeamProfile,
    TeamStats,
    TeamMatchHistory,
);
