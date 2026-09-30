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

use chrono::NaiveDate;
use domain::player::Player;
use domain::team::Team;

use crate::contract_wage_policy::wage_policy_verdict;
use crate::contracts::{
    RenewalDecision, RenewalOffer, apply_agreed_renewal, contract_owner_team_id,
    evaluate_renewal_offer, expected_contract_years, expected_wage, next_renewal_round,
    player_age_on, remaining_contract_days,
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
    for team_id in clubs {
        let Some(team) = game.teams.iter().find(|team| &team.id == team_id).cloned() else {
            continue;
        };
        graduate_overage_academy_players(game, team_id, current_date);
        plan_squad_depth(game, &team);
    }
}

/// An academy player past the academy's age limit joins the senior squad.
fn graduate_overage_academy_players(game: &mut Game, team_id: &str, current_date: NaiveDate) {
    for player in game.players.iter_mut() {
        if player.team_id.as_deref() == Some(team_id)
            && player.squad_role == domain::player::SquadRole::Youth
            && player_age_on(current_date, &player.date_of_birth)
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
fn plan_squad_depth(game: &mut Game, team: &Team) {
    use crate::squad_floor::{MIN_PLAYERS_PER_GROUP, bring_in_one, groups_thinnest_first};

    for (group, floor) in MIN_PLAYERS_PER_GROUP {
        while seniors_in(game, &team.id, Some(&group)) < floor + PLANNING_MARGIN_PER_GROUP {
            if bring_in_one(game, team, &group).is_none() {
                break;
            }
        }
    }
    while seniors_in(game, &team.id, None) < PLANNING_TARGET_SENIORS {
        let groups = groups_thinnest_first(game, &team.id);
        if !groups
            .iter()
            .any(|group| bring_in_one(game, team, group).is_some())
        {
            break;
        }
    }
}

/// Seniors registered to the club who will still be under contract past the
/// renewal horizon, in one group or in all.
fn seniors_in(game: &Game, team_id: &str, group: Option<&domain::player::Position>) -> usize {
    let current_date = game.clock.current_date.date_naive();
    game.players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .filter(|player| player.squad_role == domain::player::SquadRole::Senior)
        // No end date is a contract the expiry sweep never ends: he stays.
        .filter(|player| {
            crate::contracts::contract_days_remaining(player.contract_end.as_deref(), current_date)
                .is_none_or(|days| days > RENEWAL_HORIZON_DAYS)
        })
        .filter(|player| group.is_none_or(|group| player.position.to_group_position() == *group))
        .count()
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
    let age = player_age_on(current_date, &player.date_of_birth);
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
        let mut players = Vec::new();
        for club in ["user", "ai"] {
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

    /// Given an AI club whose academy is full of low-rated youngsters, when a
    /// senior a little below the seniors' standard runs down his contract, he
    /// is judged against the seniors — and let go. Counting the academy would
    /// drag the standard down to him.
    #[test]
    fn the_squad_median_is_the_seniors_median() {
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
        game.players
            .push(player("journeyman", "ai", Position::Midfielder, 55, 29, 90));
        let before = contract_end(&game, "journeyman");

        apply_ai_contract_decisions(&mut game, review_day());

        assert_eq!(contract_end(&game, "journeyman"), before);
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
                other.wage = 0;
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
        loaned.team_id = Some("user".to_string());
        loaned.active_loan = Some(domain::player::ActiveLoan {
            parent_team_id: "ai".to_string(),
            loan_team_id: "user".to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: "2027-06-30".to_string(),
            wage_contribution_pct: 100,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        });
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
        star.contract_end = Some(day.format("%Y-%m-%d").to_string());
        game.players.push(star);

        crate::turn::process_day(&mut game);

        let star = game.players.iter().find(|p| p.id == "star").unwrap();
        assert_eq!(star.team_id.as_deref(), Some("ai"), "he was released");
        assert!(
            star.contract_end.as_deref() > Some(day.format("%Y-%m-%d").to_string().as_str()),
            "his contract was not renewed"
        );
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
        agent.contract_end = None;
        agent.wage = 0;
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
    }

    /// The same, but paying him would break the wage policy: he is not signed,
    /// because the club is at its keeper minimum, not below it.
    #[test]
    fn an_ai_club_does_not_break_its_wage_policy_for_a_margin() {
        let mut game = world();
        game.players.retain(|p| p.id != "ai_Goalkeeper2");
        for other in game.players.iter_mut() {
            if other.team_id.as_deref() == Some("ai") {
                other.wage = 0;
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
