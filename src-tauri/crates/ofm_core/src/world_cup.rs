//! The World Cup: a quadrennial national-team tournament played in the summer
//! break (the real-world calendar: 2022, 2026, 2030, …). The field is filled
//! from the strongest national pools in the world; nations without enough
//! players get squads synthesised as free agents, so any world can stage it.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Utc};
use domain::league::{
    CompetitionScope, CompetitionType, FixtureStatus, League, MatchResult,
};
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::national_team::NationalTeam;
use rand::Rng;

use crate::game::Game;
use crate::group_stage::GroupStageConfig;
use crate::nations;

/// A World Cup format preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldCupFormat {
    pub field: usize,
    pub qualifiers_per_group: u32,
    pub best_third_qualifiers: u32,
}

/// The 2026 format: 48 teams in 12 groups; top two per group plus the eight
/// best third-placed teams reach a round of 32.
pub const FORMAT_48: WorldCupFormat = WorldCupFormat {
    field: 48,
    qualifiers_per_group: 2,
    best_third_qualifiers: 8,
};

/// The 1998–2022 format: 32 teams in 8 groups; top two reach a round of 16.
pub const FORMAT_32: WorldCupFormat = WorldCupFormat {
    field: 32,
    qualifiers_per_group: 2,
    best_third_qualifiers: 0,
};

/// A compact format for small worlds: 16 teams in 4 groups; top two reach the
/// quarterfinals.
pub const FORMAT_16: WorldCupFormat = WorldCupFormat {
    field: 16,
    qualifiers_per_group: 2,
    best_third_qualifiers: 0,
};

/// Nations with at least this many players qualify on their own strength.
const MIN_NATIONAL_POOL: usize = 15;
/// Squads synthesised or topped up reach this size.
const TOPPED_UP_POOL: usize = 18;
/// Days between group matchdays — the tournament must fit the summer break.
const GROUP_MATCHDAY_GAP_DAYS: i64 = 2;
/// Days between knockout rounds.
const KNOCKOUT_GAP_DAYS: u32 = 4;

/// World Cups take place in the summers of 2022, 2026, 2030, …
pub fn is_world_cup_summer(year: i32) -> bool {
    year.rem_euclid(4) == 2
}

/// Whether a competition is a World Cup (a national-team tournament).
pub fn is_world_cup_competition(competition: &League) -> bool {
    competition.kind == CompetitionType::InternationalNation
        && competition.scope == CompetitionScope::International
}

/// Player OVRs per nation, strongest first (non-retired players only).
fn national_pools(game: &Game) -> BTreeMap<String, Vec<u8>> {
    let mut pools: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for player in game.players.iter().filter(|player| !player.retired) {
        let nation = if player.football_nation.is_empty() {
            player.nationality.clone()
        } else {
            player.football_nation.clone()
        };
        pools.entry(nation).or_default().push(player.ovr);
    }
    for ovrs in pools.values_mut() {
        ovrs.sort_unstable_by(|a, b| b.cmp(a));
    }
    pools
}

/// Average OVR of a pool's best XI.
fn pool_strength(ovrs: &[u8]) -> f64 {
    let xi: Vec<u8> = ovrs.iter().copied().take(11).collect();
    if xi.is_empty() {
        return 0.0;
    }
    xi.iter().map(|&ovr| ovr as u32).sum::<u32>() as f64 / xi.len() as f64
}

/// Pick the World Cup field: the strongest viable nations in the world first,
/// then catalog nations (in catalog order) until the format's field is full.
fn select_field(game: &Game, format: &WorldCupFormat) -> Vec<String> {
    let pools = national_pools(game);

    let mut viable: Vec<(&String, f64)> = pools
        .iter()
        .filter(|(_, ovrs)| ovrs.len() >= MIN_NATIONAL_POOL)
        .map(|(nation, ovrs)| (nation, pool_strength(ovrs)))
        .collect();
    viable.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut field: Vec<String> = viable
        .into_iter()
        .take(format.field)
        .map(|(nation, _)| nation.clone())
        .collect();

    for nation in nations::NATION_CATALOG {
        if field.len() >= format.field {
            break;
        }
        if !field.iter().any(|code| code == nation.code) {
            field.push(nation.code.to_string());
        }
    }
    field.truncate(format.field);
    field
}

fn national_team_id(code: &str) -> String {
    format!("nt-{}", code.to_lowercase())
}

/// Top up thin national pools with generated free agents and (re)build the
/// national-team squads for every nation in the field.
fn prepare_national_squads(game: &mut Game, field: &[String]) {
    let pools = national_pools(game);
    for code in field {
        let have = pools.get(code).map(|ovrs| ovrs.len()).unwrap_or(0);
        for slot in have..TOPPED_UP_POOL {
            game.players
                .push(crate::generator::generate_national_team_player(code, slot));
        }
    }

    for code in field {
        let mut squad: Vec<(String, u8)> = game
            .players
            .iter()
            .filter(|player| !player.retired)
            .filter(|player| {
                let nation = if player.football_nation.is_empty() {
                    &player.nationality
                } else {
                    &player.football_nation
                };
                nation == code
            })
            .map(|player| (player.id.clone(), player.ovr))
            .collect();
        squad.sort_by(|a, b| b.1.cmp(&a.1));
        let squad_player_ids: Vec<String> =
            squad.into_iter().take(23).map(|(id, _)| id).collect();

        if let Some(team) = game
            .national_teams
            .iter_mut()
            .find(|team| &team.football_nation == code)
        {
            team.squad_player_ids = squad_player_ids;
        } else {
            let mut team = NationalTeam::new(
                national_team_id(code),
                nations::nation_display_name(code),
                code.clone(),
                nations::nation_by_code(code).map(|nation| nation.region_id.to_string()),
            );
            team.squad_player_ids = squad_player_ids;
            game.national_teams.push(team);
        }
    }
}

/// Schedule a World Cup for the summer beginning at `kickoff` when the year is
/// a World Cup year and none is already running. Returns whether one was
/// scheduled.
pub fn schedule_world_cup_if_due(game: &mut Game, kickoff: DateTime<Utc>) -> bool {
    if !is_world_cup_summer(kickoff.year()) {
        return false;
    }
    if game.competitions.iter().any(is_world_cup_competition) {
        return false;
    }
    schedule_world_cup(game, kickoff, &FORMAT_48);
    true
}

/// Schedule a World Cup with an explicit format (48 by default; 32 and 16
/// reproduce the older tournaments).
pub fn schedule_world_cup(game: &mut Game, kickoff: DateTime<Utc>, format: &WorldCupFormat) {
    let field = select_field(game, format);
    if field.len() < 4 {
        return;
    }
    prepare_national_squads(game, &field);

    let participant_ids: Vec<String> = field
        .iter()
        .map(|code| {
            game.national_teams
                .iter()
                .find(|team| &team.football_nation == code)
                .map(|team| team.id.clone())
                .unwrap_or_else(|| national_team_id(code))
        })
        .collect();

    let year = kickoff.year();
    let mut cup = crate::group_stage::generate_group_knockout_cup_with(
        &format!("World Cup {year}"),
        year as u32,
        &participant_ids,
        kickoff,
        CompetitionType::InternationalNation,
        CompetitionScope::International,
        &GroupStageConfig {
            legs: 1,
            matchday_gap_days: GROUP_MATCHDAY_GAP_DAYS,
            qualifiers_per_group: format.qualifiers_per_group,
            best_third_qualifiers: format.best_third_qualifiers,
            knockout_round_gap_days: KNOCKOUT_GAP_DAYS,
        },
    );
    // Sort after every club competition in browsing lists.
    cup.priority = 10_000;
    let cup_id = cup.id.clone();
    game.competitions.push(cup);
    if !game.active_competition_ids.is_empty() {
        game.active_competition_ids.push(cup_id);
    }
}

/// Simulate every World Cup fixture due on `today` with the national-team
/// engine (carry-back included), progressing groups and knockout rounds, and
/// announcing the champion when the final is decided. Returns the number of
/// fixtures simulated.
pub fn process_world_cup_fixtures_due(game: &mut Game, today: &str, rng: &mut impl Rng) -> usize {
    let competition_indices: Vec<usize> = game
        .competitions
        .iter()
        .enumerate()
        .filter(|(_, competition)| is_world_cup_competition(competition))
        .map(|(index, _)| index)
        .collect();

    let mut simulated = 0;
    for competition_index in competition_indices {
        let due: Vec<usize> = game.competitions[competition_index]
            .fixtures
            .iter()
            .enumerate()
            .filter(|(_, fixture)| {
                fixture.date == today && fixture.status == FixtureStatus::Scheduled
            })
            .map(|(fixture_index, _)| fixture_index)
            .collect();

        for fixture_index in due {
            let (home_id, away_id) = {
                let fixture = &game.competitions[competition_index].fixtures[fixture_index];
                (fixture.home_team_id.clone(), fixture.away_team_id.clone())
            };
            let (home_goals, away_goals) =
                crate::national_team::play_national_match(game, &home_id, &away_id, rng);

            let competition = &mut game.competitions[competition_index];
            let fixture = &mut competition.fixtures[fixture_index];
            fixture.status = FixtureStatus::Completed;
            fixture.result = Some(MatchResult {
                home_goals,
                away_goals,
                home_scorers: Vec::new(),
                away_scorers: Vec::new(),
                report: None,
            });
            crate::group_stage::process_completed_fixture(competition, fixture_index);
            crate::schedule::advance_knockout_competition_round(competition);
            simulated += 1;
        }

        announce_champion_if_decided(game, competition_index, today);
    }
    simulated
}

/// The tournament winner, once the final has been played.
pub fn world_cup_champion(competition: &League) -> Option<String> {
    let last_round = competition.knockout_rounds.last()?;
    if !last_round.completed || last_round.fixture_ids.len() != 1 {
        return None;
    }
    let final_fixture = competition
        .fixtures
        .iter()
        .find(|fixture| last_round.fixture_ids.contains(&fixture.id))?;
    let result = final_fixture.result.as_ref()?;
    Some(if result.home_goals >= result.away_goals {
        final_fixture.home_team_id.clone()
    } else {
        final_fixture.away_team_id.clone()
    })
}

fn announce_champion_if_decided(game: &mut Game, competition_index: usize, today: &str) {
    let competition = &game.competitions[competition_index];
    let Some(champion_id) = world_cup_champion(competition) else {
        return;
    };
    let year = competition.season;
    let msg_id = format!("world_cup_champion_{year}");
    if game.messages.iter().any(|message| message.id == msg_id) {
        return;
    }

    let nation = game
        .national_teams
        .iter()
        .find(|team| team.id == champion_id)
        .map(|team| team.name.clone())
        .unwrap_or(champion_id);

    let mut params = std::collections::HashMap::new();
    params.insert("nation".to_string(), nation);
    params.insert("year".to_string(), year.to_string());

    let message = InboxMessage::new(
        msg_id,
        String::new(),
        String::new(),
        String::new(),
        today.to_string(),
    )
    .with_category(MessageCategory::LeagueInfo)
    .with_priority(MessagePriority::High)
    .with_sender_role("")
    .with_i18n(
        "be.msg.worldCupChampion.subject",
        "be.msg.worldCupChampion.body",
        params,
    )
    .with_sender_i18n("be.sender.intlLiaison", "be.role.intlLiaison");
    game.messages.push(message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::TimeZone;
    use domain::manager::Manager;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn kickoff(year: i32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, 6, 10, 0, 0, 0).unwrap()
    }

    fn empty_game() -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap());
        let manager = Manager::new(
            "mgr".to_string(),
            "Alex".to_string(),
            "Boss".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        Game::new(clock, manager, vec![], vec![], vec![], vec![])
    }

    #[test]
    fn world_cup_summers_follow_the_real_calendar() {
        assert!(is_world_cup_summer(2022));
        assert!(is_world_cup_summer(2026));
        assert!(is_world_cup_summer(2030));
        assert!(!is_world_cup_summer(2024));
        assert!(!is_world_cup_summer(2025));
    }

    #[test]
    fn schedules_a_full_field_by_synthesising_missing_nations() {
        let mut game = empty_game();
        let players_before = game.players.len();

        schedule_world_cup(&mut game, kickoff(2026), &FORMAT_16);

        let cup = game
            .competitions
            .iter()
            .find(|c| is_world_cup_competition(c))
            .expect("a World Cup must be scheduled");
        assert_eq!(cup.participant_ids.len(), 16);
        assert_eq!(cup.groups.len(), 4);
        assert_eq!(cup.name, "World Cup 2026");
        // An empty world needed everything generated: 16 squads of free agents.
        assert_eq!(game.players.len(), players_before + 16 * 18);
        assert!(game.players.iter().all(|p| p.team_id.is_none()));
        // Every participant has a real national squad.
        for participant in &cup.participant_ids {
            let team = game
                .national_teams
                .iter()
                .find(|t| &t.id == participant)
                .expect("participant national team exists");
            assert!(team.squad_player_ids.len() >= 11);
        }
    }

    #[test]
    fn is_due_respects_the_calendar_and_never_doubles_up() {
        let mut game = empty_game();

        assert!(!schedule_world_cup_if_due(&mut game, kickoff(2025)));
        assert!(game.competitions.is_empty());

        assert!(schedule_world_cup_if_due(&mut game, kickoff(2026)));
        assert_eq!(
            game.competitions
                .iter()
                .filter(|c| is_world_cup_competition(c))
                .count(),
            1
        );

        assert!(
            !schedule_world_cup_if_due(&mut game, kickoff(2026)),
            "a running World Cup must not be scheduled twice"
        );
    }

    #[test]
    fn the_tournament_plays_to_a_champion_with_carry_back() {
        let mut game = empty_game();
        schedule_world_cup(&mut game, kickoff(2026), &FORMAT_16);
        let mut rng = StdRng::seed_from_u64(7);

        // Play every date in order until no fixtures remain scheduled.
        for _ in 0..200 {
            let next_date = game
                .competitions
                .iter()
                .filter(|c| is_world_cup_competition(c))
                .flat_map(|c| c.fixtures.iter())
                .filter(|f| f.status == FixtureStatus::Scheduled)
                .map(|f| f.date.clone())
                .min();
            let Some(date) = next_date else {
                break;
            };
            assert!(process_world_cup_fixtures_due(&mut game, &date, &mut rng) > 0);
        }

        let cup = game
            .competitions
            .iter()
            .find(|c| is_world_cup_competition(c))
            .unwrap();
        assert!(cup.knockout_rounds.iter().all(|round| round.completed));
        assert_eq!(cup.knockout_rounds.last().unwrap().fixture_ids.len(), 1);

        let champion = world_cup_champion(cup).expect("the final decides a champion");
        assert!(champion.starts_with("nt-"));
        let message = game
            .messages
            .iter()
            .find(|m| m.id == "world_cup_champion_2026")
            .expect("the champion is announced");
        assert_eq!(
            message.subject_key.as_deref(),
            Some("be.msg.worldCupChampion.subject")
        );
        assert!(message.i18n_params.contains_key("nation"));

        // Carry-back reached the squads: tournament players show fatigue.
        assert!(game.players.iter().any(|p| p.condition < 100));
    }
}
