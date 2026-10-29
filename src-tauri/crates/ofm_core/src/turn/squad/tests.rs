use super::*;
use crate::turn::matchday_lookups::MatchdayLookups;
use domain::player::{Player, PlayerAttributes, Position as DomainPos};

/// Uniform attributes: `weighted_score` averages attributes, so setting them
/// all to `v` makes the condition-free positional fit ≈ `v` for any slot, with
/// the per-slot compatibility/foot penalty identical across players (same
/// position + footedness) — so it cancels in within-slot comparisons.
pub(super) fn attrs(v: u8) -> PlayerAttributes {
    PlayerAttributes {
        pace: v,
        stamina: v,
        strength: v,
        agility: v,
        passing: v,
        shooting: v,
        tackling: v,
        dribbling: v,
        defending: v,
        positioning: v,
        vision: v,
        decisions: v,
        composure: v,
        aggression: v,
        teamwork: v,
        leadership: v,
        handling: v,
        reflexes: v,
        aerial: v,
    }
}

pub(super) fn mk(id: &str, attr: u8, condition: u8) -> Player {
    mk_pos(id, DomainPos::CenterBack, attr, condition)
}

pub(super) fn mk_pos(id: &str, position: DomainPos, attr: u8, condition: u8) -> Player {
    let mut p = Player::new(
        id.to_string(),
        id.to_string(),
        id.to_string(),
        "1998-01-01".to_string(),
        "GB".to_string(),
        position,
        attrs(attr),
    );
    p.condition = condition;
    p
}

#[test]
fn management_quality_maps_reputation_to_unit_range() {
    assert_eq!(management_quality(300), 0.0);
    assert_eq!(management_quality(900), 1.0);
    assert!((management_quality(600) - 0.5).abs() < 1e-9);
    assert_eq!(management_quality(100), 0.0); // clamped below
    assert_eq!(management_quality(1200), 1.0); // clamped above
}

/// The discriminating test: an elite club must NOT bench a strong (but mildly
/// tired) starter for a much weaker fresh player. Better clubs field better
/// teams — the gap-aware tolerance enforces this.
#[test]
fn elite_club_keeps_strong_starters_over_fresh_scrubs() {
    let mut squad = Vec::new();
    for i in 0..11 {
        squad.push(mk(&format!("star{i}"), 80, 70)); // strong, mildly tired
    }
    for i in 0..3 {
        squad.push(mk(&format!("weak{i}"), 50, 100)); // weak, fully fresh
    }
    let refs: Vec<&Player> = squad.iter().collect();

    let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0, 1, FixtureLoad::Normal);

    assert_eq!(xi.len(), 11);
    assert!(
        xi.iter().all(|p| p.id.starts_with("star")),
        "elite club fielded a weak fresh player over a strong starter: {:?}",
        xi.iter().map(|p| &p.id).collect::<Vec<_>>()
    );
}

/// An elite club rotates a tired starter for a *comparable* fresh deputy
/// (within the fit tolerance), but still leaves the much-weaker scrubs benched.
#[test]
fn elite_club_rotates_tired_starter_for_comparable_fresh_player() {
    let mut squad = Vec::new();
    for i in 0..11 {
        squad.push(mk(&format!("star{i}"), 80, 65)); // strong, tired
    }
    squad.push(mk("deputy", 72, 100)); // comparable, fresh
    for i in 0..2 {
        squad.push(mk(&format!("weak{i}"), 50, 100)); // scrub, fresh
    }
    let refs: Vec<&Player> = squad.iter().collect();

    let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0, 1, FixtureLoad::Normal);

    assert_eq!(xi.len(), 11);
    assert!(
        xi.iter().any(|p| p.id == "deputy"),
        "comparable fresh deputy should rotate in for a tired star"
    );
    assert_eq!(
        xi.iter().filter(|p| p.id.starts_with("star")).count(),
        10,
        "exactly one tired star should be rested"
    );
    assert!(
        xi.iter().all(|p| !p.id.starts_with("weak")),
        "scrubs are too far below tolerance to be rotated in"
    );
}

/// Same squad as above, but a poorly-run club: it is slow to notice fatigue and
/// rides a *mildly* tired XI. This is the reputation gradient, and it survives —
/// what it is not allowed to do is ride an exhausted one, which is the test
/// below.
#[test]
fn low_reputation_club_rides_mildly_tired_starters() {
    let mut squad = Vec::new();
    for i in 0..11 {
        squad.push(mk(&format!("star{i}"), 80, 65));
    }
    squad.push(mk("deputy", 72, 100));
    for i in 0..2 {
        squad.push(mk(&format!("weak{i}"), 50, 100));
    }
    let refs: Vec<&Player> = squad.iter().collect();

    let xi = ai_select_starting_xi(
        &refs,
        "4-4-2",
        management_quality(300),
        1,
        FixtureLoad::Normal,
    ); // q = 0

    assert_eq!(xi.len(), 11);
    assert!(
        xi.iter().all(|p| p.id.starts_with("star")),
        "a low-reputation club should ride its mildly tired first XI, not rotate"
    );
}

/// The behaviour this slice exists to change. A manager may be slow, or a poor
/// judge of who the better player is; no manager knowingly sends out a player
/// who can barely run when a rested deputy is available. Under the old
/// `fit_tolerance = 12 × quality` the worst manager accepted a quality drop of
/// exactly zero, so it could never rotate at all.
#[test]
fn even_the_worst_manager_rests_an_exhausted_starter() {
    let mut squad = Vec::new();
    for i in 0..11 {
        squad.push(mk(&format!("star{i}"), 80, 22)); // first choice, spent
    }
    squad.push(mk("deputy", 66, 95)); // clearly worse, but able to play
    let refs: Vec<&Player> = squad.iter().collect();

    let xi = ai_select_starting_xi(
        &refs,
        "4-4-2",
        management_quality(300),
        1,
        FixtureLoad::Normal,
    ); // q = 0

    assert_eq!(xi.len(), 11);
    assert!(
        xi.iter().any(|p| p.id == "deputy"),
        "an exhausted XI must be broken up even by the worst manager: {:?}",
        xi.iter().map(|p| &p.id).collect::<Vec<_>>()
    );
}

/// A universal floor is not a licence to field anybody. Even exhausted, a club
/// does not replace its best player with someone hopeless.
#[test]
fn an_exhausted_starter_is_not_replaced_by_a_hopeless_deputy() {
    let mut squad = Vec::new();
    for i in 0..11 {
        squad.push(mk(&format!("star{i}"), 80, 22));
    }
    squad.push(mk("hopeless", 30, 100));
    let refs: Vec<&Player> = squad.iter().collect();

    let xi = ai_select_starting_xi(
        &refs,
        "4-4-2",
        management_quality(300),
        1,
        FixtureLoad::Normal,
    );

    assert_eq!(xi.len(), 11);
    assert!(
        xi.iter().all(|p| p.id != "hopeless"),
        "a 30-rated player is not an adequate deputy at any fatigue level"
    );
}

/// Quality is judgement, not willingness. Given two deputies of nearly equal
/// standard, an elite manager always identifies the better one; a poor manager
/// gets it wrong some of the time. Fixed seeds, so this is deterministic.
#[test]
fn a_poor_manager_misjudges_which_deputy_is_better() {
    let pick_deputy = |quality: f64, seed: u64| -> String {
        let mut squad = Vec::new();
        for i in 0..11 {
            squad.push(mk(&format!("star{i}"), 80, 22));
        }
        squad.push(mk("better", 70, 95));
        squad.push(mk("worse", 66, 95));
        let refs: Vec<&Player> = squad.iter().collect();
        let xi = ai_select_starting_xi(&refs, "4-4-2", quality, seed, FixtureLoad::Normal);
        xi.iter()
            .find(|p| !p.id.starts_with("star"))
            .map(|p| p.id.clone())
            .unwrap_or_default()
    };

    let seeds: Vec<u64> = (0..24).collect();
    let elite: Vec<String> = seeds.iter().map(|s| pick_deputy(1.0, *s)).collect();
    let poor: Vec<String> = seeds.iter().map(|s| pick_deputy(0.0, *s)).collect();

    // Stated first and deliberately: without it, a manager that never rotated
    // at all would satisfy "sometimes picks the worse one" by picking nobody,
    // which is how the old behaviour would have passed this test.
    assert!(
        poor.iter().all(|d| d == "better" || d == "worse"),
        "a poor manager must still name a deputy for an exhausted XI: {poor:?}"
    );
    assert!(
        elite.iter().all(|d| d == "better"),
        "an elite manager should always spot the better deputy: {elite:?}"
    );
    assert!(
        poor.iter().any(|d| d == "worse"),
        "a poor manager should sometimes pick the worse deputy; got it right every time"
    );
}

/// The seed is the club and the date, so one fixture always produces one XI
/// however many times the builder is asked for it.
#[test]
fn the_same_seed_always_names_the_same_side() {
    let mut squad = Vec::new();
    for i in 0..11 {
        squad.push(mk(&format!("star{i}"), 80, 22));
    }
    squad.push(mk("a", 70, 95));
    squad.push(mk("b", 69, 95));
    let refs: Vec<&Player> = squad.iter().collect();

    let first = ai_select_starting_xi(&refs, "4-4-2", 0.0, 7, FixtureLoad::Normal);
    let second = ai_select_starting_xi(&refs, "4-4-2", 0.0, 7, FixtureLoad::Normal);

    let ids = |xi: &Vec<&Player>| xi.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&first), ids(&second));
}

/// Regression: load-management rotation must not skew the formation's
/// position distribution. A tired XI plus one fresh midfielder must still
/// field 1 GK / 4 DEF / 4 MID / 2 FWD — never a defender short (which
/// rendered only 10 players on the pitch).
#[test]
fn rotation_preserves_formation_position_distribution() {
    use DomainPos::{
        CenterBack, CentralMidfielder, Forward, Goalkeeper, LeftBack, LeftMidfielder, RightBack,
        RightMidfielder, Striker,
    };
    let squad = vec![
        mk_pos("gk", Goalkeeper, 75, 65),
        mk_pos("d1", CenterBack, 75, 65),
        mk_pos("d2", CenterBack, 75, 65),
        mk_pos("d3", LeftBack, 75, 65),
        mk_pos("d4", RightBack, 75, 65),
        mk_pos("m1", CentralMidfielder, 75, 65),
        mk_pos("m2", CentralMidfielder, 75, 65),
        mk_pos("m3", LeftMidfielder, 75, 65),
        mk_pos("m4", RightMidfielder, 75, 65),
        mk_pos("f1", Striker, 75, 65),
        mk_pos("f2", Striker, 75, 65),
        // Fresh midfielder load management will want to bring in.
        mk_pos("m_fresh", CentralMidfielder, 75, 100),
    ];
    let refs: Vec<&Player> = squad.iter().collect();

    let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0, 1, FixtureLoad::Normal); // elite: rotates eagerly

    assert_eq!(xi.len(), 11);
    let group_count = |group: DomainPos| {
        xi.iter()
            .filter(|p| p.position.to_group_position() == group)
            .count()
    };
    assert_eq!(group_count(Goalkeeper), 1, "exactly one keeper");
    assert_eq!(
        group_count(DomainPos::Defender),
        4,
        "must field four defenders"
    );
    assert_eq!(
        group_count(DomainPos::Midfielder),
        4,
        "must field four midfielders"
    );
    assert_eq!(group_count(Forward), 2, "must field two forwards");
}

/// With fewer available players than formation slots, the slot-aligned fill
/// leaves gaps; the function must fall back to a contiguous selection rather
/// than flatten()ing those gaps (which would shift starters into wrong slots).
#[test]
fn select_starting_xi_falls_back_when_fewer_players_than_slots() {
    let squad: Vec<Player> = (0..9)
        .map(|i| mk_pos(&format!("p{i}"), DomainPos::CenterBack, 70, 100))
        .collect();
    let refs: Vec<&Player> = squad.iter().collect();
    let saved: Vec<String> = (0..9).map(|i| format!("p{i}")).collect();

    let xi = select_starting_xi(&saved, &refs, "4-4-2");

    // Every available player fielded once, with no slot-misaligning gaps.
    assert_eq!(xi.len(), 9);
    let unique: HashSet<&String> = xi.iter().map(|p| &p.id).collect();
    assert_eq!(unique.len(), 9);
}

/// When a saved starter is unavailable the user XI compacts; the surviving
/// starters must keep their real saved slot (matching deployed_position /
/// the UI), not shift up into the vacated slot. Regression for an injured
/// keeper turning an outfielder into the engine's goalkeeper.
#[test]
fn user_team_starter_keeps_saved_slot_when_xi_compacts() {
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::Team;

    let mut players: Vec<Player> = (1..=11)
        .map(|i| mk_pos(&format!("p{i}"), DomainPos::CenterBack, 70, 100))
        .collect();
    // p1 is a natural Forward saved into the LEFT-BACK slot (index 1).
    players[0] = mk_pos("p1", DomainPos::Forward, 70, 100);
    for player in players.iter_mut() {
        player.team_id = Some("user".to_string());
    }

    let mut team = Team::new(
        "user".to_string(),
        "User FC".to_string(),
        "USR".to_string(),
        "England".to_string(),
        "London".to_string(),
        "Ground".to_string(),
        25_000,
    );
    team.formation = "4-4-2".to_string();
    // Slot 0 (GK) references a player that no longer exists, so
    // select_starting_xi drops it and the survivors compact.
    team.starting_xi_ids = std::iter::once("ghost-gk".to_string())
        .chain((1..=10).map(|i| format!("p{i}")))
        .collect();

    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("user".to_string());

    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
    let game = Game::new(clock, manager, vec![team], players, vec![], vec![]);

    let (team_data, _bench) = build_team_with_bench(&game, "user");

    let p1 = team_data
        .players
        .iter()
        .find(|p| p.id == "p1")
        .expect("p1 should be fielded");
    // Saved at the left-back slot -> simulated as a defender, never shoved
    // into the vacated goalkeeper slot.
    assert_eq!(p1.position, Position::Defender);
    assert_ne!(p1.position, Position::Goalkeeper);

    // The vacated goalkeeper slot must be refilled, so the XI still fields
    // exactly one keeper (otherwise the engine's goalkeeper rating collapses
    // to its empty-set fallback).
    let keepers = team_data
        .players
        .iter()
        .filter(|p| p.position == Position::Goalkeeper)
        .count();
    assert_eq!(keepers, 1, "the XI must still contain a goalkeeper");
}

// ---------------------------------------------------------------------------
// Load management on a congested run
// ---------------------------------------------------------------------------

/// The mirror of `low_reputation_club_rides_mildly_tired_starters`: the same
/// side, the same manager. On a normal week he rides a first eleven at 65 and
/// is right to — it has seven days to recover. With another match three or
/// four days away it will not recover in time, and even he rests one.
#[test]
fn on_a_congested_run_even_a_low_reputation_club_rests_a_tired_starter() {
    let mut squad = Vec::new();
    for i in 0..11 {
        squad.push(mk(&format!("star{i}"), 80, 65));
    }
    squad.push(mk("deputy", 72, 100));
    let refs: Vec<&Player> = squad.iter().collect();

    let xi = ai_select_starting_xi(
        &refs,
        "4-4-2",
        management_quality(300),
        1,
        FixtureLoad::Congested,
    ); // q = 0

    assert!(
        xi.iter().any(|p| p.id == "deputy"),
        "with another match days away, a first eleven at 65 should not all start"
    );
}

/// Congestion makes a manager rest players sooner, not field anybody. A starter
/// who is tired but far from spent stays on rather than make way for a player
/// well below his standard.
#[test]
fn a_congested_run_does_not_make_a_manager_field_a_much_weaker_player() {
    let mut squad = Vec::new();
    for i in 0..11 {
        squad.push(mk(&format!("star{i}"), 80, 70));
    }
    squad.push(mk("reserve", 60, 100)); // twenty points short
    let refs: Vec<&Player> = squad.iter().collect();

    // A poor manager, because that is where the congested tolerance floor is
    // the whole tolerance: at elite quality it grows to the same 12 on any week.
    let xi = ai_select_starting_xi(
        &refs,
        "4-4-2",
        management_quality(300),
        1,
        FixtureLoad::Congested,
    ); // q = 0

    assert!(
        xi.iter().all(|p| p.id.starts_with("star")),
        "a reserve twenty points short is not rotation, it is a weaker team"
    );
}

/// A world where `club` has fixtures on the given days from the clock's today.
fn game_with_fixtures_on(days_from_today: &[i64]) -> Game {
    use crate::clock::GameClock;
    use chrono::{Duration, TimeZone, Utc};
    use domain::league::{Fixture, FixtureCompetition, FixtureStatus, League};
    use domain::manager::Manager;

    let today = Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap();
    let fixtures = days_from_today
        .iter()
        .map(|days| Fixture {
            id: format!("f{days}"),
            date: (today + Duration::days(*days))
                .format("%Y-%m-%d")
                .to_string(),
            home_team_id: "club".to_string(),
            away_team_id: "rival".to_string(),
            competition: FixtureCompetition::League,
            status: FixtureStatus::Scheduled,
            ..Default::default()
        })
        .collect();

    let manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    let mut game = Game::new(
        GameClock::new(today),
        manager,
        vec![],
        vec![],
        vec![],
        vec![],
    );
    game.league = Some(League {
        id: "league".to_string(),
        fixtures,
        ..Default::default()
    });
    game
}

/// Midweek to weekend: another match three days after today's.
#[test]
fn a_match_three_days_after_this_one_is_a_congested_run() {
    let game = game_with_fixtures_on(&[0, 3]);
    assert_eq!(fixture_load(&game, "club"), FixtureLoad::Congested);
}

/// Weekend to weekend. Today's own match is still `Scheduled` at kick-off, which
/// is exactly why this must not count it — training's "two fixtures in the
/// coming week" would, and would call every ordinary week congested.
#[test]
fn a_match_a_week_after_this_one_is_a_normal_week() {
    let game = game_with_fixtures_on(&[0, 7]);
    assert_eq!(fixture_load(&game, "club"), FixtureLoad::Normal);
}

/// The edges of the window: tomorrow and four days on count; today's own match, yesterday's
/// and five days on do not. A fixture already played does not count wherever it falls.
#[test]
fn only_the_next_four_days_make_a_congested_run() {
    for (days, congested) in [(-1, false), (0, false), (1, true), (4, true), (5, false)] {
        let game = game_with_fixtures_on(&[days]);
        let expected = if congested {
            FixtureLoad::Congested
        } else {
            FixtureLoad::Normal
        };
        assert_eq!(
            fixture_load(&game, "club"),
            expected,
            "a match {days} days on"
        );
    }
    let mut played = game_with_fixtures_on(&[3]);
    played.league.as_mut().unwrap().fixtures[0].status = domain::league::FixtureStatus::Completed;
    assert_eq!(fixture_load(&played, "club"), FixtureLoad::Normal);
}

/// Given a world where two clubs play again in three days and a third does not, When every club's
/// load is read off the set found in one pass, Then each answer is the one the fixtures give.
#[test]
fn the_clubs_found_in_one_pass_are_the_clubs_the_fixtures_make_congested() {
    let game = game_with_fixtures_on(&[0, 3]);
    let found = MatchdayLookups::of(&game);

    for club in ["club", "rival", "bystander"] {
        assert_eq!(
            fixture_load_among(&game, club, Some(&found)),
            fixture_load(&game, club),
            "{club}"
        );
    }
    assert!(found.is_congested("club") && found.is_congested("rival"));
    assert!(!found.is_congested("bystander"));
}

/// The state a squad is actually picked in on a normal day advance: the
/// competition being played has been moved into the legacy `game.league` slot
/// for simulation, and its place in `game.competitions` holds an empty default.
/// The next round of that same competition lives only in `game.league`.
#[test]
fn a_competition_moved_out_for_simulation_still_counts() {
    let mut game = game_with_fixtures_on(&[0, 3]);
    game.competitions = vec![domain::league::League::default()];
    assert_eq!(fixture_load(&game, "club"), FixtureLoad::Congested);
}

/// A club's calendar is the same whichever competition a fixture is in: one rule for every club
/// (#869), as training's matchday and taper already read it. A fixture in a competition outside the
/// active scope is still a next match, so it makes a congested run.
///
/// Given a club with a fixture today and another three days on, in a competition nobody follows,
/// When its fixture load is read,
/// Then the run is congested.
#[test]
fn a_fixture_in_a_dormant_competition_is_a_congested_run() {
    let mut game = game_with_fixtures_on(&[0]);
    let active = game.league.clone().expect("the active league");
    let mut dormant = game_with_fixtures_on(&[3])
        .league
        .expect("a league to borrow fixtures from");
    dormant.id = "dormant".to_string();
    game.active_competition_ids = vec![active.id.clone()];
    game.competitions = vec![active, dormant];

    assert_eq!(fixture_load(&game, "club"), FixtureLoad::Congested);
}

/// Ten fit outfielders and a goalkeeper in the treatment room — a club that
/// has to field a sore keeper or put a defender in goal.
pub(super) fn club_whose_only_keeper_is_injured(managed_by_the_user: bool) -> Game {
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::Injury;
    use domain::team::Team;

    let mut keeper = mk_pos("keeper", DomainPos::Goalkeeper, 70, 100);
    keeper.injury = Some(Injury {
        name: "common.injuries.calfStrain".to_string(),
        days_remaining: 3,
    });
    let mut players = vec![keeper];
    let outfield = [
        (DomainPos::CenterBack, 4),
        (DomainPos::CentralMidfielder, 4),
        (DomainPos::Striker, 2),
    ];
    for (position, count) in outfield {
        for i in 0..count {
            players.push(mk_pos(
                &format!("{position:?}{i}"),
                position.clone(),
                70,
                100,
            ));
        }
    }
    for player in players.iter_mut() {
        player.team_id = Some("club".to_string());
    }

    let mut team = Team::new(
        "club".to_string(),
        "Club".to_string(),
        "CLB".to_string(),
        "England".to_string(),
        "London".to_string(),
        "Ground".to_string(),
        25_000,
    );
    team.formation = "4-4-2".to_string();

    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire(
        if managed_by_the_user {
            "club"
        } else {
            "elsewhere"
        }
        .to_string(),
    );

    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
    Game::new(clock, manager, vec![team], players, vec![], vec![])
}

fn who_keeps_goal(game: &Game) -> Vec<String> {
    let (team_data, _bench) = build_team_with_bench(game, "club");
    team_data
        .players
        .iter()
        .filter(|p| p.position == Position::Goalkeeper)
        .map(|p| p.id.clone())
        .collect()
}

/// Called up from the treatment room to make up the numbers, the keeper goes in
/// goal — not into whichever slot happened to be left over, which in a 4-4-2 is
/// up front, with a centre-back in goal in his place.
#[test]
fn an_injured_keeper_called_up_by_an_ai_club_plays_in_goal() {
    let game = club_whose_only_keeper_is_injured(false);
    assert_eq!(who_keeps_goal(&game), vec!["keeper".to_string()]);
}

#[test]
fn an_injured_keeper_called_up_by_the_user_s_club_plays_in_goal() {
    let game = club_whose_only_keeper_is_injured(true);
    assert_eq!(who_keeps_goal(&game), vec!["keeper".to_string()]);
}

/// A role reaches the engine only where the position a player is actually
/// playing admits it — the rule the player's own role command already
/// enforces. Generated squads are coarse, and the coarse `Forward` bucket
/// admits `WideForward`; deployed at centre-forward in a 4-4-2 he would still
/// have played with the wide forward's attribute weighting.
#[test]
fn a_role_the_deployed_slot_does_not_admit_is_not_played() {
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::{PlayerRole, Team};
    use engine::PlayerRole as EngineRole;

    let mut players = vec![mk_pos("keeper", DomainPos::Goalkeeper, 70, 100)];
    for (position, count) in [
        (DomainPos::CenterBack, 4),
        (DomainPos::CentralMidfielder, 4),
        (DomainPos::Forward, 2),
    ] {
        for i in 0..count {
            players.push(mk_pos(
                &format!("{position:?}{i}"),
                position.clone(),
                70,
                100,
            ));
        }
    }
    for player in players.iter_mut() {
        player.team_id = Some("club".to_string());
    }

    let mut team = Team::new(
        "club".to_string(),
        "Club".to_string(),
        "CLB".to_string(),
        "England".to_string(),
        "London".to_string(),
        "Ground".to_string(),
        25_000,
    );
    team.formation = "4-4-2".to_string();
    team.player_roles
        .insert("Forward0".to_string(), PlayerRole::WideForward);
    team.player_roles
        .insert("Forward1".to_string(), PlayerRole::Poacher);

    let mut manager = Manager::new(
        "mgr".to_string(),
        "Test".to_string(),
        "Manager".to_string(),
        "1980-01-01".to_string(),
        "England".to_string(),
    );
    manager.hire("elsewhere".to_string());
    let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
    let game = Game::new(clock, manager, vec![team], players, vec![], vec![]);

    let (team_data, _bench) = build_team_with_bench(&game, "club");
    let role_of = |id: &str| {
        team_data
            .players
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("{id} should start"))
            .role
    };

    assert_eq!(
        role_of("Forward0"),
        EngineRole::Standard,
        "a wide forward's role was played at centre-forward"
    );
    assert_eq!(
        role_of("Forward1"),
        EngineRole::Poacher,
        "a role the striker's slot admits must still reach the engine"
    );
}

/// Given the domain's position groups and the roles each admits, When every role is bridged to
/// the engine, Then the engine files it under the same groups, and its role list misses none.
#[test]
fn the_engines_role_groups_match_the_domains() {
    use engine::Position as EnginePosition;

    let groups = [
        (DomainPos::Goalkeeper, EnginePosition::Goalkeeper),
        (DomainPos::Defender, EnginePosition::Defender),
        (DomainPos::Midfielder, EnginePosition::Midfielder),
        (DomainPos::Forward, EnginePosition::Forward),
    ];
    let mut bridged = Vec::new();
    for (domain_group, engine_group) in &groups {
        for role in domain_group.valid_roles() {
            let engine_role = domain_to_engine_role(role);
            if engine_role != EnginePlayerRole::Standard {
                assert!(
                    engine_role.position_groups().contains(engine_group),
                    "{role:?} is admitted by {domain_group:?} but not filed under it in the engine"
                );
            }
            bridged.push(engine_role);
        }
    }
    for engine_role in EnginePlayerRole::ALL {
        assert!(
            bridged.contains(&engine_role),
            "{engine_role:?} is in the engine's list but no domain position admits it"
        );
        for engine_group in engine_role.position_groups() {
            let (domain_group, _) = groups.iter().find(|(_, g)| g == engine_group).unwrap();
            let admitted = domain_group
                .valid_roles()
                .iter()
                .any(|role| domain_to_engine_role(role) == engine_role);
            assert!(
                admitted,
                "{engine_role:?} is filed under {engine_group:?} but the domain does not admit it there"
            );
        }
    }
    for engine_role in bridged {
        assert!(
            EnginePlayerRole::ALL.contains(&engine_role),
            "{engine_role:?} is missing from the engine's list of roles"
        );
    }
}

/// Given a club and the world's other players, When its team is built from the club's players
/// found once, Then team, bench and set-piece takers are those built by scanning the world.
#[test]
fn a_team_built_from_the_clubs_players_is_the_team_built_by_scanning() {
    for is_user in [true, false] {
        let game = club_whose_only_keeper_is_injured(is_user);
        let club = crate::match_day_eligibility::club_players(&game.players, "club");

        let from_club = build_team_with_bench_from(&game, "club", &club, None);
        let with_the_set =
            build_team_with_bench_from(&game, "club", &club, Some(&MatchdayLookups::of(&game)));
        assert_eq!(format!("{from_club:?}"), format!("{with_the_set:?}"));
        let by_scanning = build_team_with_bench(&game, "club");

        assert_eq!(format!("{from_club:?}"), format!("{by_scanning:?}"));
        let starters: Vec<String> = by_scanning.0.players.iter().map(|p| p.id.clone()).collect();
        let starting: Vec<&Player> = starters
            .iter()
            .filter_map(|id| club.iter().find(|p| &p.id == id).copied())
            .collect();
        assert_eq!(
            set_piece_takers_among(&starting),
            auto_select_set_pieces(&game, &starters)
        );
    }
}

/// A full, fit 4-4-2 squad for one club, managed by someone else.
fn a_fit_four_four_two_club() -> Game {
    let mut game = club_whose_only_keeper_is_injured(false);
    for player in game.players.iter_mut() {
        player.injury = None;
    }
    game
}

/// Given a club set to play each of the six styles in turn
/// When its team is built for the engine
/// Then the engine receives that style: none of them arrives as Balanced unless it is Balanced
#[test]
fn every_play_style_reaches_the_engine_as_itself() {
    use domain::team::PlayStyle as DomainStyle;
    let mut game = a_fit_four_four_two_club();
    let styles: Vec<PlayStyle> = [
        DomainStyle::Balanced,
        DomainStyle::Attacking,
        DomainStyle::Defensive,
        DomainStyle::Possession,
        DomainStyle::Counter,
        DomainStyle::HighPress,
    ]
    .into_iter()
    .map(|style| {
        game.teams[0].play_style = style;
        build_team_with_bench(&game, "club").0.play_style
    })
    .collect();

    assert_eq!(
        styles,
        vec![
            PlayStyle::Balanced,
            PlayStyle::Attacking,
            PlayStyle::Defensive,
            PlayStyle::Possession,
            PlayStyle::Counter,
            PlayStyle::HighPress,
        ]
    );
}

/// Given a club with a fit goalkeeper, four centre-backs, four central midfielders and two
///       strikers, set up 4-4-2
/// When its team is built for the engine
/// Then the engine fields one goalkeeper, four defenders, four midfielders and two forwards:
///      the engine picks its shooters by position, so a forward must arrive as a forward
#[test]
fn a_four_four_two_reaches_the_engine_with_two_forwards() {
    let game = a_fit_four_four_two_club();

    let (team, _bench) = build_team_with_bench(&game, "club");

    assert_eq!(
        [
            team.count_position(Position::Goalkeeper),
            team.count_position(Position::Defender),
            team.count_position(Position::Midfielder),
            team.count_position(Position::Forward),
        ],
        [1, 4, 4, 2]
    );
    assert_eq!(who_keeps_goal(&game), vec!["keeper".to_string()]);
}

/// Given one club on two dates, and two clubs on one date
/// When each is asked for its matchday selection seed
/// Then every pair differs: a manager's judgement changes with the day and with the club
#[test]
fn the_selection_seed_differs_by_club_and_by_matchday() {
    let seed = selection_seed("a", "2026-08-01");

    assert_ne!(seed, selection_seed("a", "2026-08-02"));
    assert_ne!(seed, selection_seed("b", "2026-08-01"));
}

/// Eleven 80-rated centre-backs in the given condition and one fresh deputy rated
/// `deputy_rating`, as a starting eleven picked by a manager of `quality` under `load`.
fn eleven_picked(
    starter_condition: u8,
    deputy_rating: u8,
    quality: f64,
    load: FixtureLoad,
) -> Vec<String> {
    let mut squad: Vec<Player> = (0..11)
        .map(|i| mk(&format!("star{i}"), 80, starter_condition))
        .collect();
    squad.push(mk("deputy", deputy_rating, 100));
    let refs: Vec<&Player> = squad.iter().collect();
    ai_select_starting_xi(&refs, "4-4-2", quality, 1, load)
        .into_iter()
        .map(|player| player.id.clone())
        .collect()
}

/// Whether the deputy is in a full eleven. A short eleven would hold no deputy whatever the
/// rotation did, so it fails here rather than passing a "keeps his first eleven" test.
fn rotates_in_the_deputy(eleven: &[String]) -> bool {
    assert_eq!(eleven.len(), 11, "a full eleven: {eleven:?}");
    eleven.iter().any(|id| id == "deputy")
}

/// Given the poorest manager, a first eleven at condition 60, and a fresh deputy four below them
/// When he picks his eleven on a normal week
/// Then he keeps his first eleven: 60 is above the condition at which he starts resting anyone
#[test]
fn a_poor_manager_keeps_a_starter_above_his_fatigue_threshold() {
    assert!(!rotates_in_the_deputy(&eleven_picked(
        60,
        76,
        0.0,
        FixtureLoad::Normal
    )));
}

/// Given the poorest manager, a first eleven at condition 48 (tired, not exhausted), and a
///       fresh deputy ten below them
/// When he picks his eleven on a normal week
/// Then he keeps his first eleven: he accepts a drop of six at most to rest someone tired
#[test]
fn a_poor_manager_will_not_rest_a_tired_starter_for_a_deputy_ten_below() {
    assert!(!rotates_in_the_deputy(&eleven_picked(
        48,
        70,
        0.0,
        FixtureLoad::Normal
    )));
}

/// Given the best manager, a first eleven at condition 70, and a fresh deputy ten below them
/// When he picks his eleven on a normal week
/// Then he rests one starter for the deputy: he accepts a drop of up to twelve
#[test]
fn an_elite_manager_rests_a_tired_starter_for_a_deputy_ten_below() {
    assert!(rotates_in_the_deputy(&eleven_picked(
        70,
        70,
        1.0,
        FixtureLoad::Normal
    )));
}

/// Given the best manager, a first eleven at condition 70, and a fresh deputy twenty below them
/// When he picks his eleven on a normal week
/// Then he keeps his first eleven: twenty is past the twelve he accepts
#[test]
fn an_elite_manager_does_not_rest_a_tired_starter_for_a_deputy_twenty_below() {
    assert!(!rotates_in_the_deputy(&eleven_picked(
        70,
        60,
        1.0,
        FixtureLoad::Normal
    )));
}

/// Given the best manager on a congested run, a first eleven at condition 85, and a fresh deputy
///       five below them
/// When he picks his eleven
/// Then he rests one starter: on a congested run he rests anyone below 100
#[test]
fn on_a_congested_run_an_elite_manager_rests_a_starter_at_85_condition() {
    assert!(rotates_in_the_deputy(&eleven_picked(
        85,
        75,
        1.0,
        FixtureLoad::Congested
    )));
}

/// Given the poorest manager on a congested run, a first eleven at condition 90, and a fresh
///       deputy five below them
/// When he picks his eleven
/// Then he keeps his first eleven: on a congested run the poorest manager rests below 90
#[test]
fn on_a_congested_run_the_poorest_manager_keeps_a_starter_at_90_condition() {
    assert!(!rotates_in_the_deputy(&eleven_picked(
        90,
        75,
        0.0,
        FixtureLoad::Congested
    )));
}

/// Given the best manager, a first eleven at condition 70, and a deputy one point below them, well
///       inside his tolerance, but only at condition 75
/// When he picks his eleven on a normal week
/// Then he keeps his first eleven: a deputy must be at least ten fresher to be worth the change
#[test]
fn an_elite_manager_does_not_rest_a_starter_for_a_deputy_barely_fresher() {
    let mut squad: Vec<Player> = (0..11).map(|i| mk(&format!("star{i}"), 80, 70)).collect();
    squad.push(mk("deputy", 79, 75));
    let refs: Vec<&Player> = squad.iter().collect();

    let eleven: Vec<String> = ai_select_starting_xi(&refs, "4-4-2", 1.0, 1, FixtureLoad::Normal)
        .into_iter()
        .map(|player| player.id.clone())
        .collect();

    assert!(!rotates_in_the_deputy(&eleven));
}
