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
//! by the same `apply_agreed_renewal` as the player's own negotiations — there
//! is no AI-only copy of the wage or contract-length rules.

use chrono::NaiveDate;
use domain::player::Player;
use domain::team::Team;

use crate::contracts::{
    RenewalDecision, RenewalOffer, apply_agreed_renewal, contract_owner_team_id,
    evaluate_renewal_offer, expected_contract_years, expected_wage, next_renewal_round,
    player_age_on, remaining_contract_days,
};
use crate::game::Game;
use crate::squad_floor::group_floor;

/// A contract with this many days or fewer left is one the club decides on.
/// Half a season: early enough to act on every review day until it is settled.
const RENEWAL_HORIZON_DAYS: i64 = 183;

/// Letting a player go must leave his position group this far above the squad
/// floor, or the club keeps him whatever else is true.
const DEPTH_MARGIN: usize = 1;

/// A player at least as good as the squad's median, and no older than this, is
/// one the club wants to keep.
const PEAK_AGE_LIMIT: i32 = 32;

/// A player this young is kept as a prospect, whatever his rating today.
const PROSPECT_AGE_LIMIT: i32 = 23;

/// Run every AI club's contract review that falls on this weekday.
pub fn apply_ai_contract_decisions(game: &mut Game, weekday_num: u32) {
    let current_date = game.clock.current_date.date_naive();
    for team_id in crate::ai_tactics::ai_clubs_reviewing_on(game, weekday_num) {
        let Some(team) = game.teams.iter().find(|team| team.id == team_id).cloned() else {
            continue;
        };
        for player_index in contracts_running_down(game, &team_id, current_date) {
            if keeps(game, &team, &game.players[player_index], current_date) {
                renew(game, &team, player_index, current_date);
            }
        }
    }
}

/// The club's own players (its loaned-out ones included) whose contracts end
/// within the horizon.
fn contracts_running_down(game: &Game, team_id: &str, current_date: NaiveDate) -> Vec<usize> {
    game.players
        .iter()
        .enumerate()
        .filter(|(_, player)| !player.retired && player.contract_end.is_some())
        .filter(|(_, player)| contract_owner_team_id(player) == Some(team_id))
        .filter(|(_, player)| remaining_contract_days(player, current_date) <= RENEWAL_HORIZON_DAYS)
        .map(|(index, _)| index)
        .collect()
}

/// Does the club want this player for another contract?
fn keeps(game: &Game, team: &Team, player: &Player, current_date: NaiveDate) -> bool {
    needed_for_depth(game, &team.id, player) || worth_keeping(game, team, player, current_date)
}

/// Letting him go would leave his group too close to the floor. A club keeps
/// such a player even over its wage policy: the alternative is signing someone
/// in an emergency on similar terms.
fn needed_for_depth(game: &Game, team_id: &str, player: &Player) -> bool {
    let group = player.position.to_group_position();
    let others = owned_players(game, team_id)
        .filter(|other| other.id != player.id)
        .filter(|other| other.position.to_group_position() == group)
        .count();
    others < group_floor(&group) + DEPTH_MARGIN
}

/// Good enough and young enough to want, and affordable.
fn worth_keeping(game: &Game, team: &Team, player: &Player, current_date: NaiveDate) -> bool {
    let age = player_age_on(current_date, &player.date_of_birth);
    let wanted = age <= PROSPECT_AGE_LIMIT
        || (age <= PEAK_AGE_LIMIT && player.ovr >= squad_median_ovr(game, &team.id));
    let wage = expected_wage(player, team, current_date);
    wanted && crate::contract_wage_policy::renewal_wage_policy_allows(game, team, player, wage)
}

fn owned_players<'a>(game: &'a Game, team_id: &'a str) -> impl Iterator<Item = &'a Player> {
    game.players
        .iter()
        .filter(move |player| !player.retired && contract_owner_team_id(player) == Some(team_id))
}

fn squad_median_ovr(game: &Game, team_id: &str) -> u8 {
    let mut ratings: Vec<u8> = owned_players(game, team_id).map(|p| p.ovr).collect();
    if ratings.is_empty() {
        return 0;
    }
    ratings.sort_unstable();
    ratings[ratings.len() / 2]
}

/// Offer what the player expects and, if he takes it, put it into effect.
fn renew(game: &mut Game, team: &Team, player_index: usize, current_date: NaiveDate) {
    let player = &game.players[player_index];
    let offer = RenewalOffer {
        weekly_wage: expected_wage(player, team, current_date),
        contract_years: expected_contract_years(player, current_date),
    };
    let outcome = evaluate_renewal_offer(player, team, current_date, &offer);
    if outcome.decision != RenewalDecision::Accepted {
        return;
    }
    let round = next_renewal_round(player, None);
    if let Err(error) = apply_agreed_renewal(
        &mut game.players[player_index],
        offer.weekly_wage,
        offer.contract_years,
        current_date,
        round,
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
        player.wage = 1_000;
        player.contract_end = Some(
            (today() + Duration::days(days_left))
                .format("%Y-%m-%d")
                .to_string(),
        );
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

    /// Two clubs of sixteen — three or more per group, so nobody is needed for
    /// depth unless a test says so — every contract long, rated 60, aged 26.
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
        let mut players = Vec::new();
        for club in ["user", "ai"] {
            for (group, count) in [
                (Position::Goalkeeper, 3),
                (Position::Defender, 5),
                (Position::Midfielder, 5),
                (Position::Forward, 3),
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
        }
        Game::new(
            clock,
            manager,
            vec![team("user"), team("ai")],
            players,
            vec![],
            vec![],
        )
    }

    fn review_day() -> u32 {
        crate::ai_tactics::review_weekday("ai")
    }

    fn contract_end(game: &Game, id: &str) -> String {
        game.players
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .contract_end
            .clone()
            .unwrap()
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
            player.contract_end = Some(
                (today() + Duration::days(days_left))
                    .format("%Y-%m-%d")
                    .to_string(),
            );
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
}
