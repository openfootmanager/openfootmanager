use domain::league::FixtureCompetition;
use ofm_core::state::StateManager;

pub(super) fn competition_label(competition: &FixtureCompetition) -> String {
    match competition {
        FixtureCompetition::League => "League".to_string(),
        FixtureCompetition::Cup => "Cup".to_string(),
        FixtureCompetition::ContinentalClub => "ContinentalClub".to_string(),
        FixtureCompetition::InternationalClub => "InternationalClub".to_string(),
        FixtureCompetition::InternationalNation => "InternationalNation".to_string(),
        FixtureCompetition::Friendly => "Friendly".to_string(),
        FixtureCompetition::FriendlyCup => "FriendlyCup".to_string(),
        FixtureCompetition::PreseasonTournament => "PreseasonTournament".to_string(),
    }
}

pub(super) fn round_to(value: f32, digits: i32) -> f32 {
    let factor = 10_f32.powi(digits);
    (value * factor).round() / factor
}

pub(super) fn calculate_per90(total: u32, minutes_played: u32) -> Option<f32> {
    if minutes_played == 0 {
        return None;
    }

    Some(round_to((total as f32 * 90.0) / minutes_played as f32, 1))
}

pub(super) fn calculate_pass_accuracy(completed: u32, attempted: u32) -> Option<f32> {
    if attempted == 0 {
        return None;
    }

    Some(round_to((completed as f32 / attempted as f32) * 100.0, 1))
}

pub(super) fn calculate_average(total: u32, count: u32) -> Option<f32> {
    if count == 0 {
        return None;
    }

    Some(round_to(total as f32 / count as f32, 1))
}

/// Whether a bigger or a smaller value of a stat is the better one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StatDirection {
    HigherIsBetter,
    LowerIsBetter,
}

/// The advanced stats that carry a percentile, each with the one definition of which way is
/// better. A high percentile always means the player does well, so the frontend never decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AdvancedStat {
    Shots,
    ShotsOnTarget,
    PassAccuracy,
    TacklesWon,
    Interceptions,
    FoulsCommitted,
}

impl AdvancedStat {
    #[cfg(test)]
    pub(super) const ALL: [AdvancedStat; 6] = [
        AdvancedStat::Shots,
        AdvancedStat::ShotsOnTarget,
        AdvancedStat::PassAccuracy,
        AdvancedStat::TacklesWon,
        AdvancedStat::Interceptions,
        AdvancedStat::FoulsCommitted,
    ];

    pub(super) fn direction(self) -> StatDirection {
        match self {
            AdvancedStat::FoulsCommitted => StatDirection::LowerIsBetter,
            AdvancedStat::Shots
            | AdvancedStat::ShotsOnTarget
            | AdvancedStat::PassAccuracy
            | AdvancedStat::TacklesWon
            | AdvancedStat::Interceptions => StatDirection::HigherIsBetter,
        }
    }
}

/// The share of peers the player does at least as well as, in the stat's own direction, as a
/// whole percentage.
pub(super) fn percentile_rank(
    values: &[f32],
    target: Option<f32>,
    direction: StatDirection,
) -> Option<u32> {
    let target = target?;
    if values.is_empty() {
        return None;
    }

    let as_good_or_worse = values
        .iter()
        .filter(|value| match direction {
            StatDirection::HigherIsBetter => **value <= target,
            StatDirection::LowerIsBetter => **value >= target,
        })
        .count();
    Some(((as_good_or_worse as f32 / values.len() as f32) * 100.0).round() as u32)
}

pub(super) fn opponent_name(state: &StateManager, opponent_team_id: &str) -> String {
    state
        .get_game(|game| {
            game.teams
                .iter()
                .find(|team| team.id == opponent_team_id)
                .map(|team| team.name.clone())
        })
        .flatten()
        .unwrap_or_else(|| opponent_team_id.to_string())
}

pub(super) fn ensure_team_exists(state: &StateManager, team_id: &str) -> Result<(), String> {
    let team_exists = state
        .get_game(|game| game.teams.iter().any(|team| team.id == team_id))
        .ok_or("be.error.noActiveGameSession".to_string())?;

    if team_exists {
        Ok(())
    } else {
        Err("be.error.teamNotFound".to_string())
    }
}
