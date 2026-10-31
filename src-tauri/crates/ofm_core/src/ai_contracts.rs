//! AI clubs renew their own players' contracts.
//!
//! Every contract in the world runs out, and until this module the only club
//! that could renew one was the player's: renewal negotiation, delegated
//! renewals and free-agent signings all start from `game.manager.team_id`. An
//! AI club that did not win players in the transfer market therefore lost its
//! squad a contract at a time and never replaced it.
//!
//! An AI club looks at the contracts running down on its weekly review day, the
//! same day it reviews its tactics, and renews a player when it needs him or
//! wants him and can pay him. The terms are the ones the contracts module says
//! the player expects, judged by the same `evaluate_renewal_offer` and applied
//! by the same `apply_agreed_renewal` as the player's own negotiations, and
//! paid under the same `wage_policy_verdict` — there is no AI-only copy of the
//! wage, contract-length or wage-policy rules.

use std::collections::HashMap;

use chrono::NaiveDate;
use domain::player::Player;
use domain::team::Team;

use crate::contract_wage_policy::wage_policy_verdict;
use crate::contracts::{
    RenewalDecision, RenewalOffer, apply_agreed_renewal, evaluate_renewal_offer,
    expected_contract_years, expected_wage, next_renewal_round, remaining_contract_days,
};
use crate::game::Game;
use crate::squad_floor::{
    PLANNING_MARGIN_PER_GROUP, PLANNING_TARGET_SENIORS, group_floor, other_seniors,
    others_in_his_group,
};

/// A contract with this many days or fewer left is one the club decides on.
/// Half a season: early enough to act on every review day until it is settled.
const RENEWAL_HORIZON_DAYS: i64 = 183;

/// A player at least as good as the squad's median, and no older than this, is
/// one the club wants to keep.
const PEAK_AGE_LIMIT: i32 = 32;

/// A player this young is kept as a prospect, whatever his rating today.
const PROSPECT_AGE_LIMIT: i32 = 23;

/// Run every AI club's contract review that falls on this weekday.
pub(crate) fn apply_ai_contract_decisions(game: &mut Game, weekday_num: u32) {
    let current_date = game.clock.current_date.date_naive();
    // Found once for every club reviewing today: a renewal changes a player's terms, never the
    // club that holds his contract, so no club's group goes stale during the reviews.
    let contracts = contracts_by_club(&game.players);
    for team_id in crate::ai_tactics::ai_clubs_reviewing_on(game, weekday_num) {
        let Some(team) = game.teams.iter().find(|team| team.id == team_id).cloned() else {
            continue;
        };
        let held = contracts.get(&team_id).map_or(&[][..], Vec::as_slice);
        debug_assert_contracts_are_current(&game.players, &team_id, held);
        for player_index in contracts_running_down(&game.players, held, current_date) {
            if keeps(game, &team, &game.players[player_index], current_date) {
                renew(game, &team, player_index, current_date);
            }
        }
    }
}

/// Run every AI club's squad planning that falls on this weekday.
///
/// Ordinary management, so the squad floor's emergency top-up has nothing to
/// do: a club graduates academy players who have outgrown the academy, then
/// keeps one senior above the minimum in each position group and
/// [`PLANNING_TARGET_SENIORS`] in all — promoting from its academy first, then
/// signing free agents the board lets it pay
/// ([`crate::squad_floor::bring_in_one`], the same source the emergency uses).
/// Runs after the day's departures so it sees the squad the club actually has.
pub(crate) fn apply_ai_squad_planning(game: &mut Game, weekday_num: u32) {
    let clubs = crate::ai_tactics::ai_clubs_reviewing_on(game, weekday_num);
    plan_squads(game, &clubs);
}

/// Every AI club plans its squad at once, whatever its review day: what clubs
/// do at a season's end, when retirements can take several players from one
/// club overnight and a week is too long to wait.
pub(crate) fn plan_every_ai_squad(game: &mut Game) {
    let clubs = crate::ai_tactics::ai_clubs(game);
    plan_squads(game, &clubs);
}

fn plan_squads(game: &mut Game, clubs: &[String]) {
    let current_date = game.clock.current_date.date_naive();
    // Where each club's players are, found once. A signing changes one club's list and is
    // recounted where it happens; nothing else here moves a player between clubs.
    let mut positions = crate::match_day_eligibility::positions_by_club(&game.players);
    for team_id in clubs {
        let Some(team) = game.teams.iter().find(|team| &team.id == team_id).cloned() else {
            continue;
        };
        let club = positions.entry(team_id.clone()).or_default();
        crate::match_day_eligibility::debug_assert_group_is_current(&game.players, team_id, club);
        graduate_overage_academy_players(game, club, current_date);
        plan_squad_depth(game, &team, club);
    }
}

/// An academy player past the academy's age limit joins the senior squad. `club` is where the
/// club's players stand in `game.players`.
fn graduate_overage_academy_players(game: &mut Game, club: &[usize], current_date: NaiveDate) {
    for &position in club {
        let player = &mut game.players[position];
        if player.squad_role == domain::player::SquadRole::Youth
            && domain::age::age_on_or_unknown(current_date, &player.date_of_birth)
                > crate::roster::YOUTH_ACADEMY_MAX_AGE
        {
            player.squad_role = domain::player::SquadRole::Senior;
        }
    }
}

/// Fill each group to one above its minimum, then the squad to the planning
/// target, thinnest group first, for as long as a source has someone.
///
/// A senior whose contract runs out within the renewal horizon is not counted:
/// the club has had its chance to renew him, and if he is still on the old
/// contract he may walk. Generated contracts share an end date, so several can
/// leave a group on the same day — planning replaces them before they go, not
/// after.
fn plan_squad_depth(game: &mut Game, team: &Team, club: &mut Vec<usize>) {
    use crate::squad_floor::{MIN_PLAYERS_PER_GROUP, bring_in_one, groups_thinnest_first};

    for (group, floor) in MIN_PLAYERS_PER_GROUP {
        while seniors_in(game, club, Some(&group)) < floor + PLANNING_MARGIN_PER_GROUP {
            if bring_in_one(game, team, &group).is_none() {
                break;
            }
            *club = crate::match_day_eligibility::club_positions(&game.players, &team.id);
        }
    }
    while seniors_in(game, club, None) < PLANNING_TARGET_SENIORS {
        let groups = groups_thinnest_first(game, &team.id);
        let brought_one = groups
            .iter()
            .any(|group| bring_in_one(game, team, group).is_some());
        if !brought_one {
            break;
        }
        *club = crate::match_day_eligibility::club_positions(&game.players, &team.id);
    }
}

/// Seniors among the club's players (`club`: where they stand in `game.players`) who will still
/// be under contract past the renewal horizon, in one group or in all.
fn seniors_in(game: &Game, club: &[usize], group: Option<&domain::player::Position>) -> usize {
    let current_date = game.clock.current_date.date_naive();
    club.iter()
        .map(|&position| &game.players[position])
        .filter(|player| player.squad_role == domain::player::SquadRole::Senior)
        // No end date is a contract the expiry sweep never ends: he stays.
        .filter(|player| {
            crate::contracts::contract_days_remaining(player.contract_end(), current_date)
                .is_none_or(|days| days > RENEWAL_HORIZON_DAYS)
        })
        .filter(|player| group.is_none_or(|group| player.position.to_group_position() == *group))
        .count()
}

/// Where each club's contracted players (its loaned-out ones included) stand in `players`, found
/// in one pass, each club's in the order of `players`. Looked up by club, never iterated.
fn contracts_by_club(players: &[Player]) -> HashMap<String, Vec<usize>> {
    let mut by_club: HashMap<String, Vec<usize>> = HashMap::new();
    for (position, player) in players.iter().enumerate() {
        let Some(club) = player.contract_club_id() else {
            continue;
        };
        match by_club.get_mut(club) {
            Some(held) => held.push(position),
            None => {
                by_club.insert(club.to_string(), vec![position]);
            }
        }
    }
    by_club
}

/// In a debug build, panics unless `held` is where the players whose contracts `team_id` holds
/// stand in `players` now. A release build checks nothing.
fn debug_assert_contracts_are_current(players: &[Player], team_id: &str, held: &[usize]) {
    debug_assert_eq!(
        held,
        players
            .iter()
            .enumerate()
            .filter(|(_, player)| player.contract_club_id() == Some(team_id))
            .map(|(position, _)| position)
            .collect::<Vec<_>>()
            .as_slice(),
        "the contracts kept for {team_id} are stale: a contract changed club and was not found again"
    );
}

fn contracts_running_down(
    players: &[Player],
    held: &[usize],
    current_date: NaiveDate,
) -> Vec<usize> {
    held.iter()
        .copied()
        .filter(|&position| players[position].contract_end().is_some())
        .filter(|&position| {
            remaining_contract_days(&players[position], current_date) <= RENEWAL_HORIZON_DAYS
        })
        .collect()
}

/// Does the club want this player for another contract?
fn keeps(game: &Game, team: &Team, player: &Player, current_date: NaiveDate) -> bool {
    needed_for_depth(game, &team.id, player) || worth_keeping(game, team, player, current_date)
}

/// Letting him go would leave the club under what its squad planning aims for
/// — in his group, or in all — counted as the floor counts. Whether the club
/// may go over its wage policy to keep him is not decided here: `renew` asks
/// the one rule every club answers to.
fn needed_for_depth(game: &Game, team_id: &str, player: &Player) -> bool {
    others_in_his_group(game, team_id, player)
        < group_floor(&player.position.to_group_position()) + PLANNING_MARGIN_PER_GROUP
        || other_seniors(game, team_id, player) < PLANNING_TARGET_SENIORS
}

/// Good enough and young enough to want.
fn worth_keeping(game: &Game, team: &Team, player: &Player, current_date: NaiveDate) -> bool {
    let age = domain::age::age_on_or_unknown(current_date, &player.date_of_birth);
    age <= PROSPECT_AGE_LIMIT
        || (age <= PEAK_AGE_LIMIT && player.ovr >= squad_median_ovr(game, &team.id))
}

/// The median rating of the club's seniors, counted as the floor counts them:
/// an academy of low-rated youngsters is not the standard a senior is held to.
fn squad_median_ovr(game: &Game, team_id: &str) -> u8 {
    let mut ratings: Vec<u8> = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .filter(|player| player.squad_role == domain::player::SquadRole::Senior)
        .map(|player| player.ovr)
        .collect();
    if ratings.is_empty() {
        return 0;
    }
    ratings.sort_unstable();
    ratings[ratings.len() / 2]
}

/// Offer what the player expects, if the board allows the wage, and if he
/// takes it put it into effect.
fn renew(game: &mut Game, team: &Team, player_index: usize, current_date: NaiveDate) {
    let player = &game.players[player_index];
    let offer = RenewalOffer {
        weekly_wage: expected_wage(player, team, current_date),
        contract_years: expected_contract_years(player, current_date),
    };
    if !wage_policy_verdict(game, team, player, offer.weekly_wage).permits() {
        return;
    }
    let outcome = evaluate_renewal_offer(player, team, current_date, &offer);
    if outcome.decision != RenewalDecision::Accepted {
        return;
    }
    let round = next_renewal_round(player, None);
    if let Err(error) = apply_agreed_renewal(
        &mut game.players[player_index],
        team,
        offer.weekly_wage,
        offer.contract_years,
        current_date,
        crate::contracts::RenewalAgreedBy::Manager { round },
    ) {
        log::error!(
            "[ai_contracts] {} could not renew {}: {error}",
            team.id,
            game.players[player_index].id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{Datelike, Duration, TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{PlayerAttributes, Position};

    const TODAY: (i32, u32, u32) = (2026, 8, 3);

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(TODAY.0, TODAY.1, TODAY.2).unwrap()
    }

    fn attrs(level: u8) -> PlayerAttributes {
        PlayerAttributes {
            pace: level,
            stamina: level,
            strength: level,
            agility: level,
            passing: level,
            shooting: level,
            tackling: level,
            dribbling: level,
            defending: level,
            positioning: level,
            vision: level,
            decisions: level,
            composure: level,
            aggression: level,
            teamwork: level,
            leadership: level,
            handling: level,
            reflexes: level,
            aerial: level,
        }
    }

    fn player(
        id: &str,
        team_id: &str,
        position: Position,
        ovr: u8,
        age: i32,
        days_left: i64,
    ) -> Player {
        let born = format!("{}-01-01", TODAY.0 - age);
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            born,
            "England".to_string(),
            position,
            attrs(ovr),
        );
        player.team_id = Some(team_id.to_string());
        player.ovr = ovr;
        player.stage_wage(1_000);
        player.stage_contract_end(Some(
            (today() + Duration::days(days_left))
                .format("%Y-%m-%d")
                .to_string(),
        ));
        player
    }

    fn team(id: &str) -> Team {
        let mut team = Team::new(
            id.to_string(),
            id.to_string(),
            id.to_uppercase(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20_000,
        );
        team.wage_budget = 10_000_000;
        team
    }

    /// Two clubs of nineteen — one over the planning target, and a senior over
    /// the planning margin in every group, so nobody is needed for depth unless
    /// a test says so — every contract long, rated 60, aged 26.
    fn world() -> Game {
        let clock = GameClock::new(
            Utc.with_ymd_and_hms(TODAY.0, TODAY.1, TODAY.2, 12, 0, 0)
                .unwrap(),
        );
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("user".to_string());
        let players = ["user", "ai"].into_iter().flat_map(squad).collect();
        Game::new(
            clock,
            manager,
            vec![team("user"), team("ai")],
            players,
            vec![],
            vec![],
        )
    }

    /// Nineteen seniors of `club`, rated 60, aged 26, with long contracts.
    fn squad(club: &str) -> Vec<Player> {
        let mut players = Vec::new();
        for (group, count) in [
            (Position::Goalkeeper, 3),
            (Position::Defender, 6),
            (Position::Midfielder, 6),
            (Position::Forward, 4),
        ] {
            for i in 0..count {
                players.push(player(
                    &format!("{club}_{group:?}{i}"),
                    club,
                    group.clone(),
                    60,
                    26,
                    1_000,
                ));
            }
        }
        players
    }

    fn review_day() -> u32 {
        crate::ai_tactics::review_weekday("ai")
    }

    /// A second AI club, with the same squad as "ai", that reviews on the same day.
    fn add_club_reviewing_with_ai(game: &mut Game) -> String {
        let id = (0..)
            .map(|n| format!("ai{n}"))
            .find(|id| crate::ai_tactics::review_weekday(id) == review_day())
            .unwrap();
        game.teams.push(team(&id));
        game.players.extend(squad(&id));
        id
    }

    fn loan_out(player: &mut Player, parent: &str, borrower: &str) {
        player.team_id = Some(borrower.to_string());
        player.active_loan = Some(domain::player::ActiveLoan {
            parent_team_id: parent.to_string(),
            loan_team_id: borrower.to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: "2027-06-30".to_string(),
            wage_contribution_pct: 100,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
    }

    fn contract_end(game: &Game, id: &str) -> String {
        game.players
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .contract_end()
            .unwrap()
            .to_string()
    }

    #[test]
    fn a_good_player_running_down_his_contract_is_renewed_on_the_review_day() {
        let mut game = world();
        game.players
            .push(player("star", "ai", Position::Midfielder, 75, 27, 90));
        let before = contract_end(&game, "star");

        apply_ai_contract_decisions(&mut game, review_day());

        assert!(
            contract_end(&game, "star") > before,
            "the club let a player it wants run down"
        );
        // An AI renewal goes through the one renewal writer, so it is in his history.
        let star = game.players.iter().find(|p| p.id == "star").unwrap();
        let entry = star
            .movement_history
            .last()
            .expect("the renewal is recorded");
        assert_eq!(entry.kind, domain::player::PlayerMovementKind::Renewal);
        assert_eq!(entry.to_team_id.as_deref(), Some("ai"));
        assert_eq!(
            entry.contract.as_ref().map(|c| c.weekly_wage),
            Some(star.wage())
        );
    }

    #[test]
    fn nothing_is_decided_on_another_day() {
        let mut game = world();
        game.players
            .push(player("star", "ai", Position::Midfielder, 75, 27, 90));
        let before = contract_end(&game, "star");

        apply_ai_contract_decisions(&mut game, (review_day() + 1) % 7);

        assert_eq!(contract_end(&game, "star"), before);
    }

    #[test]
    fn the_players_own_club_is_left_to_the_player() {
        let mut game = world();
        game.players.push(player(
            "user_star",
            "user",
            Position::Midfielder,
            75,
            27,
            90,
        ));
        let before = contract_end(&game, "user_star");

        for weekday in 0..7 {
            apply_ai_contract_decisions(&mut game, weekday);
        }

        assert_eq!(contract_end(&game, "user_star"), before);
    }

    /// Past his best, below the squad's standard, and not needed for depth:
    /// the club lets the contract run out.
    #[test]
    fn a_fading_player_the_club_can_spare_is_let_go() {
        let mut game = world();
        game.players
            .push(player("veteran", "ai", Position::Midfielder, 50, 34, 90));
        let before = contract_end(&game, "veteran");

        apply_ai_contract_decisions(&mut game, review_day());

        assert_eq!(contract_end(&game, "veteran"), before);
    }

    /// An AI club of nineteen seniors rated 60, an academy of thirty youngsters
    /// rated 30, and a 29-year-old midfielder rated `rating` whose contract ends
    /// in 90 days. The academy is big enough to pull a median over everyone
    /// down to 55, while the seniors' median stays at 60.
    fn club_with_a_weak_academy_and_a_journeyman(rating: u8) -> Game {
        let mut game = world();
        for i in 0..30 {
            let mut youngster = player(
                &format!("academy{i}"),
                "ai",
                Position::Midfielder,
                30,
                17,
                1_000,
            );
            youngster.squad_role = domain::player::SquadRole::Youth;
            game.players.push(youngster);
        }
        game.players.push(player(
            "journeyman",
            "ai",
            Position::Midfielder,
            rating,
            29,
            90,
        ));
        game
    }

    /// Given an AI club whose academy is full of low-rated youngsters, when a
    /// senior below the seniors' standard runs down his contract, he is judged
    /// against the seniors and let go. Counting the academy would have dragged
    /// the standard down to him and kept him.
    #[test]
    fn the_squad_median_is_the_seniors_median() {
        let mut game = club_with_a_weak_academy_and_a_journeyman(55);
        // The fixture splits the two medians around him — worked out here from
        // the fixture, not by the function under test.
        let median = |seniors_only: bool| {
            let mut ratings: Vec<u8> = game
                .players
                .iter()
                .filter(|p| p.team_id.as_deref() == Some("ai"))
                .filter(|p| !seniors_only || p.squad_role == domain::player::SquadRole::Senior)
                .map(|p| p.ovr)
                .collect();
            ratings.sort_unstable();
            ratings[ratings.len() / 2]
        };
        assert!(55 >= median(false), "a median over everyone would keep him");
        assert!(55 < median(true), "the seniors' median lets him go");
        let before = contract_end(&game, "journeyman");

        apply_ai_contract_decisions(&mut game, review_day());

        assert_eq!(contract_end(&game, "journeyman"), before);
    }

    /// The same club, the same player at the seniors' standard: he is renewed,
    /// so nothing but the standard he is measured against lets him go above.
    #[test]
    fn a_player_at_the_seniors_standard_is_renewed_at_a_club_with_a_weak_academy() {
        let mut game = club_with_a_weak_academy_and_a_journeyman(60);
        let before = contract_end(&game, "journeyman");

        apply_ai_contract_decisions(&mut game, review_day());

        assert!(contract_end(&game, "journeyman") > before);
    }

    /// The same player, but the club's only other keepers are the floor itself:
    /// letting him go would leave it one injury from a crisis.
    #[test]
    fn a_player_the_club_cannot_do_without_is_renewed_whoever_he_is() {
        let mut game = world();
        game.players.retain(|p| p.id != "ai_Goalkeeper2");
        game.players
            .push(player("old_keeper", "ai", Position::Goalkeeper, 50, 34, 90));
        let before = contract_end(&game, "old_keeper");

        apply_ai_contract_decisions(&mut game, review_day());

        assert!(contract_end(&game, "old_keeper") > before);
    }

    /// An expensive forward running down his contract, at a club whose wage
    /// budget is exactly its current bill — so renewing him on the terms he
    /// expects is over the board's policy — keeping `forwards_besides_him`
    /// other forwards.
    fn over_policy_forward(forwards_besides_him: usize) -> Game {
        let mut game = world();
        game.players.retain(|p| {
            !(p.team_id.as_deref() == Some("ai")
                && p.position == Position::Forward
                && p.id.as_str() >= format!("ai_Forward{forwards_besides_him}").as_str())
        });
        // Everyone else on no wage, so his raise alone decides the verdict.
        for other in game.players.iter_mut() {
            if other.team_id.as_deref() == Some("ai") {
                other.stage_wage(0);
            }
        }
        let mut star = player("costly_fwd", "ai", Position::Forward, 80, 27, 90);
        star.market_value = 40_000_000;
        game.players.push(star);
        let bill = crate::finances::calc_wages(&game, "ai");
        game.teams[1].wage_budget = bill;
        game
    }

    fn verdict_for(game: &Game, id: &str) -> crate::contract_wage_policy::WagePolicyVerdict {
        let team = &game.teams[1];
        let player = game.players.iter().find(|p| p.id == id).unwrap();
        wage_policy_verdict(game, team, player, expected_wage(player, team, today()))
    }

    /// One other forward: without him the club is below the floor, so the board
    /// lets the wage policy go — the same rule the player's own renewals use.
    #[test]
    fn a_club_goes_over_its_wage_policy_to_keep_a_player_it_needs_for_the_floor() {
        use crate::contract_wage_policy::WagePolicyVerdict;
        let mut game = over_policy_forward(1);
        assert_eq!(
            verdict_for(&game, "costly_fwd"),
            WagePolicyVerdict::OverPolicyToKeepSquadFloor
        );
        let before = contract_end(&game, "costly_fwd");

        apply_ai_contract_decisions(&mut game, review_day());

        assert!(contract_end(&game, "costly_fwd") > before);
    }

    /// Two other forwards: the club wants him for depth, but letting him go
    /// leaves it at the floor, not below it. The wage policy holds.
    #[test]
    fn a_club_at_the_floor_without_him_keeps_to_its_wage_policy() {
        use crate::contract_wage_policy::WagePolicyVerdict;
        let mut game = over_policy_forward(2);
        assert!(needed_for_depth(&game, "ai", game.players.last().unwrap()));
        assert_eq!(
            verdict_for(&game, "costly_fwd"),
            WagePolicyVerdict::OverPolicy
        );
        let before = contract_end(&game, "costly_fwd");

        apply_ai_contract_decisions(&mut game, review_day());

        assert_eq!(contract_end(&game, "costly_fwd"), before);
    }

    /// Given an AI club with three keepers, one of them out on loan, when its
    /// fading fourth keeper's contract runs down, then he is kept: the keeper
    /// on loan plays for his borrower this week, so the club counts him as the
    /// squad floor does — not at all.
    #[test]
    fn a_keeper_out_on_loan_does_not_count_toward_the_depth_a_club_needs() {
        let mut game = world();
        let loaned = game
            .players
            .iter_mut()
            .find(|p| p.id == "ai_Goalkeeper2")
            .unwrap();
        loan_out(loaned, "ai", "user");
        game.players
            .push(player("old_keeper", "ai", Position::Goalkeeper, 50, 34, 90));
        let before = contract_end(&game, "old_keeper");

        apply_ai_contract_decisions(&mut game, review_day());

        assert!(contract_end(&game, "old_keeper") > before);
    }

    /// Given an AI club whose best player's contract ends today, and today is
    /// the club's review day, when the day is played, then he is renewed and
    /// not released: the club decides before the expiry sweep runs, or the
    /// sweep would have made him a free agent first.
    #[test]
    fn a_day_renews_a_key_player_whose_contract_ends_that_day_before_releasing_him() {
        let mut game = world();
        let mut day = game.clock.current_date;
        while day.weekday().num_days_from_monday() != review_day() {
            day += Duration::days(1);
        }
        game.clock.current_date = day;
        let mut star = player("star", "ai", Position::Midfielder, 80, 27, 0);
        star.stage_contract_end(Some(day.format("%Y-%m-%d").to_string()));
        game.players.push(star);

        crate::turn::process_day(&mut game);

        let star = game.players.iter().find(|p| p.id == "star").unwrap();
        assert_eq!(star.team_id.as_deref(), Some("ai"), "he was released");
        assert!(
            star.contract_end() > Some(day.format("%Y-%m-%d").to_string().as_str()),
            "his contract was not renewed"
        );
    }

    /// Given two AI clubs that review on the same day, each with a wanted player running down his
    /// contract, when the day's reviews run, then both are renewed: the first club's renewal
    /// leaves the second club's contracts where the review found them.
    #[test]
    fn two_clubs_reviewing_on_one_day_each_renew_their_own_player() {
        let mut game = world();
        let other = add_club_reviewing_with_ai(&mut game);
        game.players
            .push(player("ai_star", "ai", Position::Midfielder, 75, 27, 90));
        game.players.push(player(
            "other_star",
            &other,
            Position::Midfielder,
            75,
            27,
            90,
        ));
        let ai_before = contract_end(&game, "ai_star");
        let other_before = contract_end(&game, "other_star");

        apply_ai_contract_decisions(&mut game, review_day());

        assert!(contract_end(&game, "ai_star") > ai_before);
        assert!(contract_end(&game, "other_star") > other_before);
    }

    /// Given a wanted player on loan from one AI club to another, both reviewing on the same day,
    /// when the reviews run, then the club that holds his contract renews him; the borrower does
    /// not decide on a contract it does not hold.
    #[test]
    fn a_player_out_on_loan_is_renewed_by_the_club_that_holds_his_contract() {
        let mut game = world();
        let borrower = add_club_reviewing_with_ai(&mut game);
        let mut star = player("star", "ai", Position::Midfielder, 75, 27, 90);
        loan_out(&mut star, "ai", &borrower);
        game.players.push(star);
        let before = contract_end(&game, "star");

        apply_ai_contract_decisions(&mut game, review_day());

        let star = game.players.iter().find(|p| p.id == "star").unwrap();
        assert!(star.contract_end().unwrap() > before.as_str());
        let renewal = star.movement_history.last().unwrap();
        assert_eq!(renewal.kind, domain::player::PlayerMovementKind::Renewal);
        assert_eq!(renewal.to_team_id.as_deref(), Some("ai"));
    }

    /// Given each club's contracts found once, when a player's contract moves to another club and
    /// the old group is used again, then a debug build refuses it.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "the contracts kept for ai are stale")]
    fn a_stale_contract_group_is_refused() {
        let mut game = world();
        let groups = contracts_by_club(&game.players);
        let held = groups.get("ai").unwrap();
        game.players[held[0]].team_id = Some("user".to_string());

        debug_assert_contracts_are_current(&game.players, "ai", held);
    }

    // --- Ordinary squad planning ------------------------------------------------

    fn academy(id: &str, team_id: &str, position: Position, age: i32) -> Player {
        let mut youngster = player(id, team_id, position, 55, age, 1_000);
        youngster.squad_role = domain::player::SquadRole::Youth;
        youngster
    }

    fn free_agent(id: &str, position: Position, ovr: u8) -> Player {
        let mut agent = player(id, "ai", position, ovr, 27, 1_000);
        agent.team_id = None;
        agent.stage_contract_end(None);
        agent.stage_wage(0);
        agent
    }

    fn role(game: &Game, id: &str) -> domain::player::SquadRole {
        game.players.iter().find(|p| p.id == id).unwrap().squad_role
    }

    fn club_of<'a>(game: &'a Game, id: &str) -> Option<&'a str> {
        game.players
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.team_id.as_deref())
    }

    /// Given an AI club with a 22-year-old academy player, on its review day he
    /// graduates to the senior squad — the same age limit that stops a manager
    /// putting a player over 21 into the academy.
    #[test]
    fn an_ai_club_graduates_academy_players_past_the_academy_age() {
        let mut game = world();
        game.players
            .push(academy("graduate", "ai", Position::Midfielder, 22));
        game.players
            .push(academy("prospect", "ai", Position::Midfielder, 19));

        apply_ai_squad_planning(&mut game, review_day());

        assert_eq!(role(&game, "graduate"), domain::player::SquadRole::Senior);
        assert_eq!(role(&game, "prospect"), domain::player::SquadRole::Youth);
    }

    /// Given an AI club with exactly two senior keepers and an academy keeper,
    /// on its review day the academy keeper is promoted: one above the floor.
    #[test]
    fn an_ai_club_at_the_floor_promotes_youth_to_keep_a_margin() {
        let mut game = world();
        game.players.retain(|p| p.id != "ai_Goalkeeper2");
        game.players
            .push(academy("young_keeper", "ai", Position::Goalkeeper, 18));
        game.players
            .push(free_agent("free_keeper", Position::Goalkeeper, 80));

        apply_ai_squad_planning(&mut game, review_day());

        assert_eq!(
            role(&game, "young_keeper"),
            domain::player::SquadRole::Senior
        );
        assert_eq!(club_of(&game, "free_keeper"), None, "youth comes first");
    }

    /// Given an AI club with exactly two senior keepers, no academy keeper, and
    /// a free-agent keeper within its wage policy, he is signed.
    #[test]
    fn an_ai_club_at_the_floor_signs_a_free_agent_it_can_afford() {
        let mut game = world();
        game.players.retain(|p| p.id != "ai_Goalkeeper2");
        game.players
            .push(free_agent("free_keeper", Position::Goalkeeper, 70));

        apply_ai_squad_planning(&mut game, review_day());

        assert_eq!(club_of(&game, "free_keeper"), Some("ai"));
        let keeper = game.players.iter().find(|p| p.id == "free_keeper").unwrap();
        let entry = keeper
            .movement_history
            .last()
            .expect("the signing is recorded");
        assert_eq!(
            entry.kind,
            domain::player::PlayerMovementKind::FreeAgentSigning
        );
        assert_eq!(entry.to_team_id.as_deref(), Some("ai"));
    }

    fn signed_by_ai(game: &Game, ids: &[&str]) -> usize {
        ids.iter()
            .filter(|id| club_of(game, id) == Some("ai"))
            .count()
    }

    /// Given an AI club one keeper below its margin and three affordable free-agent keepers, when
    /// it plans its squad, then it signs exactly one: the signing is counted before it asks again.
    #[test]
    fn a_club_one_keeper_below_its_margin_signs_exactly_one_of_three_free_keepers() {
        let mut game = world();
        game.players.retain(|p| p.id != "ai_Goalkeeper2");
        let keepers = ["free_keeper_a", "free_keeper_b", "free_keeper_c"];
        for id in keepers {
            game.players.push(free_agent(id, Position::Goalkeeper, 70));
        }

        apply_ai_squad_planning(&mut game, review_day());

        assert_eq!(signed_by_ai(&game, &keepers), 1);
    }

    /// Given an AI club whose groups all meet their margin but whose squad is two short of the
    /// planning target, and five affordable free-agent midfielders, when it plans its squad, then
    /// it signs exactly two: each signing is counted toward the squad before it asks again.
    #[test]
    fn a_club_two_below_its_squad_target_signs_exactly_two_of_five_free_players() {
        let mut game = world();
        game.players.retain(|p| {
            !["ai_Defender5", "ai_Midfielder5", "ai_Forward3"].contains(&p.id.as_str())
        });
        let agents = ["free_a", "free_b", "free_c", "free_d", "free_e"];
        for id in agents {
            game.players.push(free_agent(id, Position::Midfielder, 70));
        }

        apply_ai_squad_planning(&mut game, review_day());

        assert_eq!(signed_by_ai(&game, &agents), 2);
    }

    /// The same, but paying him would break the wage policy: he is not signed,
    /// because the club is at its keeper minimum, not below it.
    #[test]
    fn an_ai_club_does_not_break_its_wage_policy_for_a_margin() {
        let mut game = world();
        game.players.retain(|p| p.id != "ai_Goalkeeper2");
        for other in game.players.iter_mut() {
            if other.team_id.as_deref() == Some("ai") {
                other.stage_wage(0);
            }
        }
        let mut costly = free_agent("costly_keeper", Position::Goalkeeper, 85);
        costly.market_value = 40_000_000;
        game.players.push(costly);
        game.teams[1].wage_budget = crate::finances::calc_wages(&game, "ai");

        apply_ai_squad_planning(&mut game, review_day());

        assert_eq!(club_of(&game, "costly_keeper"), None);
    }

    /// Given the player's club at its keeper minimum with an academy keeper,
    /// its review day brings nobody in: the manager decides.
    #[test]
    fn the_players_club_is_never_planned_for() {
        let mut game = world();
        game.players.retain(|p| p.id != "user_Goalkeeper2");
        game.players.push(academy(
            "user_young_keeper",
            "user",
            Position::Goalkeeper,
            18,
        ));
        let user_day = crate::ai_tactics::review_weekday("user");

        apply_ai_squad_planning(&mut game, user_day);

        assert_eq!(
            role(&game, "user_young_keeper"),
            domain::player::SquadRole::Youth
        );
    }

    #[test]
    fn a_contract_with_years_to_run_is_not_touched() {
        let mut game = world();
        game.players
            .push(player("star", "ai", Position::Midfielder, 75, 27, 800));
        let before = contract_end(&game, "star");

        apply_ai_contract_decisions(&mut game, review_day());

        assert_eq!(contract_end(&game, "star"), before);
    }

    /// A young player is kept for what he may become, not what he is.
    #[test]
    fn a_prospect_is_kept_whatever_his_rating() {
        let mut game = world();
        game.players
            .push(player("prospect", "ai", Position::Forward, 45, 20, 90));
        let before = contract_end(&game, "prospect");

        apply_ai_contract_decisions(&mut game, review_day());

        assert!(contract_end(&game, "prospect") > before);
    }

    /// Four years of contracts running out, one day at a time. Both clubs start
    /// alike — contracts ending across the next four years, ages spread from
    /// prospect to veteran — and only the AI club has anyone to renew them:
    /// it must still be at the squad floor at the end, while the player's club,
    /// which nobody renews for here, runs below it. No top-up is involved.
    #[test]
    fn an_ai_club_keeps_its_squad_across_seasons_by_renewing_it() {
        use crate::squad_floor::squad_shortfall;

        let mut game = world();
        for (index, player) in game.players.iter_mut().enumerate() {
            let days_left = 60 + (index as i64 * 97) % 1_400;
            player.stage_contract_end(Some(
                (today() + Duration::days(days_left))
                    .format("%Y-%m-%d")
                    .to_string(),
            ));
            let age = 19 + (index as i32 * 5) % 15;
            player.date_of_birth = format!("{}-01-01", TODAY.0 - age);
        }

        for _ in 0..4 * 365 {
            let weekday = game.clock.current_date.weekday().num_days_from_monday();
            apply_ai_contract_decisions(&mut game, weekday);
            crate::contracts::process_contract_expiries(&mut game);
            game.clock.advance_days(1);
        }

        assert!(
            squad_shortfall(&game, "ai").is_empty(),
            "the AI club fell below the floor: {:?}",
            squad_shortfall(&game, "ai")
        );
        assert!(
            !squad_shortfall(&game, "user").is_empty(),
            "the club nobody renewed for kept its squad too, so this test proves nothing"
        );
    }

    /// Given an AI club with a 21-year-old academy player, the oldest the academy takes
    /// When its review day comes
    /// Then he stays in the academy: only a player over 21 graduates
    #[test]
    fn an_ai_club_keeps_a_21_year_old_in_its_academy() {
        let mut game = world();
        game.players
            .push(academy("twenty_one", "ai", Position::Midfielder, 21));

        apply_ai_squad_planning(&mut game, review_day());

        assert_eq!(role(&game, "twenty_one"), domain::player::SquadRole::Youth);
    }

    /// Given an AI club with three senior keepers, one of whose contracts ends in exactly 183
    ///       days (the renewal horizon), and an 18-year-old academy keeper
    /// When its review day comes
    /// Then the academy keeper is promoted: a keeper whose contract ends on the horizon is not
    ///      counted toward the depth the club plans for
    #[test]
    fn a_senior_whose_contract_ends_on_the_renewal_horizon_does_not_count_toward_planned_depth() {
        let mut game = world();
        let horizon = (today() + Duration::days(RENEWAL_HORIZON_DAYS))
            .format("%Y-%m-%d")
            .to_string();
        game.players
            .iter_mut()
            .find(|p| p.id == "ai_Goalkeeper2")
            .unwrap()
            .stage_contract_end(Some(horizon));
        game.players
            .push(academy("young_keeper", "ai", Position::Goalkeeper, 18));

        apply_ai_squad_planning(&mut game, review_day());

        assert_eq!(
            role(&game, "young_keeper"),
            domain::player::SquadRole::Senior
        );
    }

    /// Given an AI club with three senior keepers, one of whose contracts ends 184 days away, a day
    ///       past the renewal horizon, and an 18-year-old academy keeper
    /// When its review day comes
    /// Then the academy keeper stays in the academy: that keeper still counts toward planned depth
    #[test]
    fn a_senior_whose_contract_ends_a_day_past_the_horizon_counts_toward_planned_depth() {
        let mut game = world();
        let past_horizon = (today() + Duration::days(RENEWAL_HORIZON_DAYS + 1))
            .format("%Y-%m-%d")
            .to_string();
        game.players
            .iter_mut()
            .find(|p| p.id == "ai_Goalkeeper2")
            .unwrap()
            .stage_contract_end(Some(past_horizon));
        game.players
            .push(academy("young_keeper", "ai", Position::Goalkeeper, 18));

        apply_ai_squad_planning(&mut game, review_day());

        assert_eq!(
            role(&game, "young_keeper"),
            domain::player::SquadRole::Youth
        );
    }

    /// Given an AI club with three keepers on long contracts, and a fourth, 34 and rated 50, whose
    ///       contract ends in 90 days
    /// When its review day comes
    /// Then his contract runs out: three other keepers already meet the planned depth of three
    #[test]
    fn a_fading_keeper_with_three_other_keepers_is_let_go() {
        let mut game = world();
        game.players
            .push(player("old_keeper", "ai", Position::Goalkeeper, 50, 34, 90));
        let before = contract_end(&game, "old_keeper");

        apply_ai_contract_decisions(&mut game, review_day());

        assert_eq!(contract_end(&game, "old_keeper"), before);
    }
}
