//! National-team scheduling and simulation.
//!
//! National teams are non-playable in this milestone. They contest FIFA-style
//! friendlies during international windows; results are derived from squad
//! strength and the called-up players carry fatigue, fitness, morale, and
//! injury risk back to their clubs (full carry-back over the shared
//! [`Player`](domain::player::Player) records).

use chrono::{DateTime, Datelike, Utc};
use domain::league::{Fixture, FixtureCompetition, FixtureStatus, MatchResult};
use domain::national_team::NationalTeam;
use domain::player::Player;
use rand::{Rng, RngExt};

use crate::game::Game;

/// Synthetic competition id used to tag national-team friendly fixtures.
pub const INTERNATIONAL_FRIENDLY_COMPETITION_ID: &str = "international-friendlies";

/// Number of players that feature (and therefore wear) per national-team match.
const MATCH_SQUAD_SIZE: usize = 11;

/// International break dates for a season. FIFA-style: autumn windows in
/// September, October and November, then spring windows in March and June of
/// the following calendar year. `season_start` is the club season's start
/// (around August), so all five windows fall within the season.
pub fn international_window_dates(season_start: DateTime<Utc>) -> Vec<String> {
    let start_year = season_start.year();
    let windows = [
        (start_year, 9, 9),
        (start_year, 10, 14),
        (start_year, 11, 18),
        (start_year + 1, 3, 24),
        (start_year + 1, 6, 9),
    ];
    windows
        .iter()
        .filter_map(|(year, month, day)| {
            chrono::NaiveDate::from_ymd_opt(*year, *month, *day)
                .map(|date| date.format("%Y-%m-%d").to_string())
        })
        .collect()
}

/// Schedule one friendly per international window for every national team that
/// can field a squad. Fixtures are stored on the **home** team only so each
/// match is simulated exactly once during day processing.
pub fn schedule_national_team_friendlies(
    national_teams: &mut [NationalTeam],
    window_dates: &[String],
    rng: &mut impl Rng,
) {
    let mut eligible: Vec<usize> = national_teams
        .iter()
        .enumerate()
        .filter(|(_, team)| !team.squad_player_ids.is_empty())
        .map(|(index, _)| index)
        .collect();
    if eligible.len() < 2 {
        return;
    }

    for (window_index, date) in window_dates.iter().enumerate() {
        shuffle(&mut eligible, rng);
        for pair in eligible.chunks(2) {
            let [home_idx, away_idx] = pair else {
                continue; // Odd team out sits this window.
            };
            let home_id = national_teams[*home_idx].id.clone();
            let away_id = national_teams[*away_idx].id.clone();
            let fixture = Fixture {
                id: format!("ntf-{window_index}-{home_id}-{away_id}"),
                competition_id: INTERNATIONAL_FRIENDLY_COMPETITION_ID.to_string(),
                matchday: window_index as u32 + 1,
                date: date.clone(),
                home_team_id: home_id,
                away_team_id: away_id,
                competition: FixtureCompetition::InternationalNation,
                status: FixtureStatus::Scheduled,
                result: None,
            };
            national_teams[*home_idx].fixtures.push(fixture);
        }
    }
}

/// Simulate every national-team fixture due on `today`, applying full carry-back
/// to the called-up club players. Returns the number of fixtures simulated.
pub fn process_national_team_fixtures_due(
    game: &mut Game,
    today: &str,
    rng: &mut impl Rng,
) -> usize {
    let due: Vec<(usize, usize)> = game
        .national_teams
        .iter()
        .enumerate()
        .flat_map(|(team_index, team)| {
            team.fixtures
                .iter()
                .enumerate()
                .filter(|(_, fixture)| {
                    fixture.date == today && fixture.status == FixtureStatus::Scheduled
                })
                .map(move |(fixture_index, _)| (team_index, fixture_index))
        })
        .collect();

    let mut simulated = 0;
    for (team_index, fixture_index) in due {
        let (home_id, away_id) = {
            let fixture = &game.national_teams[team_index].fixtures[fixture_index];
            (fixture.home_team_id.clone(), fixture.away_team_id.clone())
        };

        let (home_goals, away_goals) = play_national_match(game, &home_id, &away_id, rng);

        let fixture = &mut game.national_teams[team_index].fixtures[fixture_index];
        fixture.status = FixtureStatus::Completed;
        fixture.result = Some(MatchResult {
            home_goals,
            away_goals,
            home_scorers: Vec::new(),
            away_scorers: Vec::new(),
            report: None,
        });
        simulated += 1;
    }
    simulated
}

/// Play one national-team match between two squads: derive the scoreline from
/// squad strength and apply full carry-back (fatigue, fitness, injuries,
/// morale) to the involved club players.
pub fn play_national_match(
    game: &mut Game,
    home_national_team_id: &str,
    away_national_team_id: &str,
    rng: &mut impl Rng,
) -> (u8, u8) {
    let home_squad = squad_ids_for(game, home_national_team_id);
    let away_squad = squad_ids_for(game, away_national_team_id);
    let home_strength = squad_strength(&home_squad, &game.players);
    let away_strength = squad_strength(&away_squad, &game.players);
    let (home_goals, away_goals) = simulate_scoreline(home_strength, away_strength, rng);

    apply_carry_back(game, &home_squad, home_goals, away_goals, rng);
    apply_carry_back(game, &away_squad, away_goals, home_goals, rng);
    (home_goals, away_goals)
}

fn squad_ids_for(game: &Game, national_team_id: &str) -> Vec<String> {
    game.national_teams
        .iter()
        .find(|team| team.id == national_team_id)
        .map(|team| team.squad_player_ids.clone())
        .unwrap_or_default()
}

/// The best XI ids from a squad, strongest first.
fn match_day_xi(squad_player_ids: &[String], players: &[Player]) -> Vec<String> {
    let mut rated: Vec<(String, u8)> = squad_player_ids
        .iter()
        .filter_map(|pid| {
            players
                .iter()
                .find(|p| &p.id == pid)
                .map(|p| (p.id.clone(), p.ovr))
        })
        .collect();
    rated.sort_by(|left, right| right.1.cmp(&left.1));
    rated
        .into_iter()
        .take(MATCH_SQUAD_SIZE)
        .map(|(id, _)| id)
        .collect()
}

/// Average OVR of a squad's best XI; used as match strength.
fn squad_strength(squad_player_ids: &[String], players: &[Player]) -> f64 {
    let xi = match_day_xi(squad_player_ids, players);
    if xi.is_empty() {
        return 50.0;
    }
    let total: u32 = xi
        .iter()
        .filter_map(|pid| players.iter().find(|p| &p.id == pid).map(|p| p.ovr as u32))
        .sum();
    total as f64 / xi.len() as f64
}

/// Derive a scoreline from the two sides' strengths. Shared with the tiered
/// (dormant-league) simulation, which resolves matches by scoreline only.
pub(crate) fn simulate_scoreline(
    home_strength: f64,
    away_strength: f64,
    rng: &mut impl Rng,
) -> (u8, u8) {
    let edge = (home_strength - away_strength) / 10.0;
    let home_xg = (1.3 + 0.25 * edge).clamp(0.2, 4.0);
    let away_xg = (1.1 - 0.25 * edge).clamp(0.2, 4.0);
    (sample_goals(home_xg, rng), sample_goals(away_xg, rng))
}

/// Sample a goal count from an expected-goals mean (Knuth's Poisson), capped.
fn sample_goals(mean: f64, rng: &mut impl Rng) -> u8 {
    let threshold = (-mean).exp();
    let mut goals = 0u8;
    let mut product = 1.0;
    loop {
        product *= rng.random_range(0.0..1.0);
        if product <= threshold || goals >= 9 {
            break;
        }
        goals += 1;
    }
    goals
}

/// Apply fatigue, fitness, injury risk, and a small morale nudge to a national
/// team's match-day side. `goals_for`/`goals_against` drive the morale change.
fn apply_carry_back(
    game: &mut Game,
    squad_player_ids: &[String],
    goals_for: u8,
    goals_against: u8,
    rng: &mut impl Rng,
) {
    let morale_delta: i16 = match goals_for.cmp(&goals_against) {
        std::cmp::Ordering::Greater => 2,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Less => -1,
    };

    let xi = match_day_xi(squad_player_ids, &game.players);
    for pid in xi {
        if let Some(player) = game.players.iter_mut().find(|p| p.id == pid) {
            crate::player_wear::apply_match_wear(player, 90, rng);
            crate::player_wear::roll_match_injury(player, rng);
            if morale_delta != 0 {
                player.morale = (player.morale as i16 + morale_delta).clamp(10, 100) as u8;
            }
        }
    }
}

fn shuffle(items: &mut [usize], rng: &mut impl Rng) {
    for i in (1..items.len()).rev() {
        let j = rng.random_range(0..=i);
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::TimeZone;
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes, Position};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 70,
            stamina: 70,
            strength: 70,
            agility: 70,
            passing: 70,
            shooting: 70,
            tackling: 70,
            dribbling: 70,
            defending: 70,
            positioning: 70,
            vision: 70,
            decisions: 70,
            composure: 70,
            aggression: 70,
            teamwork: 70,
            leadership: 70,
            handling: 50,
            reflexes: 50,
            aerial: 60,
        }
    }

    fn make_player(id: &str, ovr: u8) -> Player {
        let mut player = Player::new(
            id.to_string(),
            format!("{id} name"),
            format!("{id} fullname"),
            "2000-01-01".to_string(),
            "ENG".to_string(),
            Position::Midfielder,
            attrs(),
        );
        player.ovr = ovr;
        player.condition = 100;
        player.fitness = 80;
        player
    }

    fn make_national_team(id: &str, nation: &str, squad: &[&str]) -> NationalTeam {
        let mut team = NationalTeam::new(
            id.to_string(),
            format!("{nation} National Team"),
            nation.to_string(),
            None,
        );
        team.squad_player_ids = squad.iter().map(|s| s.to_string()).collect();
        team
    }

    fn empty_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 9, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "ENG".to_string(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    #[test]
    fn international_window_dates_are_five_in_season_order() {
        let season_start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let dates = international_window_dates(season_start);

        assert_eq!(dates.len(), 5);
        assert_eq!(
            dates,
            vec![
                "2026-09-09".to_string(),
                "2026-10-14".to_string(),
                "2026-11-18".to_string(),
                "2027-03-24".to_string(),
                "2027-06-09".to_string(),
            ]
        );
        let mut sorted = dates.clone();
        sorted.sort();
        assert_eq!(dates, sorted, "windows should already be chronological");
    }

    #[test]
    fn schedule_assigns_one_friendly_per_team_per_window() {
        let mut teams = vec![
            make_national_team("nt-eng", "ENG", &["p1"]),
            make_national_team("nt-bra", "BRA", &["p2"]),
            make_national_team("nt-fra", "FRA", &["p3"]),
            make_national_team("nt-ger", "GER", &["p4"]),
        ];
        let windows = vec!["2026-09-09".to_string(), "2026-10-14".to_string()];
        let mut rng = StdRng::seed_from_u64(7);

        schedule_national_team_friendlies(&mut teams, &windows, &mut rng);

        let all_fixtures: Vec<&Fixture> = teams.iter().flat_map(|t| t.fixtures.iter()).collect();
        // 4 teams -> 2 fixtures per window, 2 windows -> 4 fixtures total.
        assert_eq!(all_fixtures.len(), 4);
        for window in &windows {
            let in_window: Vec<&&Fixture> =
                all_fixtures.iter().filter(|f| &f.date == window).collect();
            assert_eq!(in_window.len(), 2);
            let mut participants: Vec<&str> = in_window
                .iter()
                .flat_map(|f| [f.home_team_id.as_str(), f.away_team_id.as_str()])
                .collect();
            participants.sort();
            participants.dedup();
            assert_eq!(participants.len(), 4, "every team plays once per window");
        }
        assert!(all_fixtures.iter().all(|f| {
            f.competition == FixtureCompetition::InternationalNation
                && f.status == FixtureStatus::Scheduled
        }));
    }

    #[test]
    fn schedule_skips_teams_without_a_squad() {
        let mut teams = vec![
            make_national_team("nt-eng", "ENG", &["p1"]),
            make_national_team("nt-bra", "BRA", &[]),
        ];
        let windows = vec!["2026-09-09".to_string()];
        let mut rng = StdRng::seed_from_u64(1);

        schedule_national_team_friendlies(&mut teams, &windows, &mut rng);

        assert!(teams.iter().all(|t| t.fixtures.is_empty()));
    }

    #[test]
    fn process_due_fixtures_completes_match_and_carries_fatigue_back() {
        let mut game = empty_game();
        game.players = vec![make_player("p1", 80), make_player("p2", 60)];
        let mut home = make_national_team("nt-eng", "ENG", &["p1"]);
        home.fixtures.push(Fixture {
            id: "ntf-0".to_string(),
            competition_id: INTERNATIONAL_FRIENDLY_COMPETITION_ID.to_string(),
            matchday: 1,
            date: "2026-09-09".to_string(),
            home_team_id: "nt-eng".to_string(),
            away_team_id: "nt-bra".to_string(),
            competition: FixtureCompetition::InternationalNation,
            status: FixtureStatus::Scheduled,
            result: None,
        });
        let away = make_national_team("nt-bra", "BRA", &["p2"]);
        game.national_teams = vec![home, away];

        let mut rng = StdRng::seed_from_u64(42);
        let simulated = process_national_team_fixtures_due(&mut game, "2026-09-09", &mut rng);

        assert_eq!(simulated, 1);
        let fixture = &game.national_teams[0].fixtures[0];
        assert_eq!(fixture.status, FixtureStatus::Completed);
        assert!(fixture.result.is_some());
        // Both called-up players carried fatigue back to their clubs.
        assert!(game.players.iter().all(|p| p.condition < 100));
    }

    #[test]
    fn process_ignores_fixtures_on_other_days() {
        let mut game = empty_game();
        game.players = vec![make_player("p1", 80), make_player("p2", 70)];
        let mut home = make_national_team("nt-eng", "ENG", &["p1"]);
        home.fixtures.push(Fixture {
            id: "ntf-0".to_string(),
            competition_id: INTERNATIONAL_FRIENDLY_COMPETITION_ID.to_string(),
            matchday: 1,
            date: "2026-10-14".to_string(),
            home_team_id: "nt-eng".to_string(),
            away_team_id: "nt-bra".to_string(),
            competition: FixtureCompetition::InternationalNation,
            status: FixtureStatus::Scheduled,
            result: None,
        });
        game.national_teams = vec![home, make_national_team("nt-bra", "BRA", &["p2"])];

        let mut rng = StdRng::seed_from_u64(3);
        let simulated = process_national_team_fixtures_due(&mut game, "2026-09-09", &mut rng);

        assert_eq!(simulated, 0);
        assert_eq!(
            game.national_teams[0].fixtures[0].status,
            FixtureStatus::Scheduled
        );
        assert!(game.players.iter().all(|p| p.condition == 100));
    }

    #[test]
    fn stronger_team_outscores_weaker_over_many_friendlies() {
        let mut rng = StdRng::seed_from_u64(99);
        let mut home_total = 0u32;
        let mut away_total = 0u32;
        for _ in 0..400 {
            let (home, away) = simulate_scoreline(85.0, 60.0, &mut rng);
            home_total += home as u32;
            away_total += away as u32;
        }
        assert!(
            home_total > away_total,
            "a much stronger home side should outscore the weaker one (home {home_total} vs away {away_total})"
        );
    }
}
