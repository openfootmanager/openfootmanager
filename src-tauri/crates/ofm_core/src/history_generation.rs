use crate::seed::{rng_for_seed, standard_normal};
use crate::{game::Game, season_awards::SeasonAwards};
use chrono::{TimeZone, Utc};
use domain::{
    league::{CompetitionScope, CompetitionType, League, StandingEntry},
    manager::ManagerCareerEntry,
    player::{CareerEntry, Player, PlayerSeasonStats, Position},
    team::TeamSeasonRecord,
    world_history::{
        HistoricalManagerAwardWinner, HistoricalPlayerAwardWinner, HistoricalSeasonAwardsRecord,
    },
};
use rand::{Rng, RngExt};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const DEFAULT_HISTORY_LEAGUE_ID: &str = "history-league";
const DEFAULT_HISTORY_LEAGUE_NAME: &str = "Historical League";

fn deterministic_value(parts: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    parts.hash(&mut hasher);
    hasher.finish()
}

fn deterministic_u32(parts: impl Hash, modulo: u32) -> u32 {
    if modulo == 0 {
        0
    } else {
        (deterministic_value(parts) % modulo as u64) as u32
    }
}

fn season_start_string(season: u32) -> String {
    format!("{:04}-07-01", season)
}

/// The clubs of one domestic league, under the id its stream of randomness is keyed on.
struct LeagueGroup {
    key: String,
    team_ids: Vec<String>,
}

/// Partition the world's teams into the leagues they actually play in, so a
/// historical season reflects a real ~20-team league rather than treating all
/// 440 clubs as one mega-league (which produced ~878-match seasons). Falls back
/// to a single all-teams group for legacy single-competition worlds. A club is in one group
/// only: a second competition over the same clubs (an Apertura and its Clausura) is not a second
/// table, because a club has one record per season.
fn historical_league_groups(game: &Game) -> Vec<LeagueGroup> {
    let valid: std::collections::HashSet<&str> =
        game.teams.iter().map(|team| team.id.as_str()).collect();
    let mut groups: Vec<LeagueGroup> = Vec::new();
    let mut covered: std::collections::HashSet<String> = std::collections::HashSet::new();

    for competition in &game.competitions {
        if competition.kind != CompetitionType::League
            || competition.scope != CompetitionScope::Domestic
        {
            continue;
        }
        let candidate_ids: Vec<&String> = if competition.participant_ids.is_empty() {
            competition
                .standings
                .iter()
                .map(|entry| &entry.team_id)
                .collect()
        } else {
            competition.participant_ids.iter().collect()
        };
        let members: Vec<String> = candidate_ids
            .into_iter()
            .filter(|id| valid.contains(id.as_str()) && !covered.contains(*id))
            .cloned()
            .collect();
        if members.len() >= 2 {
            for id in &members {
                covered.insert(id.clone());
            }
            groups.push(LeagueGroup {
                key: competition.id.clone(),
                team_ids: members,
            });
        }
    }

    // Teams not assigned to a domestic league still get a history entry.
    let residual: Vec<String> = game
        .teams
        .iter()
        .map(|team| team.id.clone())
        .filter(|id| !covered.contains(id))
        .collect();
    if !residual.is_empty() {
        groups.push(LeagueGroup {
            key: "residual".to_string(),
            team_ids: residual,
        });
    }

    groups
}

// The one parameter set the past is tuned with, against real leagues: over twelve to twenty
// seasons a top flight sees 3 to 7 champions, its favourite (the best-reputation club) wins
// 30 to 50% of the titles, and the champion's points run 75 to 100 with a real spread.
//
/// How many goals of log-ratio a standard deviation of reputation is worth: the pull of
/// reputation on a club's strength.
const STRENGTH_PER_REPUTATION_SD: f64 = 0.5;
/// The spread of a club's form around what its reputation says.
const FORM_SD: f64 = 0.2;
/// How much of a club's form carries into the next season: dynasties form and break.
const FORM_PERSISTENCE: f64 = 0.7;
/// The chance in a season that a club's form is knocked by something (a manager, a takeover, an
/// injury crisis), and how hard.
const SHOCK_CHANCE: f64 = 0.03;
const SHOCK_SD: f64 = 0.4;
/// Goals a home and an away side score when the clubs are level: the home edge.
const HOME_GOALS: f64 = 1.50;
const AWAY_GOALS: f64 = 1.15;

/// A Poisson draw of mean `mean` (Knuth): the goals a side scores.
fn poisson(rng: &mut impl Rng, mean: f64) -> u32 {
    let limit = (-mean).exp();
    let mut goals = 0;
    let mut product: f64 = rng.random_range(0.0..1.0);
    while product > limit {
        goals += 1;
        product *= rng.random_range(0.0..1.0);
    }
    goals
}

/// A club's strength this season, in log-goals: what its reputation says, relative to its league,
/// plus its form.
fn strengths_of(reputations: &[f64], form: &[f64]) -> Vec<f64> {
    let count = reputations.len() as f64;
    let mean = reputations.iter().sum::<f64>() / count;
    let spread = (reputations.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / count).sqrt();
    reputations
        .iter()
        .zip(form)
        .map(|(reputation, form)| {
            let standard = if spread > 0.0 {
                (reputation - mean) / spread
            } else {
                0.0
            };
            STRENGTH_PER_REPUTATION_SD * standard + form
        })
        .collect()
}

/// Plays every fixture of a double round robin among clubs of these `strengths` and returns
/// the table, best first.
fn play_season(team_ids: &[String], strengths: &[f64], rng: &mut impl Rng) -> Vec<StandingEntry> {
    let mut table: Vec<StandingEntry> = team_ids
        .iter()
        .map(|id| StandingEntry::new(id.clone()))
        .collect();
    for home in 0..table.len() {
        for away in 0..table.len() {
            if home == away {
                continue;
            }
            let edge = strengths[home] - strengths[away];
            let home_goals = poisson(rng, HOME_GOALS * (edge / 2.0).exp());
            let away_goals = poisson(rng, AWAY_GOALS * (-edge / 2.0).exp());
            record_result(&mut table[home], home_goals, away_goals);
            record_result(&mut table[away], away_goals, home_goals);
        }
    }
    order_table(&mut table);
    table
}

/// Best first: points, then goal difference, then goals scored, then the club's id so the order
/// is the same on every run.
fn order_table(table: &mut [StandingEntry]) {
    table.sort_by(|left, right| {
        right
            .points
            .cmp(&left.points)
            .then_with(|| goal_difference(right).cmp(&goal_difference(left)))
            .then_with(|| right.goals_for.cmp(&left.goals_for))
            .then_with(|| left.team_id.cmp(&right.team_id))
    });
}

fn goal_difference(entry: &StandingEntry) -> i64 {
    i64::from(entry.goals_for) - i64::from(entry.goals_against)
}

fn record_result(entry: &mut StandingEntry, scored: u32, conceded: u32) {
    entry.played += 1;
    entry.goals_for += scored;
    entry.goals_against += conceded;
    match scored.cmp(&conceded) {
        std::cmp::Ordering::Greater => {
            entry.won += 1;
            entry.points += 3;
        }
        std::cmp::Ordering::Equal => {
            entry.drawn += 1;
            entry.points += 1;
        }
        std::cmp::Ordering::Less => entry.lost += 1,
    }
}

/// Moves every club's form on one season: part of last season's carries over (so a good run
/// lasts, and a dynasty can break), the rest is new, and now and then something knocks it
/// (a manager, a takeover, an injury crisis). The spread of a club's form stays `form_sd` (plus
/// the shocks).
fn advance_form(form: &mut [f64], form_sd: f64, rng: &mut impl Rng) {
    let carried = (1.0 - FORM_PERSISTENCE * FORM_PERSISTENCE).sqrt();
    for club_form in form.iter_mut() {
        *club_form = FORM_PERSISTENCE * *club_form + carried * form_sd * standard_normal(rng);
        if rng.random_range(0.0..1.0) < SHOCK_CHANCE {
            *club_form += SHOCK_SD * standard_normal(rng);
        }
    }
}

/// The tables of one league over `seasons` seasons, oldest first. Its clubs' form carries from
/// one season to the next, and the draws come from a stream of the world's seed and the league
/// alone: the same world has the same past, another world another.
fn simulate_league_past(
    world_seed: u64,
    group: &LeagueGroup,
    reputations: &[f64],
    seasons: usize,
) -> Vec<Vec<StandingEntry>> {
    simulate_league_with_form(world_seed, group, reputations, seasons, FORM_SD)
}

/// [`simulate_league_past`] with the spread of the clubs' form given (a test reads the model
/// without its form to see what the form adds).
fn simulate_league_with_form(
    world_seed: u64,
    group: &LeagueGroup,
    reputations: &[f64],
    seasons: usize,
    form_sd: f64,
) -> Vec<Vec<StandingEntry>> {
    let mut rng = rng_for_seed(world_seed, &format!("history/{}", group.key), "-");
    let mut form: Vec<f64> = reputations
        .iter()
        .map(|_| form_sd * standard_normal(&mut rng))
        .collect();
    (0..seasons)
        .map(|_| {
            advance_form(&mut form, form_sd, &mut rng);
            let strengths = strengths_of(reputations, &form);
            play_season(&group.team_ids, &strengths, &mut rng)
        })
        .collect()
}

/// Every league's table for each of the `seasons` seasons from `first_season`: one entry per
/// season, oldest first, holding one table per league, best club first.
fn simulate_past_tables(game: &Game, seasons: usize) -> Vec<Vec<Vec<StandingEntry>>> {
    let reputation_of: HashMap<&str, f64> = game
        .teams
        .iter()
        .map(|team| (team.id.as_str(), f64::from(team.reputation)))
        .collect();
    let per_league: Vec<Vec<Vec<StandingEntry>>> = historical_league_groups(game)
        .iter()
        .map(|group| {
            let reputations: Vec<f64> = group
                .team_ids
                .iter()
                .map(|id| reputation_of.get(id.as_str()).copied().unwrap_or(0.0))
                .collect();
            simulate_league_past(game.seed, group, &reputations, seasons)
        })
        .collect();
    (0..seasons)
        .map(|season| {
            per_league
                .iter()
                .map(|league| league[season].clone())
                .collect()
        })
        .collect()
}

/// Each club's place in its own league's table and its row, for the season whose tables these are.
fn positions_by_team(tables: &[Vec<StandingEntry>]) -> HashMap<&str, (u32, &StandingEntry)> {
    tables
        .iter()
        .flat_map(|table| {
            table.iter().enumerate().map(|(index, standing)| {
                (standing.team_id.as_str(), ((index + 1) as u32, standing))
            })
        })
        .collect()
}

fn upsert_team_history(game: &mut Game, season: u32, tables: &[Vec<StandingEntry>]) {
    let standings_by_team = positions_by_team(tables);

    for team in game.teams.iter_mut() {
        let Some((league_position, standing)) = standings_by_team.get(team.id.as_str()) else {
            continue;
        };

        team.history.retain(|record| record.season != season);
        let record = TeamSeasonRecord {
            season,
            league_position: *league_position,
            played: standing.played,
            won: standing.won,
            drawn: standing.drawn,
            lost: standing.lost,
            goals_for: standing.goals_for,
            goals_against: standing.goals_against,
        };

        if team
            .history
            .last()
            .is_none_or(|existing| existing.season < season)
        {
            team.history.push(record);
        } else {
            team.history.push(record);
            team.history.sort_by_key(|left| left.season);
        }
    }
}

fn prepare_seeded_managers(game: &mut Game, first_season: u32) {
    crate::ai_hiring::seed_ai_managers(game);
    let start_date = season_start_string(first_season);

    for manager in game.managers.iter_mut() {
        let Some(team_id) = manager.team_id.clone() else {
            continue;
        };

        if let Some(entry) = manager
            .career_history
            .iter_mut()
            .find(|entry| entry.team_id == team_id && entry.end_date.is_none())
        {
            if entry.matches == 0 && entry.wins == 0 && entry.draws == 0 && entry.losses == 0 {
                entry.start_date = start_date.clone();
            }
            manager
                .career_history
                .sort_by(|left, right| left.start_date.cmp(&right.start_date));
            continue;
        }

        let team_name = game
            .teams
            .iter()
            .find(|team| team.id == team_id)
            .map(|team| team.name.clone())
            .unwrap_or_default();
        manager.career_history.push(ManagerCareerEntry {
            team_id,
            team_name,
            start_date: start_date.clone(),
            end_date: None,
            matches: 0,
            wins: 0,
            draws: 0,
            losses: 0,
            best_league_position: None,
        });
        manager
            .career_history
            .sort_by(|left, right| left.start_date.cmp(&right.start_date));
    }
}

fn upsert_manager_history(game: &mut Game, tables: &[Vec<StandingEntry>]) {
    let standings_by_team = positions_by_team(tables);

    for manager in game.managers.iter_mut() {
        let Some(team_id) = manager.team_id.clone() else {
            continue;
        };
        let Some((position, standing)) = standings_by_team.get(team_id.as_str()) else {
            continue;
        };
        let total_matches = standing.won + standing.drawn + standing.lost;

        manager.career_stats.matches_managed += total_matches;
        manager.career_stats.wins += standing.won;
        manager.career_stats.draws += standing.drawn;
        manager.career_stats.losses += standing.lost;
        if *position == 1 {
            manager.career_stats.trophies += 1;
        }
        if manager
            .career_stats
            .best_finish
            .is_none_or(|best_finish| *position < best_finish)
        {
            manager.career_stats.best_finish = Some(*position);
        }

        if let Some(entry) = manager
            .career_history
            .iter_mut()
            .find(|entry| entry.team_id == team_id && entry.end_date.is_none())
        {
            entry.matches += total_matches;
            entry.wins += standing.won;
            entry.draws += standing.drawn;
            entry.losses += standing.lost;
            if entry
                .best_league_position
                .is_none_or(|best_finish| *position < best_finish)
            {
                entry.best_league_position = Some(*position);
            }
        }
    }
}

fn synthesize_player_season(
    player: &Player,
    matches_played: u32,
    season: u32,
) -> PlayerSeasonStats {
    let missed_matches = deterministic_u32((&player.id, season, "missed"), 8);
    let appearances = matches_played.saturating_sub(missed_matches).max(8);
    let minutes_played = appearances.saturating_mul(90);
    let group_position = player.position.to_group_position();

    let (goals, assists, clean_sheets, shots, passes_completed, tackles_won, interceptions) =
        match group_position {
            Position::Goalkeeper => {
                let clean_sheets =
                    appearances / 3 + deterministic_u32((&player.id, season, "clean_sheets"), 4);
                (
                    0,
                    0,
                    clean_sheets,
                    0,
                    appearances.saturating_mul(18),
                    appearances / 2,
                    appearances,
                )
            }
            Position::Defender => {
                let clean_sheets =
                    appearances / 4 + deterministic_u32((&player.id, season, "clean_sheets"), 3);
                (
                    appearances / 12 + deterministic_u32((&player.id, season, "goals"), 3),
                    appearances / 10 + deterministic_u32((&player.id, season, "assists"), 3),
                    clean_sheets,
                    appearances.saturating_mul(1),
                    appearances.saturating_mul(26),
                    appearances.saturating_mul(3),
                    appearances.saturating_mul(2),
                )
            }
            Position::Midfielder => (
                appearances / 6 + deterministic_u32((&player.id, season, "goals"), 5),
                appearances / 5 + deterministic_u32((&player.id, season, "assists"), 4),
                0,
                appearances.saturating_mul(2),
                appearances.saturating_mul(32),
                appearances.saturating_mul(2),
                appearances.saturating_mul(2),
            ),
            Position::Forward => (
                appearances / 3 + deterministic_u32((&player.id, season, "goals"), 8),
                appearances / 7 + deterministic_u32((&player.id, season, "assists"), 3),
                0,
                appearances.saturating_mul(3),
                appearances.saturating_mul(18),
                appearances,
                appearances / 2,
            ),
            _ => unreachable!(),
        };

    let shots_on_target = shots.saturating_sub(shots / 3);
    let passes_attempted = passes_completed.saturating_add(appearances.saturating_mul(4));
    let fouls_committed = deterministic_u32((&player.id, season, "fouls"), appearances.max(1));

    PlayerSeasonStats {
        appearances,
        goals,
        assists,
        clean_sheets,
        yellow_cards: deterministic_u32((&player.id, season, "yellow"), 5),
        red_cards: deterministic_u32((&player.id, season, "red"), 2),
        // Historical counts are synthesized, but no match ratings were recorded.
        avg_rating: 0.0,
        rated_appearances: 0,
        minutes_played,
        shots,
        shots_on_target,
        passes_completed,
        passes_attempted,
        tackles_won,
        interceptions,
        fouls_committed,
    }
}

/// Youngest age at which a generated career season is recorded.
const MIN_SENIOR_CAREER_AGE: i32 = 18;

fn upsert_player_career(game: &mut Game, season: u32, standings: &[StandingEntry]) {
    let played_by_team: HashMap<&str, u32> = standings
        .iter()
        .map(|standing| (standing.team_id.as_str(), standing.played))
        .collect();
    let team_names: HashMap<&str, &str> = game
        .teams
        .iter()
        .map(|team| (team.id.as_str(), team.name.as_str()))
        .collect();

    for player in game.players.iter_mut() {
        let Some(team_id) = player.team_id.as_deref() else {
            continue;
        };
        let Some(matches_played) = played_by_team.get(team_id).copied() else {
            continue;
        };
        // No youth teams yet, so a career starts with the season a player is a senior at
        // kick-off. `opening_player_age` measures on 1 July, the date seasons start here.
        if crate::generator::opening_player_age(&player.date_of_birth, season as i32)
            .is_some_and(|age| age < MIN_SENIOR_CAREER_AGE)
        {
            player.stats = PlayerSeasonStats::default();
            continue;
        }

        player.stats = synthesize_player_season(player, matches_played, season);
        player.career.retain(|entry| entry.season != season);
        let entry = CareerEntry {
            season,
            team_id: team_id.to_string(),
            team_name: team_names
                .get(team_id)
                .copied()
                .unwrap_or_default()
                .to_string(),
            appearances: player.stats.appearances,
            goals: player.stats.goals,
            assists: player.stats.assists,
        };

        if player
            .career
            .last()
            .is_none_or(|existing| existing.season < season)
        {
            player.career.push(entry);
        } else {
            player.career.push(entry);
            player.career.sort_by_key(|left| left.season);
        }
    }
}

fn map_player_award_entry(
    entry: Option<&crate::season_awards::AwardEntry>,
) -> Option<HistoricalPlayerAwardWinner> {
    entry.map(|entry| HistoricalPlayerAwardWinner {
        player_id: entry.player_id.clone(),
        player_name: entry.player_name.clone(),
        team_id: entry.team_id.clone(),
        team_name: entry.team_name.clone(),
        value: entry.value,
    })
}

fn map_manager_award_entry(
    entry: Option<&crate::season_awards::ManagerAwardEntry>,
) -> Option<HistoricalManagerAwardWinner> {
    entry.map(|entry| HistoricalManagerAwardWinner {
        manager_id: entry.manager_id.clone(),
        manager_name: entry.manager_name.clone(),
        team_id: entry.team_id.clone(),
        team_name: entry.team_name.clone(),
        value: entry.value,
        win_rate: entry.win_rate,
    })
}

fn record_historical_awards(game: &mut Game, season: u32, awards: &SeasonAwards) {
    game.world_history
        .record_season_awards(HistoricalSeasonAwardsRecord {
            season,
            golden_boot: map_player_award_entry(awards.golden_boot.first()),
            assist_king: map_player_award_entry(awards.assist_king.first()),
            player_of_year: map_player_award_entry(awards.player_of_year.first()),
            clean_sheet_king: map_player_award_entry(awards.clean_sheet_king.first()),
            most_appearances: map_player_award_entry(awards.most_appearances.first()),
            young_player: map_player_award_entry(awards.young_player.first()),
            manager_of_season: map_manager_award_entry(awards.manager_of_season.first()),
        });
}

fn update_historical_rivalries(game: &mut Game, season: u32, standings: &[StandingEntry]) {
    let top_two = standings.iter().take(2).collect::<Vec<_>>();
    if top_two.len() == 2 {
        let team_a = &top_two[0].team_id;
        let team_b = &top_two[1].team_id;
        let existing = game.world_history.rivalries.iter().find(|rivalry| {
            (rivalry.team_a_id == *team_a && rivalry.team_b_id == *team_b)
                || (rivalry.team_a_id == *team_b && rivalry.team_b_id == *team_a)
        });
        let started_season = existing
            .and_then(|rivalry| rivalry.started_season)
            .map(|existing_start| existing_start.min(season))
            .or(Some(season));
        let intensity = existing
            .map(|rivalry| rivalry.intensity)
            .unwrap_or(55)
            .max(55 + deterministic_u32((team_a, team_b, season, "rivalry"), 36) as u8);
        game.world_history.upsert_rivalry(
            team_a.clone(),
            team_b.clone(),
            intensity,
            started_season,
        );
    }
}

fn reset_player_stats(game: &mut Game) {
    for player in &mut game.players {
        player.stats = PlayerSeasonStats::default();
    }
}

pub fn generate_past_world_history(game: &mut Game, start_year: i32, history_depth_years: u32) {
    if history_depth_years == 0 || game.teams.len() < 2 {
        return;
    }

    let first_season = start_year.saturating_sub(history_depth_years as i32);
    if first_season >= start_year {
        return;
    }

    prepare_seeded_managers(game, first_season as u32);

    let original_date = game.clock.current_date;
    let original_league = game.league.clone();

    let past = simulate_past_tables(game, (start_year - first_season) as usize);
    for (season_year, tables) in (first_season..start_year).zip(&past) {
        let season = season_year as u32;
        let standings: Vec<StandingEntry> = tables.iter().flatten().cloned().collect();
        upsert_team_history(game, season, tables);
        upsert_manager_history(game, tables);
        upsert_player_career(game, season, &standings);

        game.clock.current_date = Utc
            .with_ymd_and_hms(season_year + 1, 5, 31, 0, 0, 0)
            .unwrap();
        game.league = Some(League {
            id: DEFAULT_HISTORY_LEAGUE_ID.to_string(),
            name: DEFAULT_HISTORY_LEAGUE_NAME.to_string(),
            season,
            fixtures: Vec::new(),
            standings: standings.clone(),
            transfer_log: Vec::new(),
            transfer_rumours: Vec::new(),
            ..League::default()
        });

        let awards = crate::season_awards::compute_season_awards(game);
        record_historical_awards(game, season, &awards);
        update_historical_rivalries(game, season, &standings);
        reset_player_stats(game);
    }

    game.clock.current_date = original_date;
    game.league = original_league;
}

#[cfg(test)]
mod tests {
    use super::generate_past_world_history;
    use crate::{clock::GameClock, game::Game};
    use chrono::{TimeZone, Utc};
    use domain::{
        manager::{Manager, ManagerCareerEntry},
        player::{CareerEntry, Player, PlayerAttributes, Position},
        staff::{Staff, StaffAttributes, StaffRole},
        team::{Team, TeamSeasonRecord},
    };

    fn sample_player_attributes(position: &Position) -> PlayerAttributes {
        let mut attributes = PlayerAttributes {
            pace: 68,
            stamina: 70,
            strength: 67,
            agility: 69,
            passing: 66,
            shooting: 64,
            tackling: 58,
            dribbling: 65,
            defending: 57,
            positioning: 64,
            vision: 65,
            decisions: 66,
            composure: 64,
            aggression: 55,
            teamwork: 68,
            leadership: 54,
            handling: 18,
            reflexes: 18,
            aerial: 55,
        };

        match position {
            Position::Goalkeeper => {
                attributes.handling = 72;
                attributes.reflexes = 74;
                attributes.aerial = 70;
                attributes.positioning = 69;
                attributes.defending = 32;
                attributes.tackling = 24;
                attributes.shooting = 16;
            }
            Position::Striker => {
                attributes.shooting = 78;
                attributes.positioning = 74;
                attributes.dribbling = 72;
            }
            Position::CentralMidfielder => {
                attributes.passing = 74;
                attributes.vision = 73;
                attributes.stamina = 73;
            }
            _ => {}
        }

        attributes
    }

    fn make_team(index: usize) -> Team {
        let city = format!("City {}", index + 1);
        let mut team = Team::new(
            format!("team-{}", index + 1),
            format!("Club {}", index + 1),
            format!("C{}", index + 1),
            if index.is_multiple_of(2) {
                "England".to_string()
            } else {
                "Spain".to_string()
            },
            city.clone(),
            format!("{} Arena", city),
            25_000 + (index as u32 * 1_000),
        );
        team.reputation = 520 + (index as u32 * 55);
        team.finance = 1_500_000 + (index as i64 * 250_000);
        team
    }

    fn make_staff(team_id: &str, index: usize) -> Staff {
        let mut staff = Staff::new(
            format!("staff-{}", index + 1),
            format!("Coach{}", index + 1),
            "Seed".to_string(),
            format!("197{}-02-01", index),
            StaffRole::AssistantManager,
            StaffAttributes {
                coaching: 70,
                judging_ability: 68,
                judging_potential: 67,
                physiotherapy: 42,
            },
        );
        staff.nationality = "England".to_string();
        staff.team_id = Some(team_id.to_string());
        staff
    }

    fn make_player(team_id: &str, index: usize, position: Position, ovr: u8) -> Player {
        let year = 1990 + index as i32;
        let mut player = Player::new(
            format!("player-{}", index + 1),
            format!("P{}", index + 1),
            format!("Player {}", index + 1),
            format!("{}-03-15", year),
            "England".to_string(),
            position.clone(),
            sample_player_attributes(&position),
        );
        player.team_id = Some(team_id.to_string());
        player.ovr = ovr;
        player.potential = (ovr + 6).min(99);
        player
    }

    fn make_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2032, 7, 1, 0, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr-user".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = (0..4).map(make_team).collect::<Vec<_>>();
        let staff = teams
            .iter()
            .enumerate()
            .map(|(index, team)| make_staff(&team.id, index))
            .collect::<Vec<_>>();
        let mut players = Vec::new();
        for (index, team) in teams.iter().enumerate() {
            players.push(make_player(
                &team.id,
                index * 3,
                Position::Goalkeeper,
                64 + index as u8,
            ));
            players.push(make_player(
                &team.id,
                index * 3 + 1,
                Position::CentralMidfielder,
                68 + index as u8,
            ));
            players.push(make_player(
                &team.id,
                index * 3 + 2,
                Position::Striker,
                72 + index as u8,
            ));
        }

        Game::new(clock, manager, teams, players, staff, vec![])
    }

    fn seed_later_history_entries(game: &mut Game) {
        let team_id = game.teams[0].id.clone();
        let team_name = game.teams[0].name.clone();

        game.manager.team_id = Some(team_id.clone());
        game.teams[0].manager_id = Some(game.manager.id.clone());
        game.teams[0].history.push(TeamSeasonRecord {
            season: 2031,
            league_position: 2,
            played: 38,
            won: 22,
            drawn: 8,
            lost: 8,
            goals_for: 64,
            goals_against: 36,
        });

        let player = game
            .players
            .iter_mut()
            .find(|player| player.team_id.as_deref() == Some(team_id.as_str()))
            .expect("seed player for first team");
        player.career.push(CareerEntry {
            season: 2031,
            team_id: team_id.clone(),
            team_name: team_name.clone(),
            appearances: 33,
            goals: 7,
            assists: 4,
        });

        game.manager.career_history.push(ManagerCareerEntry {
            team_id,
            team_name,
            start_date: "2031-07-01".to_string(),
            end_date: Some("2032-06-30".to_string()),
            matches: 38,
            wins: 21,
            draws: 9,
            losses: 8,
            best_league_position: Some(2),
        });
        game.sync_user_manager_record();
    }

    fn serialized_history_snapshot(game: &Game) -> serde_json::Value {
        serde_json::json!({
            "teamHistory": game
                .teams
                .iter()
                .map(|team| serde_json::to_value(&team.history).unwrap())
                .collect::<Vec<_>>(),
            "playerCareerCounts": game
                .players
                .iter()
                .map(|player| player.career.len())
                .collect::<Vec<_>>(),
            "managerHistory": game
                .managers
                .iter()
                .map(|manager| serde_json::to_value(&manager.career_history).unwrap())
                .collect::<Vec<_>>(),
            "worldHistory": serde_json::to_value(&game.world_history).unwrap(),
        })
    }

    #[test]
    fn historical_standings_use_per_league_match_counts() {
        use domain::league::{League, StandingEntry};

        let clock = GameClock::new(Utc.with_ymd_and_hms(2032, 7, 1, 0, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = (0..12).map(make_team).collect::<Vec<_>>();
        let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);

        let division = |id: &str, range: std::ops::Range<usize>| League {
            id: id.to_string(),
            name: id.to_string(),
            season: 2031,
            standings: range
                .map(|index| StandingEntry::new(format!("team-{}", index + 1)))
                .collect(),
            ..League::default()
        };
        // Two 6-team domestic leagues, not one 12-team mega-league.
        game.competitions = vec![division("league-a", 0..6), division("league-b", 6..12)];

        let tables = super::simulate_past_tables(&game, 1);
        let standings: Vec<_> = tables[0].iter().flatten().collect();

        assert_eq!(standings.len(), 12);
        // A 6-team double round robin is (6-1)*2 = 10 matches — not the
        // (12-1)*2 = 22 a single combined table would produce.
        assert!(
            standings.iter().all(|entry| entry.played == 10),
            "expected per-league 10-match seasons, got {:?}",
            standings
                .iter()
                .map(|entry| entry.played)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn generate_past_world_history_populates_prior_records() {
        let mut game = make_game();

        generate_past_world_history(&mut game, 2032, 3);

        assert!(game.teams.iter().all(|team| team.history.len() == 3));
        assert!(game.players.iter().any(|player| !player.career.is_empty()));
        assert!(
            game.managers
                .iter()
                .any(|manager| !manager.career_history.is_empty())
        );
        assert_eq!(game.world_history.season_awards.len(), 3);
        assert!(!game.world_history.rivalries.is_empty());
    }

    fn career_seasons(game: &Game, player_id: &str) -> Vec<u32> {
        game.players
            .iter()
            .find(|player| player.id == player_id)
            .expect("player")
            .career
            .iter()
            .map(|entry| entry.season)
            .collect()
    }

    #[test]
    fn generated_career_starts_in_the_season_a_player_turns_eighteen() {
        let mut game = make_game();
        // 21 at the 2032 start: senior from 2029, so 2029-2031 of a 12-season backfill.
        game.players[0].date_of_birth = "2011-03-15".to_string();

        generate_past_world_history(&mut game, 2032, 12);

        assert_eq!(career_seasons(&game, "player-1"), vec![2029, 2030, 2031]);
    }

    #[test]
    fn underage_imported_stats_cannot_win_first_backfilled_season_awards() {
        let mut game = make_game();
        let player = &mut game.players[0];
        player.date_of_birth = "2015-03-15".to_string(); // 16 at the 2031 season start.
        player.stats.appearances = 38;
        player.stats.goals = 999;
        player.career.push(CareerEntry {
            season: 2031,
            team_id: player.team_id.clone().expect("club"),
            team_name: "Club 1".to_string(),
            appearances: 38,
            goals: 999,
            assists: 0,
        });

        generate_past_world_history(&mut game, 2032, 1);

        let first_season_awards = game
            .world_history
            .season_awards
            .iter()
            .find(|awards| awards.season == 2031)
            .expect("first backfilled season awards");
        assert_ne!(
            first_season_awards
                .golden_boot
                .as_ref()
                .expect("winner")
                .player_id,
            "player-1"
        );
        assert_eq!(career_seasons(&game, "player-1"), vec![2031]);
        assert_eq!(game.players[0].career[0].goals, 999);
    }

    #[test]
    fn a_birthday_on_the_season_start_counts_across_a_leap_year() {
        let mut game = make_game();
        // Born 1 July of a leap year: 18 on 1 July 2022 exactly. Day-of-year arithmetic
        // compares ordinal 183 (2004) with 182 (2022) and wrongly calls him 17.
        game.players[0].date_of_birth = "2004-07-01".to_string();
        // One day younger: still 17 when the 2022 season starts.
        game.players[1].date_of_birth = "2004-07-02".to_string();

        generate_past_world_history(&mut game, 2024, 3);

        assert_eq!(career_seasons(&game, "player-1"), vec![2022, 2023]);
        assert_eq!(career_seasons(&game, "player-2"), vec![2023]);
    }

    #[test]
    fn generate_past_world_history_is_deterministic() {
        let mut left = make_game();
        let mut right = make_game();

        generate_past_world_history(&mut left, 2032, 4);
        generate_past_world_history(&mut right, 2032, 4);

        assert_eq!(
            serialized_history_snapshot(&left),
            serialized_history_snapshot(&right)
        );
    }

    #[test]
    fn generate_past_world_history_backfills_seeded_entries_in_chronological_order() {
        let mut left = make_game();
        let mut right = make_game();

        seed_later_history_entries(&mut left);
        seed_later_history_entries(&mut right);

        generate_past_world_history(&mut left, 2032, 4);
        generate_past_world_history(&mut right, 2032, 4);

        let team = &left.teams[0];
        assert!(
            team.history
                .windows(2)
                .all(|window| window[0].season <= window[1].season)
        );

        let player = left
            .players
            .iter()
            .find(|player| player.team_id.as_deref() == Some(team.id.as_str()))
            .expect("player history for seeded team");
        assert!(
            player
                .career
                .windows(2)
                .all(|window| window[0].season <= window[1].season)
        );

        let manager = left
            .managers
            .iter()
            .find(|manager| manager.id == left.manager.id)
            .expect("user manager in manager list");
        assert!(
            manager
                .career_history
                .windows(2)
                .all(|window| window[0].start_date <= window[1].start_date)
        );

        assert_eq!(
            serialized_history_snapshot(&left),
            serialized_history_snapshot(&right)
        );
    }
    /// Given generated history for every position group, when stats are synthesized,
    /// then no match rating is invented.
    #[test]
    fn synthesized_player_seasons_are_unrated_in_every_position_group() {
        for position in [
            Position::Goalkeeper,
            Position::Defender,
            Position::Midfielder,
            Position::Forward,
        ] {
            let player = make_player("team-1", 0, position, 80);
            let stats = super::synthesize_player_season(&player, 38, 2031);
            assert_eq!(stats.avg_rating, 0.0);
            assert!(stats.appearances > 0);
        }
    }

    /// Given backfilled adult and young careers, when history is generated,
    /// then rating winners are absent while count-based awards and career rows remain.
    #[test]
    fn generated_unrated_history_has_no_player_of_year_winners() {
        let mut game = make_game();
        game.players.last_mut().unwrap().date_of_birth = "2010-01-01".to_string();
        generate_past_world_history(&mut game, 2032, 3);
        assert_eq!(game.world_history.season_awards.len(), 3);
        for awards in &game.world_history.season_awards {
            assert!(awards.player_of_year.is_none());
            assert!(awards.young_player.is_none());
            assert!(awards.golden_boot.is_some());
            assert!(awards.assist_king.is_some());
            assert!(awards.clean_sheet_king.is_some());
            assert!(awards.most_appearances.is_some());
        }
        assert!(game.players.iter().any(|player| !player.career.is_empty()));
    }

    fn league_of(count: usize) -> (super::LeagueGroup, Vec<f64>) {
        let group = super::LeagueGroup {
            key: "league".to_string(),
            team_ids: (0..count).map(|index| format!("club-{index:02}")).collect(),
        };
        // What a generated division looks like: reputation falling in even steps down the table.
        let reputations = (0..count)
            .map(|index| 850.0 - 10.8 * index as f64)
            .collect();
        (group, reputations)
    }

    /// Given a 20-club league,
    /// When one past season is played,
    /// Then every club played 38, wins draws and losses add up to that, the goals scored equal the
    ///      goals conceded, points are three a win and one a draw, and the table is in points order.
    #[test]
    fn a_past_season_is_a_double_round_robin_whose_numbers_tally() {
        let (group, reputations) = league_of(20);

        let table = &super::simulate_league_past(9, &group, &reputations, 1)[0];

        assert_eq!(table.len(), 20);
        for entry in table {
            assert_eq!(entry.played, 38);
            assert_eq!(entry.won + entry.drawn + entry.lost, 38);
            assert_eq!(entry.points, entry.won * 3 + entry.drawn);
        }
        let scored: u32 = table.iter().map(|entry| entry.goals_for).sum();
        let conceded: u32 = table.iter().map(|entry| entry.goals_against).sum();
        assert_eq!(scored, conceded);
        assert_eq!(
            table.iter().map(|entry| entry.won).sum::<u32>(),
            table.iter().map(|entry| entry.lost).sum::<u32>()
        );
        assert!(
            table
                .windows(2)
                .all(|pair| pair[0].points >= pair[1].points)
        );
    }

    /// Given the same league in the same world twice, and in other worlds,
    /// When the past is played,
    /// Then the same world has the same past, and the other worlds do not all crown the same
    ///      champions.
    #[test]
    fn a_pasts_champions_follow_the_worlds_seed() {
        let (group, reputations) = league_of(20);
        let champions = |seed: u64| -> Vec<String> {
            super::simulate_league_past(seed, &group, &reputations, 12)
                .iter()
                .map(|table| table[0].team_id.clone())
                .collect()
        };

        assert_eq!(champions(3), champions(3));
        let others: std::collections::HashSet<Vec<String>> = (10..20).map(champions).collect();
        assert!(
            others.len() > 5,
            "{} different pasts in ten worlds",
            others.len()
        );
    }

    /// Given many 20-club leagues of generated reputations over twelve seasons,
    /// When the champions are counted,
    /// Then they are those of a typical top league: 3 to 7 different champions, the
    ///      best-reputation club winning 30 to 50% of the titles, a champion on 75 to 100 points
    ///      with a real spread from year to year.
    #[test]
    fn a_past_has_the_variety_of_a_typical_top_league() {
        let (group, reputations) = league_of(20);
        let (mut distinct, mut favourite, mut points) = (Vec::new(), Vec::new(), Vec::new());
        for seed in 0..60 {
            let past = super::simulate_league_past(seed, &group, &reputations, 12);
            let champions: Vec<&str> = past.iter().map(|table| table[0].team_id.as_str()).collect();
            let unique: std::collections::HashSet<&str> = champions.iter().copied().collect();
            distinct.push(unique.len() as f64);
            favourite.push(champions.iter().filter(|id| **id == "club-00").count() as f64 / 12.0);
            points.extend(past.iter().map(|table| f64::from(table[0].points)));
        }
        let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len() as f64;
        let points_mean = mean(&points);
        let points_spread = (points
            .iter()
            .map(|p| (p - points_mean).powi(2))
            .sum::<f64>()
            / points.len() as f64)
            .sqrt();
        assert!(
            (3.5..=6.5).contains(&mean(&distinct)),
            "distinct {}",
            mean(&distinct)
        );
        assert!(
            (0.30..=0.50).contains(&mean(&favourite)),
            "favourite {}",
            mean(&favourite)
        );
        assert!((78.0..=95.0).contains(&points_mean), "points {points_mean}");
        assert!(points_spread > 3.0, "points spread {points_spread}");
    }

    /// Given a world of two 6-club leagues,
    /// When its past is generated,
    /// Then every club's recorded place is within its own league (1 to 6) and each league has one
    ///      club in each place every season.
    #[test]
    fn a_clubs_place_in_the_past_is_its_place_in_its_own_league() {
        use domain::league::League;
        let clock = GameClock::new(Utc.with_ymd_and_hms(2032, 7, 1, 0, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let teams = (0..12).map(make_team).collect::<Vec<_>>();
        let mut game = Game::new(clock, manager, teams, vec![], vec![], vec![]);
        let division = |id: &str, range: std::ops::Range<usize>| League {
            id: id.to_string(),
            name: id.to_string(),
            season: 2031,
            participant_ids: range.map(|index| format!("team-{}", index + 1)).collect(),
            ..League::default()
        };
        game.competitions = vec![division("league-a", 0..6), division("league-b", 6..12)];

        generate_past_world_history(&mut game, 2032, 3);

        for (members, name) in [(0..6, "league-a"), (6..12, "league-b")] {
            for season in 2029..2032 {
                let mut places: Vec<u32> = game.teams[members.clone()]
                    .iter()
                    .map(|team| {
                        team.history
                            .iter()
                            .find(|record| record.season == season)
                            .unwrap_or_else(|| panic!("{name} {season}"))
                            .league_position
                    })
                    .collect();
                places.sort_unstable();
                assert_eq!(places, vec![1, 2, 3, 4, 5, 6], "{name} {season}");
            }
        }
    }

    fn entry(team: &str, points: u32, scored: u32, conceded: u32) -> domain::league::StandingEntry {
        let mut entry = domain::league::StandingEntry::new(team.to_string());
        entry.points = points;
        entry.goals_for = scored;
        entry.goals_against = conceded;
        entry
    }

    /// Given clubs level on points, then on goal difference, then on goals scored,
    /// When the table is ordered,
    /// Then points come first, goal difference breaks a tie, goals scored breaks the next, and the
    ///      club id the last.
    #[test]
    fn a_table_is_ordered_by_points_goal_difference_goals_scored_then_id() {
        let mut table = vec![
            entry("e", 60, 50, 40),
            entry("d", 60, 55, 45),
            entry("c", 60, 60, 45),
            entry("b", 70, 20, 60),
            entry("a", 60, 55, 45),
        ];

        super::order_table(&mut table);

        let order: Vec<&str> = table.iter().map(|entry| entry.team_id.as_str()).collect();
        // b: most points. c: +15. then a, d (+10, 55 scored, by id), then e (+10, 50 scored).
        assert_eq!(order, ["b", "c", "a", "d", "e"]);
    }

    /// Given clubs level on points whose goal difference and goals scored disagree
    ///       (the best difference scored the fewest, another conceded nothing's worth of ratio),
    /// When the table is ordered,
    /// Then goal difference decides, not goals scored and not the ratio of the two.
    #[test]
    fn goal_difference_decides_a_points_tie_before_goals_scored() {
        let mut table = vec![
            entry("r", 60, 55, 50),
            entry("q", 60, 20, 9),
            entry("p", 60, 50, 30),
        ];

        super::order_table(&mut table);

        let order: Vec<&str> = table.iter().map(|entry| entry.team_id.as_str()).collect();
        // p +20, q +11, r +5: by goals scored it would be r, p, q; by the ratio q, r, p.
        assert_eq!(order, ["p", "q", "r"]);

        let mut conceding_nothing = vec![entry("x", 60, 10, 0), entry("y", 60, 30, 12)];
        super::order_table(&mut conceding_nothing);
        assert_eq!(conceding_nothing[0].team_id, "y", "+18 beats +10");
    }

    /// Given two clubs whose strengths are 0.5 and -0.5 (an edge of one),
    /// When many seasons of the pair are played,
    /// Then the stronger club's goals a season are the home and away means scaled by half the
    ///      edge (about 4.4): the goal model is pinned, not just the shape of its output.
    #[test]
    fn the_edge_between_two_clubs_scales_their_goals_by_half_the_gap() {
        use rand::SeedableRng;
        let mut rng = rand_chacha::ChaCha12Rng::seed_from_u64(7);
        let ids = vec!["strong".to_string(), "weak".to_string()];
        let seasons = 4000;
        let mut strong_goals = 0u32;
        let mut weak_goals = 0u32;
        for _ in 0..seasons {
            let table = super::play_season(&ids, &[0.5, -0.5], &mut rng);
            strong_goals += table
                .iter()
                .find(|row| row.team_id == "strong")
                .unwrap()
                .goals_for;
            weak_goals += table
                .iter()
                .find(|row| row.team_id == "weak")
                .unwrap()
                .goals_for;
        }
        let strong = f64::from(strong_goals) / f64::from(seasons);
        let weak = f64::from(weak_goals) / f64::from(seasons);

        let half_edge = 0.5_f64.exp();
        let expected_strong = super::HOME_GOALS * half_edge + super::AWAY_GOALS * half_edge;
        let expected_weak = super::HOME_GOALS / half_edge + super::AWAY_GOALS / half_edge;
        assert!(
            (strong - expected_strong).abs() < 0.15,
            "strong {strong}, expected {expected_strong}"
        );
        assert!(
            (weak - expected_weak).abs() < 0.15,
            "weak {weak}, expected {expected_weak}"
        );
    }

    /// Given three clubs of reputation 100, 200 and 300 and no form,
    /// When their strengths are read,
    /// Then they are the standardised reputation times the pull of reputation, level at the
    ///      middle, and form is added as it is.
    #[test]
    fn a_clubs_strength_is_its_standardised_reputation_plus_its_form() {
        let level = super::strengths_of(&[100.0, 200.0, 300.0], &[0.0, 0.0, 0.0]);
        // The population spread of 100, 200 and 300 is sqrt(20000 / 3).
        let expected = super::STRENGTH_PER_REPUTATION_SD * 100.0 / (20_000.0_f64 / 3.0).sqrt();
        assert!((level[0] + expected).abs() < 1e-9, "{level:?}");
        assert!(level[1].abs() < 1e-9);
        assert!((level[2] - expected).abs() < 1e-9);

        let formed = super::strengths_of(&[100.0, 200.0, 300.0], &[0.1, -0.2, 0.3]);
        assert!((formed[0] - (level[0] + 0.1)).abs() < 1e-9);
        assert!((formed[1] - (level[1] - 0.2)).abs() < 1e-9);
        assert!((formed[2] - (level[2] + 0.3)).abs() < 1e-9);
        // A league of equals has no pull to read.
        assert_eq!(
            super::strengths_of(&[500.0, 500.0], &[0.0, 0.0]),
            vec![0.0, 0.0]
        );
    }

    /// Given many clubs' form moved on for many seasons,
    /// When the spread of the form and its correlation with last season's are read,
    /// Then the spread stays what the model sets (with the shocks) and about the persistence of
    ///      it carries over.
    #[test]
    fn form_keeps_its_spread_and_carries_over_by_the_persistence() {
        use rand::SeedableRng;
        let mut rng = rand_chacha::ChaCha12Rng::seed_from_u64(5);
        let mut form = vec![0.0; 2000];
        for _ in 0..30 {
            super::advance_form(&mut form, super::FORM_SD, &mut rng);
        }
        let previous = form.clone();
        super::advance_form(&mut form, super::FORM_SD, &mut rng);

        let n = form.len() as f64;
        let spread = (form.iter().map(|f| f * f).sum::<f64>() / n).sqrt();
        let correlation = form.iter().zip(&previous).map(|(f, p)| f * p).sum::<f64>()
            / (n * spread * (previous.iter().map(|p| p * p).sum::<f64>() / n).sqrt());
        // 0.2 of ordinary form, plus a shock 3% of the time, 0.4 wide and fading by the persistence.
        assert!((0.205..0.245).contains(&spread), "spread {spread}");
        assert!(
            (0.62..0.78).contains(&correlation),
            "correlation {correlation}"
        );
    }

    /// Given a 20-club league over twelve seasons, with and without clubs' form,
    /// When a club's points are correlated with its points the season before (each against the
    ///      club's own average),
    /// Then with form they follow each other (good and bad spells), and without it they do not.
    #[test]
    fn a_clubs_good_and_bad_spells_come_from_its_form() {
        let (group, reputations) = league_of(20);
        let autocorrelation = |form_sd: f64| {
            let (mut numerator, mut left, mut right) = (0.0, 0.0, 0.0);
            for seed in 0..40 {
                let past =
                    super::simulate_league_with_form(seed, &group, &reputations, 12, form_sd);
                for id in &group.team_ids {
                    let points: Vec<f64> = past
                        .iter()
                        .map(|table| {
                            f64::from(table.iter().find(|row| &row.team_id == id).unwrap().points)
                        })
                        .collect();
                    let mean = points.iter().sum::<f64>() / points.len() as f64;
                    for pair in points.windows(2) {
                        numerator += (pair[0] - mean) * (pair[1] - mean);
                        left += (pair[0] - mean).powi(2);
                        right += (pair[1] - mean).powi(2);
                    }
                }
            }
            numerator / (left * right).sqrt()
        };

        let with_form = autocorrelation(super::FORM_SD);
        let without = autocorrelation(0.0);

        assert!(with_form > 0.07, "with form {with_form}");
        assert!(
            with_form > without + 0.08,
            "with {with_form}, without {without}"
        );
    }

    /// Given the managers of a small world's clubs,
    /// When four past seasons are generated,
    /// Then every club's manager carries that club's titles, matches and best finish.
    #[test]
    fn a_clubs_manager_carries_the_clubs_past() {
        let mut game = make_game();

        generate_past_world_history(&mut game, 2032, 4);

        let mut titles_seen = 0;
        for team in &game.teams {
            let manager = game
                .managers
                .iter()
                .find(|manager| manager.team_id.as_deref() == Some(team.id.as_str()))
                .unwrap_or_else(|| panic!("{} has a manager", team.id));
            let titles = team
                .history
                .iter()
                .filter(|record| record.league_position == 1)
                .count() as u32;
            titles_seen += titles;
            assert_eq!(manager.career_stats.trophies, titles, "{}", team.id);
            assert_eq!(manager.career_stats.matches_managed, 6 * 4);
            let best = team.history.iter().map(|r| r.league_position).min();
            assert_eq!(manager.career_stats.best_finish, best);
        }
        assert_eq!(
            titles_seen, 4,
            "one champion a season in a one-league world"
        );
    }
}
