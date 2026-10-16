use std::collections::{HashMap, HashSet};

use domain::player::{Player, PlayerSeasonStats, Position};
use domain::stats::PlayerMatchStatsRecord;
use ofm_core::game::Game;
use ofm_core::state::StateManager;

use super::dto::{
    PeerGroupDto, PlayerAdvancedMetricDto, PlayerAdvancedPassMetricDto, PlayerMatchHistoryEntryDto,
    PlayerStatsOverviewDto, PlayerStatsOverviewMetricsDto,
};
use super::shared::{
    calculate_pass_accuracy, calculate_per90, competition_label, opponent_name, percentile_rank,
    AdvancedStat,
};

const DEFAULT_MINIMUM_MINUTES: u32 = 180;
const DEFAULT_MINIMUM_COHORT_SIZE: usize = 3;

#[derive(Debug, Clone, Default)]
struct PlayerAggregate {
    minutes_played: u32,
    shots: u32,
    shots_on_target: u32,
    passes_completed: u32,
    passes_attempted: u32,
    tackles_won: u32,
    interceptions: u32,
    fouls_committed: u32,
}

fn position_key(player: &Player) -> &Position {
    &player.natural_position
}

/// A player on the roster, or a retiree kept in the archive: both have a history page. A
/// retiree has no club, so he has totals and rates but no peer group and no percentiles.
fn find_player_or_retiree(game: &Game, player_id: &str) -> Option<Player> {
    game.players
        .iter()
        .find(|candidate| candidate.id == player_id)
        .cloned()
        .or_else(|| ofm_core::retirees::find_retiree(game, player_id))
}

/// The players a percentile compares with: those at clubs in the same league table who share the
/// player's natural position. A player with no club or no league table has no group, and so no
/// percentiles, rather than being ranked against strangers.
fn league_peers(game: &Game, player: &Player) -> (Option<PeerGroupDto>, HashSet<String>) {
    let own_only = || HashSet::from([player.id.clone()]);
    let Some(table) = player
        .team_id
        .as_deref()
        .and_then(|club| ofm_core::end_of_season::club_league_table(game, club))
    else {
        return (None, own_only());
    };

    let clubs: HashSet<&str> = table
        .participant_ids
        .iter()
        .map(String::as_str)
        .chain(table.standings.iter().map(|entry| entry.team_id.as_str()))
        .collect();
    let position = position_key(player);
    let peer_ids = game
        .players
        .iter()
        .filter(|candidate| {
            position_key(candidate) == position
                && candidate
                    .team_id
                    .as_deref()
                    .is_some_and(|club| clubs.contains(club))
        })
        .map(|candidate| candidate.id.clone())
        .collect();
    let group = PeerGroupDto {
        competition_id: table.id.clone(),
        competition_name: table.name.clone(),
        competition_name_key: table.name_key.clone(),
        country_id: table.country_id.clone(),
        // The serialized name, the same one `PlayerData.natural_position` sends.
        position: serde_json::to_value(position)
            .ok()
            .and_then(|name| name.as_str().map(str::to_owned))
            .unwrap_or_default(),
    };
    (Some(group), peer_ids)
}

fn aggregate_from_history(records: &[PlayerMatchStatsRecord]) -> Option<PlayerAggregate> {
    if records.is_empty() {
        return None;
    }

    let mut aggregate = PlayerAggregate::default();
    for record in records {
        aggregate.minutes_played += record.minutes_played as u32;
        aggregate.shots += record.shots as u32;
        aggregate.shots_on_target += record.shots_on_target as u32;
        aggregate.passes_completed += record.passes_completed as u32;
        aggregate.passes_attempted += record.passes_attempted as u32;
        aggregate.tackles_won += record.tackles_won as u32;
        aggregate.interceptions += record.interceptions as u32;
        aggregate.fouls_committed += record.fouls_committed as u32;
    }

    Some(aggregate)
}

fn aggregate_from_season_stats(stats: &PlayerSeasonStats) -> PlayerAggregate {
    PlayerAggregate {
        minutes_played: stats.minutes_played,
        shots: stats.shots,
        shots_on_target: stats.shots_on_target,
        passes_completed: stats.passes_completed,
        passes_attempted: stats.passes_attempted,
        tackles_won: stats.tackles_won,
        interceptions: stats.interceptions,
        fouls_committed: stats.fouls_committed,
    }
}

fn metric_percentile<F>(
    stat: AdvancedStat,
    peers: &[&PlayerAggregate],
    selector: F,
    player_aggregate: &PlayerAggregate,
) -> Option<u32>
where
    F: Fn(&PlayerAggregate) -> Option<f32>,
{
    let values = peers
        .iter()
        .filter_map(|aggregate| selector(aggregate))
        .collect::<Vec<_>>();

    percentile_rank(&values, selector(player_aggregate), stat.direction())
}

/// The player and the peers they are ranked against, so each metric is built the same way.
struct Cohort<'a> {
    player: &'a PlayerAggregate,
    eligible_peers: Vec<&'a PlayerAggregate>,
    percentiles_computable: bool,
}

impl Cohort<'_> {
    fn percentile(
        &self,
        stat: AdvancedStat,
        rate: impl Fn(&PlayerAggregate) -> Option<f32>,
    ) -> Option<u32> {
        if !self.percentiles_computable {
            return None;
        }
        metric_percentile(stat, &self.eligible_peers, rate, self.player)
    }

    fn per90_metric(
        &self,
        stat: AdvancedStat,
        total: fn(&PlayerAggregate) -> u32,
    ) -> PlayerAdvancedMetricDto {
        let per90 = |aggregate: &PlayerAggregate| {
            calculate_per90(total(aggregate), aggregate.minutes_played)
        };
        PlayerAdvancedMetricDto {
            total: total(self.player),
            per90: per90(self.player),
            percentile: self.percentile(stat, per90),
        }
    }

    fn pass_metric(&self) -> PlayerAdvancedPassMetricDto {
        let accuracy = |aggregate: &PlayerAggregate| {
            calculate_pass_accuracy(aggregate.passes_completed, aggregate.passes_attempted)
        };
        PlayerAdvancedPassMetricDto {
            completed: self.player.passes_completed,
            attempted: self.player.passes_attempted,
            accuracy: accuracy(self.player),
            percentile: self.percentile(AdvancedStat::PassAccuracy, accuracy),
        }
    }
}

fn build_overview_from_aggregate(
    player_aggregate: &PlayerAggregate,
    peers: &[PlayerAggregate],
    season_avg_rating: f32,
    peer_group: Option<PeerGroupDto>,
) -> PlayerStatsOverviewDto {
    let eligible_peers = peers
        .iter()
        .filter(|aggregate| aggregate.minutes_played >= DEFAULT_MINIMUM_MINUTES)
        .collect::<Vec<_>>();
    let cohort = Cohort {
        player: player_aggregate,
        percentiles_computable: player_aggregate.minutes_played >= DEFAULT_MINIMUM_MINUTES
            && eligible_peers.len() >= DEFAULT_MINIMUM_COHORT_SIZE,
        eligible_peers,
    };

    PlayerStatsOverviewDto {
        percentile_eligible: cohort.percentiles_computable,
        peer_group,
        season_rating_rated: ofm_core::match_rating::is_rated(season_avg_rating),
        metrics: PlayerStatsOverviewMetricsDto {
            shots: cohort.per90_metric(AdvancedStat::Shots, |a| a.shots),
            shots_on_target: cohort
                .per90_metric(AdvancedStat::ShotsOnTarget, |a| a.shots_on_target),
            passes: cohort.pass_metric(),
            tackles_won: cohort.per90_metric(AdvancedStat::TacklesWon, |a| a.tackles_won),
            interceptions: cohort.per90_metric(AdvancedStat::Interceptions, |a| a.interceptions),
            fouls_committed: cohort
                .per90_metric(AdvancedStat::FoulsCommitted, |a| a.fouls_committed),
        },
    }
}

fn build_history_overview(
    state: &StateManager,
    player_id: &str,
) -> Result<Option<PlayerStatsOverviewDto>, String> {
    let game = state
        .get_game(|game| game.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;
    let Some(player) = find_player_or_retiree(&game, player_id) else {
        return Err("be.error.playerNotFound".to_string());
    };
    let (peer_group, peer_ids) = league_peers(&game, &player);

    let Some(history_aggregates) = state.get_stats_state(|stats| {
        let mut records_by_player: HashMap<String, Vec<PlayerMatchStatsRecord>> = HashMap::new();

        for record in &stats.player_matches {
            if peer_ids.contains(&record.player_id) {
                records_by_player
                    .entry(record.player_id.clone())
                    .or_default()
                    .push(record.clone());
            }
        }

        records_by_player
            .into_iter()
            .filter_map(|(candidate_id, records)| {
                aggregate_from_history(&records).map(|aggregate| (candidate_id, aggregate))
            })
            .collect::<HashMap<_, _>>()
    }) else {
        return Ok(None);
    };

    let Some(player_aggregate) = history_aggregates.get(player_id) else {
        return Ok(None);
    };

    let peers = peer_ids
        .iter()
        .filter_map(|candidate_id| history_aggregates.get(candidate_id).cloned())
        .collect::<Vec<_>>();

    Ok(Some(build_overview_from_aggregate(
        player_aggregate,
        &peers,
        player.stats.avg_rating,
        peer_group,
    )))
}

fn build_legacy_overview(
    state: &StateManager,
    player_id: &str,
) -> Result<PlayerStatsOverviewDto, String> {
    let game = state
        .get_game(|game| game.clone())
        .ok_or("be.error.noActiveGameSession".to_string())?;
    let Some(player) = find_player_or_retiree(&game, player_id) else {
        return Err("be.error.playerNotFound".to_string());
    };
    let (peer_group, peer_ids) = league_peers(&game, &player);
    let peers = game
        .players
        .iter()
        .filter(|candidate| peer_ids.contains(&candidate.id))
        .map(|candidate| aggregate_from_season_stats(&candidate.stats))
        .collect::<Vec<_>>();

    Ok(build_overview_from_aggregate(
        &aggregate_from_season_stats(&player.stats),
        &peers,
        player.stats.avg_rating,
        peer_group,
    ))
}

fn to_dto(state: &StateManager, record: &PlayerMatchStatsRecord) -> PlayerMatchHistoryEntryDto {
    let team_goals = if record.team_id == record.home_team_id {
        record.home_goals
    } else {
        record.away_goals
    };
    let opponent_goals = if record.team_id == record.home_team_id {
        record.away_goals
    } else {
        record.home_goals
    };

    PlayerMatchHistoryEntryDto {
        fixture_id: record.fixture_id.clone(),
        date: record.date.clone(),
        competition: competition_label(&record.competition),
        matchday: record.matchday,
        opponent_team_id: record.opponent_team_id.clone(),
        opponent_name: opponent_name(state, &record.opponent_team_id),
        team_goals,
        opponent_goals,
        minutes_played: record.minutes_played,
        goals: record.goals,
        assists: record.assists,
        shots: record.shots,
        shots_on_target: record.shots_on_target,
        passes_completed: record.passes_completed,
        passes_attempted: record.passes_attempted,
        tackles_won: record.tackles_won,
        interceptions: record.interceptions,
        fouls_committed: record.fouls_committed,
        yellow_cards: record.yellow_cards,
        red_cards: record.red_cards,
        rating: record.rating,
        rated: ofm_core::match_rating::is_rated(record.rating),
    }
}

pub fn get_player_match_history_internal(
    state: &StateManager,
    player_id: &str,
    limit: Option<usize>,
) -> Result<Vec<PlayerMatchHistoryEntryDto>, String> {
    let Some(mut history) = state.get_stats_state(|stats| {
        stats
            .player_matches
            .iter()
            .filter(|record| record.player_id == player_id)
            .cloned()
            .collect::<Vec<_>>()
    }) else {
        return Ok(Vec::new());
    };

    history.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then(right.matchday.cmp(&left.matchday))
            .then(right.fixture_id.cmp(&left.fixture_id))
    });

    let limit = limit.unwrap_or(5);
    Ok(history
        .into_iter()
        .take(limit)
        .map(|record| to_dto(state, &record))
        .collect())
}

pub fn get_player_stats_overview_internal(
    state: &StateManager,
    player_id: &str,
) -> Result<PlayerStatsOverviewDto, String> {
    if let Some(overview) = build_history_overview(state, player_id)? {
        return Ok(overview);
    }

    build_legacy_overview(state, player_id)
}
