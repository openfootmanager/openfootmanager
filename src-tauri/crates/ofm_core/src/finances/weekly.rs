//! The posts a club's week produces on a Monday: its wages, upkeep and sponsorship.

use std::collections::HashMap;

use chrono::NaiveDate;
use domain::finance::CashKind;

use super::league_position::league_positions;
use super::{PostRequest, calc_upkeep, evaluate_sponsorship_bonus, player_weekly_wage_for_team};
use crate::game::Game;

/// Each club's weekly player wages and weekly staff wages, in two maps by club id.
fn weekly_wages_by_team(game: &Game) -> (HashMap<String, i64>, HashMap<String, i64>) {
    // Sum each member's weekly wage into its team in a single pass, instead of
    // rescanning every player and staff member once per team
    // (O(teams * (players + staff)) -> O(teams + players + staff)).
    let mut player_wages_by_team: HashMap<String, i64> = HashMap::new();
    let mut staff_wages_by_team: HashMap<String, i64> = HashMap::new();
    for player in &game.players {
        if let Some(loan) = &player.active_loan {
            add_weekly_wage(
                &mut player_wages_by_team,
                &loan.loan_team_id,
                player_weekly_wage_for_team(player, &loan.loan_team_id),
            );
            add_weekly_wage(
                &mut player_wages_by_team,
                &loan.parent_team_id,
                player_weekly_wage_for_team(player, &loan.parent_team_id),
            );
        } else if let Some(team_id) = &player.team_id {
            add_weekly_wage(&mut player_wages_by_team, team_id, i64::from(player.wage()));
        }
    }
    for staff_member in &game.staff {
        if let Some(team_id) = &staff_member.team_id {
            add_weekly_wage(
                &mut staff_wages_by_team,
                team_id,
                i64::from(staff_member.wage),
            );
        }
    }

    (player_wages_by_team, staff_wages_by_team)
}

fn add_weekly_wage(map: &mut HashMap<String, i64>, team_id: &str, amount: i64) {
    if amount == 0 {
        return;
    }
    *map.entry(team_id.to_string()).or_default() += amount;
}

/// Every club's posts for the week, in the order the teams are listed: what it pays in wages and
/// upkeep and what its sponsor pays, the league-position bonus judged on the club's own table.
pub(super) fn weekly_posts_by_club(
    game: &Game,
    post_date: NaiveDate,
) -> Vec<(String, Vec<PostRequest>)> {
    let (player_wages_by_team, staff_wages_by_team) = weekly_wages_by_team(game);
    let position_by_team = league_positions(game);

    let mut weekly_by_club: Vec<(String, Vec<PostRequest>)> = Vec::new();
    for team in &game.teams {
        let mut reqs = Vec::new();
        let player_wages = player_wages_by_team.get(&team.id).copied().unwrap_or(0);
        let staff_wages = staff_wages_by_team.get(&team.id).copied().unwrap_or(0);
        let upkeep = calc_upkeep(team);
        if player_wages != 0 {
            reqs.push(PostRequest::new(
                &team.id,
                -player_wages,
                CashKind::PlayerWages,
                post_date,
            ));
        }
        if staff_wages != 0 {
            reqs.push(PostRequest::new(
                &team.id,
                -staff_wages,
                CashKind::StaffWages,
                post_date,
            ));
        }
        if upkeep != 0 {
            reqs.push(PostRequest::new(
                &team.id,
                -upkeep,
                CashKind::Upkeep,
                post_date,
            ));
        }
        let current_position = position_by_team.get(&team.id).copied();
        let sponsorship_income = team
            .sponsorship
            .as_ref()
            .map(|sponsorship| {
                sponsorship.base_value
                    + evaluate_sponsorship_bonus(current_position, &team.form, sponsorship)
            })
            .unwrap_or(0);
        if sponsorship_income > 0 {
            reqs.push(PostRequest::new(
                &team.id,
                sponsorship_income,
                CashKind::Sponsorship,
                post_date,
            ));
        }
        weekly_by_club.push((team.id.clone(), reqs));
    }
    weekly_by_club
}
