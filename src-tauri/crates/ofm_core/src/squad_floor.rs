//! A club never runs out of players.
//!
//! The floor is how many players of each position group a club must have
//! registered — injured or not — to put a side out at all, with a keeper to
//! spare. It is one rule with one home: the generator builds clubs to it, and
//! every path that can take players away from a club answers to it.

use crate::game::Game;
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::player::{Player, Position};
use std::collections::HashMap;

/// Players a club must keep registered in each position group, in
/// `[GK, DEF, MID, FWD]` order. Twelve in all: an eleven and a second keeper,
/// because a club whose only keeper is injured still has to put one in goal.
pub const MIN_PLAYERS_PER_GROUP: [(Position, usize); 4] = [
    (Position::Goalkeeper, 2),
    (Position::Defender, 4),
    (Position::Midfielder, 4),
    (Position::Forward, 2),
];

/// The floor for one position group, or 0 for a group with none.
pub(crate) fn group_floor(group: &Position) -> usize {
    MIN_PLAYERS_PER_GROUP
        .iter()
        .find(|(position, _)| position == group)
        .map(|(_, floor)| *floor)
        .unwrap_or(0)
}

/// Every position group this club is short in, with how many players it is
/// short by. Empty when the club is at or above the floor everywhere.
///
/// Counts every player registered to the club — injured players included,
/// since they are still the club's to field in a crisis, and players out on
/// loan excluded, since they are registered to their borrower.
pub fn squad_shortfall(game: &Game, team_id: &str) -> Vec<(Position, usize)> {
    let mut registered = [0; 4];
    for player in &game.players {
        if player.team_id.as_deref() == Some(team_id) {
            registered[group_index(&player.position)] += 1;
        }
    }
    shortfall_of(registered)
}

/// Where a position's group sits in [`MIN_PLAYERS_PER_GROUP`]: 0 GK, 1 DEF,
/// 2 MID, 3 FWD. The one place that order is written down.
pub(crate) fn group_index(position: &Position) -> usize {
    let group = position.to_group_position();
    MIN_PLAYERS_PER_GROUP
        .iter()
        .position(|(floor_group, _)| *floor_group == group)
        .expect("every position belongs to one of the four groups")
}

/// The shortfall for a club with `registered` players per group, in
/// [`MIN_PLAYERS_PER_GROUP`] order.
fn shortfall_of(registered: [usize; 4]) -> Vec<(Position, usize)> {
    MIN_PLAYERS_PER_GROUP
        .iter()
        .zip(registered)
        .filter(|((_, floor), have)| have < floor)
        .map(|((group, floor), have)| (group.clone(), floor - have))
        .collect()
}

/// Registered players per club and group, for every club, in one pass over
/// the world's players. A club with nobody registered is absent.
pub(crate) fn registered_by_club(game: &Game) -> HashMap<&str, [usize; 4]> {
    let mut by_club: HashMap<&str, [usize; 4]> = HashMap::new();
    for player in &game.players {
        if let Some(team_id) = player.team_id.as_deref() {
            by_club.entry(team_id).or_default()[group_index(&player.position)] += 1;
        }
    }
    by_club
}

/// The group this player's departure would take below the floor, if any.
///
/// Answers for the club he is registered to, which is the club that loses him:
/// the buyer or borrower only gains. A club already short in his group cannot
/// let him go either — the floor is not a line a club may cross further.
pub(crate) fn departure_would_leave_short(game: &Game, player_id: &str) -> Option<Position> {
    let player = game.players.iter().find(|player| player.id == player_id)?;
    let team_id = player.team_id.as_deref()?;
    club_needs_him_for_the_floor(game, team_id, player).then(|| player.position.to_group_position())
}

/// Players of this player's group registered to `team_id`, him excluded —
/// the floor's own count, so a player out on loan counts for his borrower.
pub(crate) fn others_in_his_group(game: &Game, team_id: &str, player: &Player) -> usize {
    let group = player.position.to_group_position();
    game.players
        .iter()
        .filter(|other| other.id != player.id && other.team_id.as_deref() == Some(team_id))
        .filter(|other| other.position.to_group_position() == group)
        .count()
}

/// Whether `team_id` would be below the floor in this player's group without
/// him: he is one of its players now and might leave (a renewal, a sale), or
/// he is not yet and the club is short (a signing).
///
/// This is also the one case in which the board's wage policy yields — see
/// [`crate::contract_wage_policy::wage_policy_verdict`].
pub(crate) fn club_needs_him_for_the_floor(game: &Game, team_id: &str, player: &Player) -> bool {
    others_in_his_group(game, team_id, player) < group_floor(&player.position.to_group_position())
}

/// Whether a club with `registered` players per group (in
/// [`MIN_PLAYERS_PER_GROUP`] order) can let one player of `position`'s group go
/// and stay at the floor. The one statement of that rule, for the refusals and
/// for the AI market deciding who is worth approaching.
pub(crate) fn can_spare_one(registered: [usize; 4], position: &Position) -> bool {
    let group = group_index(position);
    registered[group] > MIN_PLAYERS_PER_GROUP[group].1
}

/// [`departure_would_leave_short`] as the refusal a sale, loan or release
/// returns, naming the group it would leave the club short of.
pub(crate) fn ensure_departure_keeps_floor(game: &Game, player_id: &str) -> Result<(), String> {
    match departure_would_leave_short(game, player_id) {
        Some(group) => Err(crate::contracts::backend_text_with_param(
            WOULD_LEAVE_SQUAD_SHORT_ERROR,
            "group",
            &position_group_key(&group),
        )),
        None => Ok(()),
    }
}

/// The frontend key naming a position group, resolved there as a param value.
fn position_group_key(group: &Position) -> String {
    format!("common.positionGroups.{group:?}")
}

const WOULD_LEAVE_SQUAD_SHORT_ERROR: &str = "be.error.squadFloor.wouldLeaveShort";

/// The daily check, run once every way a club can lose players has had its
/// turn: an AI club short of the floor signs its way back to it; the player's
/// own club is told which groups are short and left to act, because signing
/// players on a manager's behalf is his decision until a match cannot go ahead
/// without them ([`ready_for_kick_off`]).
pub(crate) fn keep_squads_at_the_floor(game: &mut Game) {
    for (team_id, shortfall) in clubs_below_the_floor(game) {
        if Some(&team_id) == game.manager.team_id.as_ref() {
            warn_user_club_is_short(game, &team_id, &shortfall);
        } else {
            let signed = restore_minimum_squad(game, &team_id);
            log::info!(
                "[squad_floor] {team_id} was short {shortfall:?}; signed {}",
                signed.len()
            );
        }
    }
}

/// Bring a loaded or freshly built world up to the floor.
///
/// A save written before the floor was enforced, or a package whose author
/// left a club thin, can open with clubs already short. The same rule as the
/// daily check, with one difference: when the player's club has a match today,
/// the warning would arrive with no time to act on it, so it is left to the
/// kick-off top-up and its own message instead. Returns whether anything
/// changed, so a loaded save knows it needs writing back.
pub fn repair_squads_on_load(game: &mut Game) -> bool {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let user_club_plays_today =
        game.manager.team_id.as_ref().is_some_and(|team_id| {
            crate::training::teams_playing_on(game, &today).contains(team_id)
        });
    let short_clubs = clubs_below_the_floor(game);
    let changed = !short_clubs.is_empty();
    for (team_id, shortfall) in short_clubs {
        if Some(&team_id) == game.manager.team_id.as_ref() {
            if !user_club_plays_today {
                warn_user_club_is_short(game, &team_id, &shortfall);
            }
        } else {
            let signed = restore_minimum_squad(game, &team_id);
            log::info!(
                "[squad_floor] on load, {team_id} was short {shortfall:?}; signed {}",
                signed.len()
            );
        }
    }
    changed
}

/// The last line: a club about to kick off short of players signs free agents
/// first, so every match is played by two sides that can be fielded.
///
/// The player's club is only topped up when it could not put a side out at all
/// — fewer than eleven registered, or no goalkeeper — because until then who to
/// sign is the manager's call, and the daily warnings have told them. When it
/// is topped up, it is brought all the way to the floor and told who was
/// signed. An AI club is topped up whenever it is short; the daily check should
/// have got there first, so a top-up here is logged as the sign of a gap in it.
pub(crate) fn ready_for_kick_off(game: &mut Game, team_id: &str) {
    let is_users_club = Some(team_id) == game.manager.team_id.as_deref();
    if is_users_club && can_put_a_side_out(game, team_id) {
        return;
    }
    let signed = restore_minimum_squad(game, team_id);
    if signed.is_empty() {
        return;
    }
    if is_users_club {
        let message = squad_topped_up_message(game, team_id, &signed);
        crate::inbox::emit(game, message);
    } else {
        log::warn!(
            "[squad_floor] {team_id} reached kick-off short of the floor; signed {}",
            signed.len()
        );
    }
}

/// Players a side puts on the pitch.
const PLAYERS_ON_THE_PITCH: usize = 11;

/// Whether the club has eleven players registered, a goalkeeper among them.
/// Injured players count: the match builder fields them when it must.
fn can_put_a_side_out(game: &Game, team_id: &str) -> bool {
    let mut registered = 0;
    let mut goalkeepers = 0;
    for player in &game.players {
        if player.team_id.as_deref() == Some(team_id) {
            registered += 1;
            if player.position.to_group_position() == Position::Goalkeeper {
                goalkeepers += 1;
            }
        }
    }
    registered >= PLAYERS_ON_THE_PITCH && goalkeepers > 0
}

/// Every club short of the floor, with its shortfall, in the order clubs are
/// stored so the signings a day makes do not depend on hashing.
fn clubs_below_the_floor(game: &Game) -> Vec<(String, Vec<(Position, usize)>)> {
    let registered = registered_by_club(game);
    game.teams
        .iter()
        .filter_map(|team| {
            let shortfall = shortfall_of(
                registered
                    .get(team.id.as_str())
                    .copied()
                    .unwrap_or_default(),
            );
            (!shortfall.is_empty()).then(|| (team.id.clone(), shortfall))
        })
        .collect()
}

/// One message per short group. Keyed on the season, the group and how many
/// are left, so the player hears once when a group goes short and again if it
/// gets worse — not every day it stays that way.
fn warn_user_club_is_short(game: &mut Game, team_id: &str, shortfall: &[(Position, usize)]) {
    let season = crate::inbox::recurrence_season(game);
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let team_name = game.team_name_or_id(team_id);
    for (group, missing) in shortfall {
        let floor = group_floor(group);
        let have = floor - missing;
        let key = format!("squad_below_floor_{team_id}_{season}_{group:?}_{have}");
        let mut params = HashMap::new();
        params.insert("team".to_string(), team_name.clone());
        params.insert("group".to_string(), position_group_key(group));
        params.insert("have".to_string(), have.to_string());
        params.insert("floor".to_string(), floor.to_string());
        let message = InboxMessage::new(
            key,
            String::new(),
            String::new(),
            String::new(),
            date.clone(),
        )
        .with_category(MessageCategory::Contract)
        .with_priority(MessagePriority::Urgent)
        .with_sender_role("")
        .with_i18n(
            "be.msg.squadBelowFloor.subject",
            "be.msg.squadBelowFloor.body",
            params,
        )
        .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager");
        crate::inbox::emit(game, message);
    }
}

fn squad_topped_up_message(game: &Game, team_id: &str, signed: &[String]) -> InboxMessage {
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let names = signed
        .iter()
        .filter_map(|id| game.players.iter().find(|player| &player.id == id))
        .map(|player| player.full_name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let mut params = HashMap::new();
    params.insert("team".to_string(), game.team_name_or_id(team_id));
    params.insert("players".to_string(), names);
    InboxMessage::new(
        format!("squad_topped_up_{team_id}_{date}"),
        String::new(),
        String::new(),
        String::new(),
        date,
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_i18n(
        "be.msg.squadToppedUp.subject",
        "be.msg.squadToppedUp.body",
        params,
    )
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

/// Bring a club back up to the floor by signing free agents, and return the
/// ids of the players it signed (empty when the club was not short).
///
/// For each group the club is short in, the best free agent of that group
/// (highest rating, not retired, fit before injured) is signed on the terms
/// the contracts module expects him to want. When nobody suitable is on the
/// market a free agent is generated for the club's country and signed the same
/// way, so the club is never left short. The wage policy always yields here,
/// by the same rule every renewal and signing answers to
/// ([`crate::contract_wage_policy::wage_policy_verdict`]): the club is short in
/// that group, and "we cannot afford to field a team" is not an outcome the
/// game allows.
pub(crate) fn restore_minimum_squad(game: &mut Game, team_id: &str) -> Vec<String> {
    use chrono::Datelike;

    let shortfall = squad_shortfall(game, team_id);
    if shortfall.is_empty() {
        return Vec::new();
    }
    let Some(team) = game.teams.iter().find(|team| team.id == team_id).cloned() else {
        log::error!("[squad_floor] cannot restore the squad of unknown club {team_id}");
        return Vec::new();
    };
    let current_date = game.clock.current_date.date_naive();

    let mut signed = Vec::new();
    for (group, missing) in shortfall {
        for _ in 0..missing {
            let index = best_free_agent(game, &group).unwrap_or_else(|| {
                game.players.push(crate::generator::generate_free_agent(
                    &group,
                    &team.country,
                    current_date.year() as u32,
                ));
                game.players.len() - 1
            });
            let wage = crate::contracts::expected_wage(&game.players[index], &team, current_date);
            let years =
                crate::contracts::expected_contract_years(&game.players[index], current_date);
            debug_assert!(
                crate::contract_wage_policy::wage_policy_verdict(
                    game,
                    &team,
                    &game.players[index],
                    wage
                )
                .permits(),
                "a club short of the floor was refused a signing on wages"
            );
            match crate::contracts::sign_free_agent(game, index, &team, wage, years, current_date) {
                Ok(()) => signed.push(game.players[index].id.clone()),
                Err(error) => log::error!(
                    "[squad_floor] could not sign {} for {team_id}: {error}",
                    game.players[index].id
                ),
            }
        }
    }
    signed
}

/// The free agent of this group a club would sign first: not retired, fit
/// before injured, then the highest rating. Ties fall to the lowest id so the
/// answer does not depend on the order players are stored in.
fn best_free_agent(game: &Game, group: &Position) -> Option<usize> {
    game.players
        .iter()
        .enumerate()
        .filter(|(_, player)| player.team_id.is_none() && !player.retired)
        .filter(|(_, player)| player.position.to_group_position() == *group)
        .max_by(|(_, left), (_, right)| {
            left.injury
                .is_none()
                .cmp(&right.injury.is_none())
                .then(left.ovr.cmp(&right.ovr))
                .then_with(|| right.id.cmp(&left.id))
        })
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes};
    use domain::team::Team;

    fn attrs() -> PlayerAttributes {
        PlayerAttributes {
            pace: 60,
            stamina: 60,
            strength: 60,
            agility: 60,
            passing: 60,
            shooting: 60,
            tackling: 60,
            dribbling: 60,
            defending: 60,
            positioning: 60,
            vision: 60,
            decisions: 60,
            composure: 60,
            aggression: 60,
            teamwork: 60,
            leadership: 60,
            handling: 60,
            reflexes: 60,
            aerial: 60,
        }
    }

    pub(super) fn player(id: &str, team_id: Option<&str>, position: Position) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1998-01-01".to_string(),
            "England".to_string(),
            position,
            attrs(),
        );
        player.team_id = team_id.map(str::to_string);
        player
    }

    /// A club with exactly `per_group` players in each group, `[GK, DEF, MID, FWD]`.
    pub(super) fn club_with(per_group: [usize; 4]) -> Game {
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap());
        let mut manager = Manager::new(
            "mgr".to_string(),
            "Test".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        manager.hire("user".to_string());
        let team = Team::new(
            "club".to_string(),
            "Club".to_string(),
            "CLB".to_string(),
            "England".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20_000,
        );
        let mut players = Vec::new();
        for ((group, _), count) in MIN_PLAYERS_PER_GROUP.iter().zip(per_group) {
            for i in 0..count {
                players.push(player(
                    &format!("{group:?}{i}"),
                    Some("club"),
                    group.clone(),
                ));
            }
        }
        Game::new(clock, manager, vec![team], players, vec![], vec![])
    }

    #[test]
    fn a_club_at_the_floor_is_short_of_nothing() {
        let game = club_with([2, 4, 4, 2]);
        assert!(squad_shortfall(&game, "club").is_empty());
    }

    #[test]
    fn a_club_is_short_by_group_and_by_how_many() {
        let game = club_with([0, 3, 4, 5]);
        assert_eq!(
            squad_shortfall(&game, "club"),
            vec![(Position::Goalkeeper, 2), (Position::Defender, 1)]
        );
    }

    #[test]
    fn a_club_with_nobody_is_short_everywhere() {
        let game = club_with([0, 0, 0, 0]);
        assert_eq!(
            squad_shortfall(&game, "club"),
            MIN_PLAYERS_PER_GROUP.to_vec()
        );
    }

    /// A player out on loan is registered to his borrower, and an injured one
    /// is still the club's to field. Granular positions count in their group.
    #[test]
    fn injured_players_count_and_loaned_out_players_do_not() {
        let mut game = club_with([2, 4, 4, 1]);
        let mut injured = player("injured_striker", Some("club"), Position::Striker);
        injured.injury = Some(domain::player::Injury {
            name: "common.injuries.calfStrain".to_string(),
            days_remaining: 30,
        });
        game.players.push(injured);
        game.players
            .push(player("loaned_out", Some("borrower"), Position::Goalkeeper));
        assert!(squad_shortfall(&game, "club").is_empty());
    }

    fn free_agent(id: &str, position: Position, ovr: u8) -> Player {
        let mut agent = player(id, None, position);
        agent.ovr = ovr;
        agent.contract_end = None;
        agent.wage = 0;
        agent
    }

    /// Short by one keeper, with two keepers and a forward on the market: the
    /// better keeper is signed, on a real contract, and nobody else.
    #[test]
    fn a_short_club_signs_the_best_free_agent_of_the_group_it_is_short_in() {
        let mut game = club_with([1, 4, 4, 2]);
        game.players
            .push(free_agent("weak_keeper", Position::Goalkeeper, 55));
        game.players
            .push(free_agent("good_keeper", Position::Goalkeeper, 70));
        game.players
            .push(free_agent("forward", Position::Forward, 80));

        let signed = restore_minimum_squad(&mut game, "club");

        assert_eq!(signed, vec!["good_keeper".to_string()]);
        assert!(squad_shortfall(&game, "club").is_empty());
        let keeper = game.players.iter().find(|p| p.id == "good_keeper").unwrap();
        assert_eq!(keeper.team_id.as_deref(), Some("club"));
        assert!(keeper.contract_end.is_some(), "signed without a contract");
        assert!(keeper.wage > 0, "signed on no wage");
        let forward = game.players.iter().find(|p| p.id == "forward").unwrap();
        assert_eq!(
            forward.team_id, None,
            "a club short of keepers signed a forward"
        );
    }

    #[test]
    fn a_retired_player_is_never_signed() {
        let mut game = club_with([1, 4, 4, 2]);
        let mut retired = free_agent("retired_keeper", Position::Goalkeeper, 90);
        retired.retired = true;
        game.players.push(retired);

        let signed = restore_minimum_squad(&mut game, "club");

        assert_eq!(signed.len(), 1);
        assert_ne!(signed[0], "retired_keeper");
        assert!(squad_shortfall(&game, "club").is_empty());
    }

    /// With nobody on the market the club still gets its players: generated
    /// free agents of the right group, signed like any other.
    #[test]
    fn a_club_with_nobody_to_sign_is_given_generated_free_agents() {
        let mut game = club_with([0, 0, 0, 0]);

        let signed = restore_minimum_squad(&mut game, "club");

        assert_eq!(signed.len(), 12);
        assert!(squad_shortfall(&game, "club").is_empty());
        for id in &signed {
            let player = game.players.iter().find(|p| &p.id == id).unwrap();
            assert_eq!(player.team_id.as_deref(), Some("club"));
            assert!(player.contract_end.is_some());
        }
    }

    #[test]
    fn a_club_at_the_floor_signs_nobody() {
        let mut game = club_with([2, 4, 4, 2]);
        game.players
            .push(free_agent("keeper", Position::Goalkeeper, 90));
        assert!(restore_minimum_squad(&mut game, "club").is_empty());
    }

    #[test]
    fn a_player_cannot_leave_a_group_that_is_at_the_floor() {
        let game = club_with([2, 5, 4, 2]);
        assert_eq!(
            departure_would_leave_short(&game, "Goalkeeper0"),
            Some(Position::Goalkeeper)
        );
        assert_eq!(departure_would_leave_short(&game, "Defender0"), None);
        assert!(ensure_departure_keeps_floor(&game, "Defender0").is_ok());
        assert_eq!(
            ensure_departure_keeps_floor(&game, "Forward1"),
            Err(
                "be.error.squadFloor.wouldLeaveShort?group=common.positionGroups.Forward"
                    .to_string()
            )
        );
    }

    /// A player who is not at a club leaves nobody short.
    #[test]
    fn a_free_agent_leaves_nobody_short() {
        let mut game = club_with([2, 4, 4, 2]);
        game.players
            .push(free_agent("agent", Position::Goalkeeper, 70));
        assert_eq!(departure_would_leave_short(&game, "agent"), None);
    }

    /// The same squad as [`club_with`], managed by the player.
    fn users_club_with(per_group: [usize; 4]) -> Game {
        let mut game = club_with(per_group);
        game.manager.hire("club".to_string());
        game
    }

    fn inbox_keys(game: &Game) -> Vec<(&str, Option<&str>)> {
        game.messages
            .iter()
            .map(|message| {
                (
                    message.body_key.as_deref().unwrap_or(""),
                    message.i18n_params.get("group").map(String::as_str),
                )
            })
            .collect()
    }

    #[test]
    fn the_daily_check_tops_up_an_ai_club_and_only_warns_the_player() {
        let mut game = users_club_with([1, 4, 4, 1]);
        game.teams.push(Team::new(
            "rival".to_string(),
            "Rival".to_string(),
            "RIV".to_string(),
            "England".to_string(),
            "Leeds".to_string(),
            "Park".to_string(),
            10_000,
        ));

        keep_squads_at_the_floor(&mut game);

        assert!(
            squad_shortfall(&game, "rival").is_empty(),
            "the AI club was left short"
        );
        assert_eq!(
            squad_shortfall(&game, "club"),
            vec![(Position::Goalkeeper, 1), (Position::Forward, 1)],
            "players were signed for the player's club without asking"
        );
        assert_eq!(
            inbox_keys(&game),
            vec![
                (
                    "be.msg.squadBelowFloor.body",
                    Some("common.positionGroups.Goalkeeper")
                ),
                (
                    "be.msg.squadBelowFloor.body",
                    Some("common.positionGroups.Forward")
                ),
            ]
        );
        let keeper_warning = &game.messages[0].i18n_params;
        assert_eq!(keeper_warning.get("have").map(String::as_str), Some("1"));
        assert_eq!(keeper_warning.get("floor").map(String::as_str), Some("2"));
    }

    /// A shortfall that lasts is one warning, not one a day; the same group
    /// getting worse is news again.
    #[test]
    fn the_player_is_warned_once_until_the_shortfall_gets_worse() {
        let mut game = users_club_with([1, 4, 4, 2]);
        keep_squads_at_the_floor(&mut game);
        game.clock.advance_days(1);
        keep_squads_at_the_floor(&mut game);
        assert_eq!(game.messages.len(), 1);

        game.messages.clear();
        game.players.retain(|player| player.id != "Goalkeeper0");
        keep_squads_at_the_floor(&mut game);
        assert_eq!(game.messages.len(), 1, "a worse shortfall went unreported");
        assert_eq!(
            game.messages[0].i18n_params.get("have").map(String::as_str),
            Some("0")
        );
    }

    /// No goalkeeper at all: the match cannot go ahead as things stand, so the
    /// club is brought to the floor — both keepers — and told who they are.
    #[test]
    fn at_kick_off_a_players_club_that_cannot_field_a_side_is_topped_up_and_told() {
        let mut game = users_club_with([0, 4, 4, 2]);
        game.players
            .push(free_agent("keeper", Position::Goalkeeper, 70));

        ready_for_kick_off(&mut game, "club");

        assert!(squad_shortfall(&game, "club").is_empty());
        assert_eq!(game.messages.len(), 1);
        let message = &game.messages[0];
        assert_eq!(
            message.body_key.as_deref(),
            Some("be.msg.squadToppedUp.body")
        );
        let named = message.i18n_params.get("players").unwrap();
        assert!(named.starts_with("keeper, "), "signings named: {named}");
    }

    /// Eleven players and a keeper can play. Short of a spare keeper is a
    /// reason to warn, not to sign someone on the manager's behalf.
    #[test]
    fn at_kick_off_a_players_club_that_can_field_a_side_is_left_to_its_manager() {
        let mut game = users_club_with([1, 4, 4, 2]);
        game.players
            .push(free_agent("keeper", Position::Goalkeeper, 70));

        ready_for_kick_off(&mut game, "club");

        assert!(game.messages.is_empty());
        let keeper = game.players.iter().find(|p| p.id == "keeper").unwrap();
        assert_eq!(keeper.team_id, None);
    }

    /// Ten outfield players and a keeper registered, one short of a side.
    #[test]
    fn at_kick_off_a_players_club_of_ten_is_topped_up() {
        let mut game = users_club_with([1, 4, 4, 1]);
        ready_for_kick_off(&mut game, "club");
        assert!(squad_shortfall(&game, "club").is_empty());
        assert_eq!(game.messages.len(), 1);
    }

    #[test]
    fn at_kick_off_an_ai_club_is_topped_up_without_a_message() {
        let mut game = club_with([0, 0, 0, 0]);
        ready_for_kick_off(&mut game, "club");
        assert!(squad_shortfall(&game, "club").is_empty());
        assert!(game.messages.is_empty());
    }

    #[test]
    fn at_kick_off_a_club_at_the_floor_is_left_alone() {
        let mut game = users_club_with([2, 4, 4, 2]);
        game.players
            .push(free_agent("keeper", Position::Goalkeeper, 70));
        ready_for_kick_off(&mut game, "club");
        assert!(game.messages.is_empty());
        let keeper = game.players.iter().find(|p| p.id == "keeper").unwrap();
        assert_eq!(keeper.team_id, None);
    }

    fn with_fixture_on(game: &mut Game, date: &str) {
        let teams = vec!["club".to_string(), "rival".to_string()];
        let mut league =
            domain::league::League::new("league".to_string(), "League".to_string(), 2026, &teams);
        league.fixtures.push(domain::league::Fixture {
            id: "fixture".to_string(),
            competition_id: "league".to_string(),
            matchday: 1,
            date: date.to_string(),
            home_team_id: "club".to_string(),
            away_team_id: "rival".to_string(),
            competition: domain::league::FixtureCompetition::League,
            status: domain::league::FixtureStatus::Scheduled,
            result: None,
        });
        game.league = Some(league);
    }

    /// On load, the player hears about a short squad — unless the club plays
    /// today, when the warning would come with no time to act and kick-off
    /// sends its own message instead.
    #[test]
    fn load_repair_warns_the_player_unless_the_club_plays_today() {
        let mut quiet_day = users_club_with([1, 4, 4, 2]);
        with_fixture_on(&mut quiet_day, "2026-08-08");
        assert!(repair_squads_on_load(&mut quiet_day));
        assert_eq!(quiet_day.messages.len(), 1);
        assert!(!squad_shortfall(&quiet_day, "club").is_empty());

        let mut matchday = users_club_with([1, 4, 4, 2]);
        with_fixture_on(&mut matchday, "2026-08-01");
        assert!(repair_squads_on_load(&mut matchday));
        assert!(matchday.messages.is_empty());
        assert!(!squad_shortfall(&matchday, "club").is_empty());
    }

    #[test]
    fn load_repair_tops_up_ai_clubs_and_reports_nothing_to_do_when_all_are_full() {
        let mut game = club_with([0, 4, 4, 2]);
        assert!(repair_squads_on_load(&mut game));
        assert!(squad_shortfall(&game, "club").is_empty());
        assert!(
            !repair_squads_on_load(&mut game),
            "a full world was rewritten"
        );
    }
}
