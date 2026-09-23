use super::*;
use domain::player::{Player, PlayerAttributes, Position as DomainPos};

/// Uniform attributes: `weighted_score` averages attributes, so setting them
/// all to `v` makes the condition-free positional fit ≈ `v` for any slot, with
/// the per-slot compatibility/foot penalty identical across players (same
/// position + footedness) — so it cancels in within-slot comparisons.
fn attrs(v: u8) -> PlayerAttributes {
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

fn mk(id: &str, attr: u8, condition: u8) -> Player {
    mk_pos(id, DomainPos::CenterBack, attr, condition)
}

fn mk_pos(id: &str, position: DomainPos, attr: u8, condition: u8) -> Player {
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

    let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0, 1);

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

    let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0, 1);

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

    let xi = ai_select_starting_xi(&refs, "4-4-2", management_quality(300), 1); // q = 0

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

    let xi = ai_select_starting_xi(&refs, "4-4-2", management_quality(300), 1); // q = 0

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

    let xi = ai_select_starting_xi(&refs, "4-4-2", management_quality(300), 1);

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
        let xi = ai_select_starting_xi(&refs, "4-4-2", quality, seed);
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

    let first = ai_select_starting_xi(&refs, "4-4-2", 0.0, 7);
    let second = ai_select_starting_xi(&refs, "4-4-2", 0.0, 7);

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

    let xi = ai_select_starting_xi(&refs, "4-4-2", 1.0, 1); // elite: rotates eagerly

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
