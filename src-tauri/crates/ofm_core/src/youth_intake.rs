//! A club's annual youth intake: the youngsters who join its academy at each
//! season's end.
//!
//! The intake is the world's one ordinary source of new players. Without it a
//! world only loses them — players retire, the academies the generator seeded
//! graduate within a few seasons, the free-agent pool drains, and from then on
//! clubs live on the squad floor's emergency top-up.
//!
//! One rule for every club, the player's included. It is split in two on
//! purpose: [`plan_for`] decides how many youngsters a club takes and where they
//! play, from its academy alone and with no randomness, so a later academy cost
//! can price a plan before anything is generated; [`apply_youth_intake`] then
//! brings the planned youngsters in.

use crate::game::Game;
use crate::squad_floor::{group_index, thinnest_first};
use chrono::{Datelike, NaiveDate};
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::player::{Player, Position, SquadRole};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::collections::HashMap;

/// Academy players a club aims to hold in each position group, in the squad
/// floor's `[GK, DEF, MID, FWD]` order. Shaped like a squad, a keeper included,
/// so the players who graduate from it can replace the ones who retire.
const ACADEMY_TARGET_PER_GROUP: [usize; 4] = [1, 2, 2, 1];

/// Youngsters every club takes in a season, however full its academy is. A
/// club always renews itself a little; the minimum is what keeps one whose
/// academy is full from going a whole season without a newcomer.
const MIN_INTAKE: usize = 1;

/// Youngsters a club takes in a season at most, however empty its academy is.
/// A club rebuilding an academy does it over a few seasons, not in one summer.
const MAX_INTAKE: usize = 3;

/// The ages a youngster joins an academy at. Younger than a youth scout's
/// recruit, who is found ready to play: an intake joins to be brought on.
const INTAKE_AGES: std::ops::RangeInclusive<u32> = 15..=17;

/// Which youngsters a club takes in this season: one position group per
/// recruit, a keeper first when the academy has none.
///
/// Pure and free: what the finance epic will attach an academy cost to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntakePlan {
    pub groups: Vec<Position>,
}

/// The intake a club with `academy` takes: as many youngsters as it is short of
/// [`ACADEMY_TARGET_PER_GROUP`], between [`MIN_INTAKE`] and [`MAX_INTAKE`]; a
/// keeper first when it has none, then its thinnest groups.
pub fn plan_for<'a>(academy: impl IntoIterator<Item = &'a Player>) -> IntakePlan {
    let mut have = [0usize; 4];
    for player in academy {
        have[group_index(&player.position)] += 1;
    }
    let lacking: usize = ACADEMY_TARGET_PER_GROUP
        .iter()
        .zip(have)
        .map(|(target, have)| target.saturating_sub(have))
        .sum();
    let size = lacking.clamp(MIN_INTAKE, MAX_INTAKE);

    let keeper = group_index(&Position::Goalkeeper);
    let mut groups = Vec::with_capacity(size);
    if have[keeper] == 0 {
        groups.push(Position::Goalkeeper);
        have[keeper] += 1;
    }
    while groups.len() < size {
        let thinnest = thinnest_first(have, ACADEMY_TARGET_PER_GROUP).remove(0);
        have[group_index(&thinnest)] += 1;
        groups.push(thinnest);
    }
    IntakePlan { groups }
}

/// Every club takes its season's intake into its academy on `date`, the end of
/// `season`. The player's club is told who joined.
pub fn apply_youth_intake(game: &mut Game, date: NaiveDate, season: u32) {
    let mut academies: HashMap<&str, Vec<&Player>> = HashMap::new();
    for player in &game.players {
        if player.squad_role == SquadRole::Youth
            && let Some(team_id) = player.team_id.as_deref()
        {
            academies.entry(team_id).or_default().push(player);
        }
    }
    let plans: Vec<(usize, IntakePlan)> = game
        .teams
        .iter()
        .enumerate()
        .map(|(index, team)| {
            let academy = academies.remove(team.id.as_str()).unwrap_or_default();
            (index, plan_for(academy))
        })
        .collect();

    let user_team_id = game.manager.team_id.clone();
    for (team_index, plan) in plans {
        let joined = take_in(game, team_index, &plan, date, season);
        if user_team_id.as_deref() == Some(game.teams[team_index].id.as_str()) {
            tell_the_player(game, team_index, &joined, date, season);
        }
    }
}

/// Bring `plan`'s youngsters into the academy of the club at `team_index`.
/// Returns their names, in plan order.
fn take_in(
    game: &mut Game,
    team_index: usize,
    plan: &IntakePlan,
    date: NaiveDate,
    season: u32,
) -> Vec<String> {
    // Seeded from the club and the season, so a replayed season end draws the
    // same youngsters whatever order the clubs come in (their ids are still
    // fresh). The seed knows nothing of the save: two careers from one package,
    // whose club ids are authored, draw the same intake. It moves onto the
    // game's own seed (`Game::rng_for`) once that exists.
    let seed = crate::stable_hash::stable_hash(
        game.teams[team_index].id.as_bytes(),
        u64::from(season) ^ INTAKE_STREAM,
    );
    let mut rng = StdRng::seed_from_u64(seed);
    let contract_start = date.format("%Y-%m-%d").to_string();
    let mut joined = Vec::with_capacity(plan.groups.len());
    for group in &plan.groups {
        let team = &game.teams[team_index];
        let age = rng.random_range(INTAKE_AGES);
        let mut recruit = crate::generator::generate_youth_intake_recruit(
            team,
            group,
            age,
            date.year() as u32,
            &mut rng,
        );
        recruit.contract_start = Some(contract_start.clone());
        recruit.jersey_number = crate::roster::resolve_jersey_for(game, &recruit, team);
        joined.push(recruit.full_name.clone());
        game.players.push(recruit);
    }
    joined
}

/// Folded into the intake's seed so its stream is its own, not whichever other
/// policy hashes a club id under the same season.
const INTAKE_STREAM: u64 = 0x5955_4f55_5448; // "YOUTH"

fn tell_the_player(
    game: &mut Game,
    team_index: usize,
    joined: &[String],
    date: NaiveDate,
    season: u32,
) {
    if joined.is_empty() {
        return;
    }
    let team = &game.teams[team_index];
    let mut params = HashMap::new();
    params.insert("team".to_string(), team.name.clone());
    params.insert("players".to_string(), joined.join(", "));
    let message = InboxMessage::new(
        format!("youth_intake_{}_{season}", team.id),
        String::new(),
        String::new(),
        String::new(),
        date.format("%Y-%m-%d").to_string(),
    )
    .with_category(MessageCategory::ScoutReport)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("")
    .with_i18n(
        "be.msg.youthIntake.subject",
        "be.msg.youthIntake.body",
        params,
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager");
    crate::inbox::emit(game, message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::squad_floor::MIN_PLAYERS_PER_GROUP;
    use crate::test_support::uniform_attributes;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::Team;

    const SEASON: u32 = 2026;

    fn season_end() -> NaiveDate {
        NaiveDate::from_ymd_opt(2027, 5, 30).unwrap()
    }

    fn youngster(id: &str, team_id: &str, position: Position) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "2009-03-01".to_string(),
            "ENG".to_string(),
            position,
            uniform_attributes(50),
        );
        player.team_id = Some(team_id.to_string());
        player.squad_role = SquadRole::Youth;
        player
    }

    /// An academy with `per_group` youngsters, `[GK, DEF, MID, FWD]`.
    fn academy(team_id: &str, per_group: [usize; 4]) -> Vec<Player> {
        MIN_PLAYERS_PER_GROUP
            .iter()
            .zip(per_group)
            .flat_map(|((group, _), count)| {
                (0..count).map(move |n| {
                    youngster(&format!("{team_id}-{group:?}-{n}"), team_id, group.clone())
                })
            })
            .collect()
    }

    fn team(id: &str) -> Team {
        Team::new(
            id.to_string(),
            format!("{id} FC"),
            "CLB".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20_000,
        )
    }

    /// The player's club `user` and an AI club `rival`, with these academies.
    fn world(user_academy: [usize; 4], rival_academy: [usize; 4]) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2027, 5, 30, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("user".to_string());
        let mut players = academy("user", user_academy);
        players.extend(academy("rival", rival_academy));
        Game::new(
            clock,
            manager,
            vec![team("user"), team("rival")],
            players,
            vec![],
            vec![],
        )
    }

    fn newcomers<'a>(before: &Game, after: &'a Game, team_id: &str) -> Vec<&'a Player> {
        after
            .players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some(team_id))
            .filter(|player| !before.players.iter().any(|old| old.id == player.id))
            .collect()
    }

    fn intake(game: &Game) -> Game {
        let mut after = game.clone();
        apply_youth_intake(&mut after, season_end(), SEASON);
        after
    }

    // -- the rule ------------------------------------------------------------

    #[test]
    fn every_club_takes_youngsters_in_each_season() {
        let before = world([1, 3, 3, 2], [0, 1, 1, 0]);
        let after = intake(&before);
        for club in ["user", "rival"] {
            assert!(
                !newcomers(&before, &after, club).is_empty(),
                "{club} took nobody into its academy"
            );
        }
    }

    #[test]
    fn the_intake_fills_what_the_academy_lacks() {
        let at_target = academy("club", ACADEMY_TARGET_PER_GROUP);
        assert_eq!(plan_for(&at_target).groups.len(), MIN_INTAKE);

        let empty: Vec<Player> = Vec::new();
        assert_eq!(plan_for(&empty).groups.len(), MAX_INTAKE);

        // A defender and a forward short, between the bounds: exactly those two.
        let mut shape = ACADEMY_TARGET_PER_GROUP;
        shape[group_index(&Position::Defender)] -= 1;
        shape[group_index(&Position::Forward)] -= 1;
        let two_short = academy("club", shape);
        assert_eq!(
            plan_for(&two_short).groups,
            vec![Position::Defender, Position::Forward]
        );
    }

    #[test]
    fn a_club_with_a_full_academy_still_takes_the_minimum() {
        let overfull = academy("club", [2, 5, 5, 3]);
        assert_eq!(plan_for(&overfull).groups.len(), MIN_INTAKE);
    }

    #[test]
    fn the_intake_includes_a_keeper_when_the_academy_has_none() {
        // Further short of defenders and midfielders than of a keeper, and at
        // the maximum intake: the keeper is still the first recruit.
        let no_keeper = academy("club", [0, 0, 0, 1]);
        let plan = plan_for(&no_keeper);
        assert_eq!(plan.groups.len(), MAX_INTAKE);
        assert_eq!(plan.groups[0], Position::Goalkeeper);

        let with_keeper = academy("club", [1, 1, 1, 1]);
        assert!(
            !plan_for(&with_keeper)
                .groups
                .contains(&Position::Goalkeeper)
        );
    }

    #[test]
    fn recruits_are_academy_players_of_the_club() {
        let mut before = world([0, 0, 0, 0], [0, 0, 0, 0]);
        // A senior squad already wearing the shirts a generated player's
        // squad slot would hand him.
        for number in 1..=40u8 {
            let mut senior = youngster(&format!("senior-{number}"), "rival", Position::Midfielder);
            senior.squad_role = SquadRole::Senior;
            senior.jersey_number = Some(number);
            before.players.push(senior);
        }
        let after = intake(&before);
        let recruits = newcomers(&before, &after, "rival");
        assert_eq!(recruits.len(), MAX_INTAKE);
        let mut shirts: std::collections::HashSet<u8> = (1..=40).collect();
        for recruit in recruits {
            assert_eq!(recruit.squad_role, SquadRole::Youth);
            let age = crate::generator::opening_player_age(&recruit.date_of_birth, 2027)
                .expect("a recruit has a date of birth");
            assert!(
                (14..=crate::roster::YOUTH_ACADEMY_MAX_AGE).contains(&age),
                "a recruit of academy age, not {age}"
            );
            assert_eq!(recruit.contract_start.as_deref(), Some("2027-05-30"));
            assert!(
                recruit
                    .contract_end
                    .as_deref()
                    .is_some_and(|end| end > "2027-05-30"),
                "a recruit holds a contract that runs past the day he joins"
            );
            assert!(recruit.wage > 0);
            assert!(!recruit.transfer_listed && !recruit.loan_listed);
            let shirt = recruit.jersey_number.expect("a recruit wears a shirt");
            assert!(
                shirts.insert(shirt),
                "shirt {shirt} is already worn at the club"
            );
        }
    }

    #[test]
    fn the_intake_is_one_plan_a_cost_can_attach_to() {
        let before = world([0, 2, 1, 2], [1, 3, 3, 2]);
        let after = intake(&before);
        // No money moves today: the finance epic prices the plan later.
        for (old, new) in before.teams.iter().zip(&after.teams) {
            assert_eq!(old.finance, new.finance, "{} paid for its intake", old.id);
        }
        // What was taken in is exactly the plan.
        for club in ["user", "rival"] {
            let academy: Vec<&Player> = before
                .players
                .iter()
                .filter(|player| player.team_id.as_deref() == Some(club))
                .collect();
            let mut planned = plan_for(academy).groups;
            let mut taken: Vec<Position> = newcomers(&before, &after, club)
                .into_iter()
                .map(|recruit| recruit.position.to_group_position())
                .collect();
            planned.sort_by_key(group_index);
            taken.sort_by_key(group_index);
            assert_eq!(
                taken, planned,
                "{club} took in something other than its plan"
            );
        }
    }

    #[test]
    fn a_replayed_season_end_takes_in_the_same_youngsters() {
        let before = world([0, 0, 0, 0], [0, 1, 1, 1]);
        let names = |game: &Game| {
            newcomers(&before, game, "rival")
                .into_iter()
                .map(|recruit| (recruit.full_name.clone(), recruit.date_of_birth.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&intake(&before)), names(&intake(&before)));
    }

    /// Given an AI club with no seniors to spare and nothing on the market, when
    /// the season end's squad turnover runs, then the youngsters it takes in are
    /// still in its academy afterwards: a fifteen-year-old joins to be brought
    /// on, not to be promoted by the rebuild on the day he arrives.
    #[test]
    fn a_recruit_is_not_promoted_on_the_day_he_joins() {
        let before = world([0, 0, 0, 0], [0, 0, 0, 0]);
        let mut after = before.clone();
        crate::end_of_season::apply_season_end_squad_turnover(&mut after, season_end(), SEASON);

        let recruits = newcomers(&before, &after, "rival");
        assert_eq!(recruits.len(), MAX_INTAKE);
        for recruit in recruits {
            assert_eq!(
                recruit.squad_role,
                SquadRole::Youth,
                "{} was promoted the day he joined",
                recruit.full_name
            );
        }
    }

    // -- the player's club -----------------------------------------------------

    #[test]
    fn the_player_is_told_who_joined_the_academy() {
        let before = world([0, 0, 0, 0], [0, 0, 0, 0]);
        let after = intake(&before);
        let joined: Vec<String> = newcomers(&before, &after, "user")
            .into_iter()
            .map(|recruit| recruit.full_name.clone())
            .collect();
        let told: Vec<&InboxMessage> = after
            .messages
            .iter()
            .filter(|message| message.subject_key.as_deref() == Some("be.msg.youthIntake.subject"))
            .collect();
        assert_eq!(told.len(), 1, "one message, for the player's club only");
        assert_eq!(told[0].body_key.as_deref(), Some("be.msg.youthIntake.body"));
        let named = told[0]
            .i18n_params
            .get("players")
            .expect("the message names them");
        for name in &joined {
            assert!(
                named.contains(name.as_str()),
                "{name} joined but was not named"
            );
        }
    }

    #[test]
    fn the_players_club_takes_the_same_intake_as_an_ai_club() {
        let before = world([0, 2, 1, 2], [0, 2, 1, 2]);
        let after = intake(&before);
        let groups = |club: &str| {
            let mut groups: Vec<Position> = newcomers(&before, &after, club)
                .into_iter()
                .map(|recruit| recruit.position.to_group_position())
                .collect();
            groups.sort_by_key(group_index);
            groups
        };
        assert!(!groups("user").is_empty());
        assert_eq!(groups("user"), groups("rival"));
    }
}
