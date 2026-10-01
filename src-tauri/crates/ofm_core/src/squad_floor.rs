//! A club never runs out of players.
//!
//! The floor is what a club needs to keep itself sound: at least
//! [`MIN_SENIOR_PLAYERS`] senior players registered, injured or not, and
//! within them [`MIN_PLAYERS_PER_GROUP`] of each position group — an eleven and
//! a keeper to spare. It is one rule with one home: the generator builds clubs
//! above it, every path that can take a player away from a club answers to it,
//! and an AI club's ordinary squad planning keeps it clear of it.
//!
//! When a club is short anyway, it is filled only from players who exist: its
//! own academy first, then the free-agent market. The game never creates a
//! player, or money, for a club. When neither source has anyone, the gap is
//! reported, not papered over.

use crate::game::Game;
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::player::{Player, Position, SquadRole};
use domain::team::Team;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Players a club must keep registered in each position group, in
/// `[GK, DEF, MID, FWD]` order: an eleven and a second keeper, because a club
/// whose only keeper is injured still has to put one in goal.
pub const MIN_PLAYERS_PER_GROUP: [(Position, usize); 4] = [
    (Position::Goalkeeper, 2),
    (Position::Defender, 4),
    (Position::Midfielder, 4),
    (Position::Forward, 2),
];

/// Senior players a club must keep registered in all. The maintainer's rule: a
/// club cannot keep itself sound with fewer than fifteen — an eleven, a keeper
/// to spare and enough cover for a normal run of injuries and suspensions.
pub const MIN_SENIOR_PLAYERS: usize = 15;

/// What an AI club's ordinary squad planning keeps above the floor: one more
/// than the minimum in each group, and this many seniors in all. Three over
/// [`MIN_SENIOR_PLAYERS`] because a season's end can retire two or three of a
/// club's players at once, and planning only acts on the club's review day —
/// a margin of one would put clubs below the floor every summer.
pub(crate) const PLANNING_MARGIN_PER_GROUP: usize = 1;
pub(crate) const PLANNING_TARGET_SENIORS: usize = 18;

/// The floor for one position group, or 0 for a group with none.
pub(crate) fn group_floor(group: &Position) -> usize {
    MIN_PLAYERS_PER_GROUP
        .iter()
        .find(|(position, _)| position == group)
        .map(|(_, floor)| *floor)
        .unwrap_or(0)
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

/// One way a club can fall short of the floor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shortage {
    /// Fewer than the minimum in this position group.
    Group(Position),
    /// Fewer than [`MIN_SENIOR_PLAYERS`] in all, beyond what the groups need.
    Seniors,
}

/// Whether this player counts toward `team_id`'s floor: a senior player
/// registered to it. Injured players count, since they are still the club's to
/// field in a crisis. Players out on loan count for their borrower, where they
/// are registered. Academy players do not: they are what a short club promotes.
fn holds_up_the_floor(player: &Player, team_id: &str) -> bool {
    player.team_id.as_deref() == Some(team_id) && player.squad_role == SquadRole::Senior
}

/// Senior players registered to `team_id`, per group, in
/// [`MIN_PLAYERS_PER_GROUP`] order.
fn senior_counts(game: &Game, team_id: &str) -> [usize; 4] {
    let mut seniors = [0; 4];
    for player in &game.players {
        if holds_up_the_floor(player, team_id) {
            seniors[group_index(&player.position)] += 1;
        }
    }
    seniors
}

/// Every way this club is short of the floor, and by how many players. Group
/// shortages come first; [`Shortage::Seniors`] counts only the players still
/// needed once every group is at its minimum. Empty when the club is sound.
pub fn squad_shortfall(game: &Game, team_id: &str) -> Vec<(Shortage, usize)> {
    shortfall_of(senior_counts(game, team_id))
}

/// The shortfall for a club with `seniors` per group.
fn shortfall_of(seniors: [usize; 4]) -> Vec<(Shortage, usize)> {
    let mut shortfall: Vec<(Shortage, usize)> = MIN_PLAYERS_PER_GROUP
        .iter()
        .zip(seniors)
        .filter(|((_, floor), have)| have < floor)
        .map(|((group, floor), have)| (Shortage::Group(group.clone()), floor - have))
        .collect();
    let after_groups: usize =
        seniors.iter().sum::<usize>() + shortfall.iter().map(|(_, missing)| missing).sum::<usize>();
    if after_groups < MIN_SENIOR_PLAYERS {
        shortfall.push((Shortage::Seniors, MIN_SENIOR_PLAYERS - after_groups));
    }
    shortfall
}

/// Senior players per club and group, for every club, in one pass over the
/// world's players — the floor's count. A club with nobody is absent.
pub(crate) fn registered_by_club(game: &Game) -> HashMap<&str, [usize; 4]> {
    let mut by_club: HashMap<&str, [usize; 4]> = HashMap::new();
    for player in &game.players {
        if let Some(team_id) = player.team_id.as_deref()
            && holds_up_the_floor(player, team_id)
        {
            by_club.entry(team_id).or_default()[group_index(&player.position)] += 1;
        }
    }
    by_club
}

/// Whether a club with `seniors` per group could do without one senior of
/// `position`'s group and stay at the floor — in the group and in all. The one
/// statement of that rule: the refusals, the wage waiver and the AI market all
/// ask it.
pub(crate) fn can_spare_one(seniors: [usize; 4], position: &Position) -> bool {
    shortage_without_one(seniors, position).is_none()
}

/// What losing one senior of `position`'s group would leave the club short of,
/// the group first.
fn shortage_without_one(seniors: [usize; 4], position: &Position) -> Option<Shortage> {
    let group = group_index(position);
    if seniors[group] <= MIN_PLAYERS_PER_GROUP[group].1 {
        Some(Shortage::Group(position.to_group_position()))
    } else if seniors.iter().sum::<usize>() <= MIN_SENIOR_PLAYERS {
        Some(Shortage::Seniors)
    } else {
        None
    }
}

/// What this player's departure would leave his club short of, if anything.
///
/// Answers for the club he is registered to, which is the club that loses him:
/// the buyer or borrower only gains. A club already short cannot let him go
/// either — the floor is not a line a club may cross further. An academy
/// player holds up no floor, so his departure never does.
pub(crate) fn departure_would_leave_short(game: &Game, player_id: &str) -> Option<Shortage> {
    let player = game.players.iter().find(|player| player.id == player_id)?;
    let team_id = player.team_id.as_deref()?;
    if !holds_up_the_floor(player, team_id) {
        return None;
    }
    shortage_without_one(senior_counts(game, team_id), &player.position)
}

/// Whether `team_id` would be below the floor without this player: he is one
/// of its seniors now and might leave (a renewal), or he is not at the club yet
/// and it is short where he would play (a signing). An academy player of the
/// club holds up no floor, so the answer for him is no.
///
/// This is also the one case in which the board's wage policy yields — see
/// [`crate::contract_wage_policy::wage_policy_verdict`].
pub(crate) fn club_needs_him_for_the_floor(game: &Game, team_id: &str, player: &Player) -> bool {
    let mut seniors = senior_counts(game, team_id);
    if player.team_id.as_deref() == Some(team_id) {
        if player.squad_role != SquadRole::Senior {
            return false;
        }
    } else {
        // Count him in, then ask whether the club could spare him.
        seniors[group_index(&player.position)] += 1;
    }
    !can_spare_one(seniors, &player.position)
}

/// Seniors of this player's group registered to `team_id`, him excluded.
pub(crate) fn others_in_his_group(game: &Game, team_id: &str, player: &Player) -> usize {
    let group = player.position.to_group_position();
    game.players
        .iter()
        .filter(|other| other.id != player.id && holds_up_the_floor(other, team_id))
        .filter(|other| other.position.to_group_position() == group)
        .count()
}

/// Seniors registered to `team_id`, this player excluded.
pub(crate) fn other_seniors(game: &Game, team_id: &str, player: &Player) -> usize {
    game.players
        .iter()
        .filter(|other| other.id != player.id && holds_up_the_floor(other, team_id))
        .count()
}

/// [`departure_would_leave_short`] as the refusal a sale, loan or release
/// returns, naming what it would leave the club short of.
pub(crate) fn ensure_departure_keeps_floor(game: &Game, player_id: &str) -> Result<(), String> {
    match departure_would_leave_short(game, player_id) {
        Some(Shortage::Group(group)) => Err(crate::contracts::backend_text_with_param(
            WOULD_LEAVE_GROUP_SHORT_ERROR,
            "group",
            &position_group_key(&group),
        )),
        Some(Shortage::Seniors) => Err(crate::contracts::backend_text_with_param(
            WOULD_LEAVE_SENIORS_SHORT_ERROR,
            "floor",
            &MIN_SENIOR_PLAYERS.to_string(),
        )),
        None => Ok(()),
    }
}

/// The frontend key naming a position group, resolved there as a param value.
fn position_group_key(group: &Position) -> String {
    format!("common.positionGroups.{group:?}")
}

const WOULD_LEAVE_GROUP_SHORT_ERROR: &str = "be.error.squadFloor.wouldLeaveShort";
const WOULD_LEAVE_SENIORS_SHORT_ERROR: &str = "be.error.squadFloor.wouldLeaveSeniorsShort";

/// The daily check, run once every way a club can lose players has had its
/// turn: an AI club short of the floor is filled back toward it; the player's
/// own club is told what it is short of and left to act, because bringing in
/// players on a manager's behalf is his decision until a match cannot go ahead
/// without them ([`ready_for_kick_off`]).
pub(crate) fn keep_squads_at_the_floor(game: &mut Game) {
    for (team_id, shortfall) in clubs_below_the_floor(game) {
        if Some(&team_id) == game.manager.team_id.as_ref() {
            warn_user_club_is_short(game, &team_id, &shortfall);
        } else {
            let top_up = restore_minimum_squad(game, &team_id);
            report_unfilled_ai_club(&team_id, &top_up);
        }
    }
}

/// Bring a loaded or freshly built world up to the floor.
///
/// A save written before the floor was enforced, or a package whose author
/// left a club thin, can open with clubs already short. The same rule as the
/// daily check, with one difference: when the player's club has a match today,
/// the warning would arrive with no time to act on it, so it is left to the
/// kick-off gate and its own message instead. Returns whether anything
/// changed, so a loaded save knows it needs writing back.
pub fn repair_squads_on_load(game: &mut Game) -> bool {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let user_club_plays_today =
        game.manager.team_id.as_ref().is_some_and(|team_id| {
            crate::training::teams_playing_on(game, &today).contains(team_id)
        });
    // Only what changed the game counts: a save whose one short club is the
    // player's, already warned, must not be rewritten on every load.
    let mut changed = false;
    for (team_id, shortfall) in clubs_below_the_floor(game) {
        if Some(&team_id) == game.manager.team_id.as_ref() {
            if !user_club_plays_today {
                changed |= warn_user_club_is_short(game, &team_id, &shortfall);
            }
        } else {
            let top_up = restore_minimum_squad(game, &team_id);
            changed |= !top_up.brought_in().is_empty();
            report_unfilled_ai_club(&team_id, &top_up);
        }
    }
    changed
}

/// The last line, which normal play should never reach: a club about to kick
/// off without enough players is filled first, so every match is played by the
/// most complete sides the world allows, and the day always finishes.
///
/// The player's club is only topped up when it could not put a side out at all
/// — fewer than eleven seniors, or no senior goalkeeper — because until then
/// who to bring in is the manager's call, and the daily warnings have told
/// them. When it is, it is filled toward the floor, told who was brought in,
/// and told what nobody could fill. An AI club is topped up whenever it is
/// short, and the top-up is logged as the bug it is: ordinary squad planning
/// should have got there first.
pub(crate) fn ready_for_kick_off(game: &mut Game, team_id: &str) {
    let is_users_club = Some(team_id) == game.manager.team_id.as_deref();
    if is_users_club && can_put_a_side_out(game, team_id) {
        return;
    }
    let top_up = restore_minimum_squad(game, team_id);
    if is_users_club {
        if !top_up.brought_in().is_empty() {
            let message = squad_topped_up_message(game, team_id, &top_up.brought_in());
            crate::inbox::emit(game, message);
        }
        tell_user_club_it_cannot_be_filled(game, team_id, &top_up.unfilled);
    } else {
        if !top_up.brought_in().is_empty() {
            log::warn!(
                "[squad_floor] {team_id} reached kick-off short of the floor; brought in {}",
                top_up.brought_in().len()
            );
        }
        report_unfilled_ai_club(team_id, &top_up);
    }
}

/// An AI club nobody could fill is a world running out of players, and worth
/// an error in the log: the match still goes ahead with who is left.
fn report_unfilled_ai_club(team_id: &str, top_up: &TopUp) {
    if !top_up.unfilled.is_empty() {
        log::error!(
            "[squad_floor] {team_id} is short {:?} with no academy player or free agent to fill it",
            top_up.unfilled
        );
    }
}

/// Players a side puts on the pitch.
const PLAYERS_ON_THE_PITCH: usize = 11;

/// Whether the club has eleven seniors, a goalkeeper among them — counted as
/// the floor counts. Injured players count: the match builder fields them when
/// it must.
fn can_put_a_side_out(game: &Game, team_id: &str) -> bool {
    let seniors = senior_counts(game, team_id);
    seniors.iter().sum::<usize>() >= PLAYERS_ON_THE_PITCH
        && seniors[group_index(&Position::Goalkeeper)] > 0
}

/// Every club short of the floor, with its shortfall, in the order clubs are
/// stored so what a day does does not depend on hashing.
fn clubs_below_the_floor(game: &Game) -> Vec<(String, Vec<(Shortage, usize)>)> {
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

/// The message keys and params that describe one shortage to the manager.
/// `key` is the message family (`be.msg.squadBelowFloor`, ...); a group
/// shortage uses its `subject`/`body`, the overall one `totalSubject`/`totalBody`.
fn shortage_message_parts(
    key: &str,
    team_name: &str,
    shortage: &Shortage,
    have: usize,
) -> (String, String, HashMap<String, String>) {
    let mut params = HashMap::new();
    params.insert("team".to_string(), team_name.to_string());
    params.insert("have".to_string(), have.to_string());
    match shortage {
        Shortage::Group(group) => {
            params.insert("group".to_string(), position_group_key(group));
            params.insert("floor".to_string(), group_floor(group).to_string());
            (format!("{key}.subject"), format!("{key}.body"), params)
        }
        Shortage::Seniors => {
            params.insert("floor".to_string(), MIN_SENIOR_PLAYERS.to_string());
            (
                format!("{key}.totalSubject"),
                format!("{key}.totalBody"),
                params,
            )
        }
    }
}

fn shortage_message(
    id: String,
    date: &str,
    subject_key: &str,
    body_key: &str,
    params: HashMap<String, String>,
) -> InboxMessage {
    InboxMessage::new(
        id,
        String::new(),
        String::new(),
        String::new(),
        date.to_string(),
    )
    .with_category(MessageCategory::Contract)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("")
    .with_i18n(subject_key, body_key, params)
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

/// How many seniors the club has where it is short: in the group, or in all.
fn have_for(game: &Game, team_id: &str, shortage: &Shortage) -> usize {
    let seniors = senior_counts(game, team_id);
    match shortage {
        Shortage::Group(group) => seniors[group_index(group)],
        Shortage::Seniors => seniors.iter().sum(),
    }
}

/// One message per shortage. Keyed on the season, the shortage and how many are
/// left, so the player hears once when the club goes short and again if it gets
/// worse — not every day it stays that way. Returns whether any was new.
fn warn_user_club_is_short(
    game: &mut Game,
    team_id: &str,
    shortfall: &[(Shortage, usize)],
) -> bool {
    let mut sent = false;
    let season = crate::inbox::recurrence_season(game);
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let team_name = game.team_name_or_id(team_id);
    for (shortage, _) in shortfall {
        let have = have_for(game, team_id, shortage);
        let (subject, body, params) =
            shortage_message_parts("be.msg.squadBelowFloor", &team_name, shortage, have);
        let id = format!("squad_below_floor_{team_id}_{season}_{shortage:?}_{have}");
        let message = shortage_message(id, &date, &subject, &body, params);
        sent |= crate::inbox::emit(game, message);
    }
    sent
}

/// One message per shortage nobody could fill, for the player's club. Keyed on
/// the day and the shortage: it is news every match day it stays true.
fn tell_user_club_it_cannot_be_filled(
    game: &mut Game,
    team_id: &str,
    unfilled: &[(Shortage, usize)],
) {
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let team_name = game.team_name_or_id(team_id);
    for (shortage, _) in unfilled {
        let have = have_for(game, team_id, shortage);
        let (subject, body, params) =
            shortage_message_parts("be.msg.squadCannotBeFilled", &team_name, shortage, have);
        let id = format!("squad_cannot_be_filled_{team_id}_{date}_{shortage:?}");
        let message = shortage_message(id, &date, &subject, &body, params);
        crate::inbox::emit(game, message);
    }
}

fn squad_topped_up_message(game: &Game, team_id: &str, brought_in: &[String]) -> InboxMessage {
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();
    let names = brought_in
        .iter()
        .filter_map(|id| game.players.iter().find(|player| &player.id == id))
        .map(|player| player.full_name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let mut params = HashMap::new();
    params.insert("team".to_string(), game.team_name_or_id(team_id));
    params.insert("players".to_string(), names);
    shortage_message(
        format!("squad_topped_up_{team_id}_{date}"),
        &date,
        "be.msg.squadToppedUp.subject",
        "be.msg.squadToppedUp.body",
        params,
    )
}

/// What a top-up did: who it promoted from the club's academy, who it signed
/// from the free-agent market, and what it could not fill.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TopUp {
    pub(crate) promoted: Vec<String>,
    pub(crate) signed: Vec<String>,
    pub(crate) unfilled: Vec<(Shortage, usize)>,
}

impl TopUp {
    /// Everyone brought into the senior squad, promotions first.
    pub(crate) fn brought_in(&self) -> Vec<String> {
        self.promoted.iter().chain(&self.signed).cloned().collect()
    }
}

/// One emergency top-up, for the game's runtime record. Not saved: it is a
/// diagnostic of the session being played — a club reaching the emergency
/// means ordinary squad planning did not get there first — for tests and the
/// season harness to read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquadFloorTopUp {
    pub date: String,
    pub team_id: String,
    pub short: Vec<(Shortage, usize)>,
    pub unfilled: Vec<(Shortage, usize)>,
}

/// Bring a club back to the floor from the players who exist, and say what was
/// done. A club at the floor is left alone.
///
/// Each missing player is looked for in the club's own academy first, and then
/// on the free-agent market ([`bring_in_one`]). Nobody is created: when no
/// source has anyone, the gap is returned in [`TopUp::unfilled`] for the
/// caller to report. Every top-up that found the club short is added to
/// [`Game::squad_floor_top_ups`].
pub(crate) fn restore_minimum_squad(game: &mut Game, team_id: &str) -> TopUp {
    let shortfall = squad_shortfall(game, team_id);
    if shortfall.is_empty() {
        return TopUp::default();
    }
    let Some(team) = game.teams.iter().find(|team| team.id == team_id).cloned() else {
        log::error!("[squad_floor] cannot restore the squad of unknown club {team_id}");
        return TopUp::default();
    };

    let mut top_up = TopUp::default();
    for (shortage, missing) in &shortfall {
        for filled in 0..*missing {
            let groups = match shortage {
                Shortage::Group(group) => vec![group.clone()],
                Shortage::Seniors => groups_thinnest_first(game, team_id),
            };
            let brought = groups
                .iter()
                .find_map(|group| bring_in_one(game, &team, group));
            match brought {
                Some(BroughtIn::Promoted(id)) => top_up.promoted.push(id),
                Some(BroughtIn::Signed(id)) => top_up.signed.push(id),
                None => {
                    top_up.unfilled.push((shortage.clone(), missing - filled));
                    break;
                }
            }
        }
    }
    game.squad_floor_top_ups.push(SquadFloorTopUp {
        date: game.clock.current_date.format("%Y-%m-%d").to_string(),
        team_id: team_id.to_string(),
        short: shortfall,
        unfilled: top_up.unfilled.clone(),
    });
    top_up
}

/// The club's position groups, the one with least to spare over its minimum
/// first — where an extra senior helps most. Ties keep [`MIN_PLAYERS_PER_GROUP`]
/// order.
pub(crate) fn groups_thinnest_first(game: &Game, team_id: &str) -> Vec<Position> {
    let seniors = senior_counts(game, team_id);
    let mut groups: Vec<(usize, Position)> = MIN_PLAYERS_PER_GROUP
        .iter()
        .enumerate()
        .map(|(index, (group, floor))| (seniors[index].saturating_sub(*floor), group.clone()))
        .collect();
    groups.sort_by_key(|(spare, _)| *spare);
    groups.into_iter().map(|(_, group)| group).collect()
}

/// How one player came into a club's senior squad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BroughtIn {
    Promoted(String),
    Signed(String),
}

/// Bring one senior of `group` into `team`'s squad from where the rules allow,
/// or `None` when nowhere has anyone.
///
/// The club's own academy comes first: promotion costs no new wages, and the
/// player is already the club's. Then the free-agent market, best first, the
/// first one the board lets the club pay ([`wage_policy_verdict`] — which
/// always yields for a club below the floor). The shared source for the
/// emergency top-up and an AI club's ordinary squad planning.
///
/// [`wage_policy_verdict`]: crate::contract_wage_policy::wage_policy_verdict
pub(crate) fn bring_in_one(game: &mut Game, team: &Team, group: &Position) -> Option<BroughtIn> {
    if let Some(index) = best_academy_player(game, &team.id, group) {
        game.players[index].squad_role = SquadRole::Senior;
        return Some(BroughtIn::Promoted(game.players[index].id.clone()));
    }

    let current_date = game.clock.current_date.date_naive();
    for index in free_agents_best_first(game, group) {
        let wage = crate::contracts::expected_wage(&game.players[index], team, current_date);
        let permitted = crate::contract_wage_policy::wage_policy_verdict(
            game,
            team,
            &game.players[index],
            wage,
        )
        .permits();
        if !permitted {
            continue;
        }
        let years = crate::contracts::expected_contract_years(&game.players[index], current_date);
        match crate::contracts::sign_free_agent(game, index, team, wage, years, current_date) {
            Ok(()) => return Some(BroughtIn::Signed(game.players[index].id.clone())),
            Err(error) => log::error!(
                "[squad_floor] could not sign {} for {}: {error}",
                game.players[index].id,
                team.id
            ),
        }
    }
    None
}

/// The club's academy player of this group it would promote first: the best
/// rated, then the oldest, then the lowest id, so the answer does not depend
/// on the order players are stored in.
fn best_academy_player(game: &Game, team_id: &str, group: &Position) -> Option<usize> {
    game.players
        .iter()
        .enumerate()
        .filter(|(_, player)| player.team_id.as_deref() == Some(team_id))
        .filter(|(_, player)| player.squad_role == SquadRole::Youth)
        .filter(|(_, player)| player.position.to_group_position() == *group)
        .max_by(|(_, left), (_, right)| {
            left.ovr
                .cmp(&right.ovr)
                .then_with(|| right.date_of_birth.cmp(&left.date_of_birth))
                .then_with(|| right.id.cmp(&left.id))
        })
        .map(|(index, _)| index)
}

/// Free agents of this group in the order a club would try to sign them: not
/// retired, fit before injured, then the highest rating, then the lowest id.
fn free_agents_best_first(game: &Game, group: &Position) -> Vec<usize> {
    let mut candidates: Vec<usize> = game
        .players
        .iter()
        .enumerate()
        .filter(|(_, player)| player.team_id.is_none() && !player.retired)
        .filter(|(_, player)| player.position.to_group_position() == *group)
        .map(|(index, _)| index)
        .collect();
    candidates.sort_by(|left, right| {
        let (left, right) = (&game.players[*left], &game.players[*right]);
        right
            .injury
            .is_none()
            .cmp(&left.injury.is_none())
            .then(right.ovr.cmp(&left.ovr))
            .then_with(|| left.id.cmp(&right.id))
    });
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::player::{Player, PlayerAttributes};
    use domain::team::Team;

    const GK: Shortage = Shortage::Group(Position::Goalkeeper);
    const DEF: Shortage = Shortage::Group(Position::Defender);
    const FWD: Shortage = Shortage::Group(Position::Forward);

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

    /// A club with exactly `per_group` seniors in each group, `[GK, DEF, MID, FWD]`.
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

    /// The same squad, managed by the player.
    fn users_club_with(per_group: [usize; 4]) -> Game {
        let mut game = club_with(per_group);
        game.manager.hire("club".to_string());
        game
    }

    const SOUND: [usize; 4] = [2, 5, 5, 3];

    fn free_agent(id: &str, position: Position, ovr: u8) -> Player {
        let mut agent = player(id, None, position);
        agent.ovr = ovr;
        agent.contract_end = None;
        agent.wage = 0;
        agent
    }

    fn academy_player(id: &str, position: Position) -> Player {
        let mut youngster = player(id, Some("club"), position);
        youngster.squad_role = SquadRole::Youth;
        youngster.date_of_birth = "2009-01-01".to_string();
        youngster
    }

    /// Free agents enough to make any club of nobody sound.
    fn free_agent_pool(game: &mut Game) {
        for ((group, _), count) in MIN_PLAYERS_PER_GROUP.iter().zip(SOUND) {
            for i in 0..count {
                game.players
                    .push(free_agent(&format!("fa_{group:?}{i}"), group.clone(), 60));
            }
        }
    }

    fn team_of<'a>(game: &'a Game, id: &str) -> Option<&'a str> {
        game.players
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.team_id.as_deref())
    }

    fn role_of(game: &Game, id: &str) -> SquadRole {
        game.players.iter().find(|p| p.id == id).unwrap().squad_role
    }

    fn body_keys(game: &Game) -> Vec<&str> {
        game.messages
            .iter()
            .map(|message| message.body_key.as_deref().unwrap_or(""))
            .collect()
    }

    // --- The floor's count ---------------------------------------------------

    #[test]
    fn a_sound_club_is_short_of_nothing() {
        assert!(squad_shortfall(&club_with(SOUND), "club").is_empty());
    }

    #[test]
    fn a_club_is_short_by_group_and_by_how_many() {
        let game = club_with([0, 3, 7, 5]);
        assert_eq!(squad_shortfall(&game, "club"), vec![(GK, 2), (DEF, 1)]);
    }

    /// Given 2/4/6/2 — every group at its minimum, fourteen in all — the club
    /// is one senior short overall and in no group.
    #[test]
    fn a_club_of_fourteen_seniors_is_short_overall() {
        let game = club_with([2, 4, 6, 2]);
        assert_eq!(squad_shortfall(&game, "club"), vec![(Shortage::Seniors, 1)]);
    }

    #[test]
    fn a_club_with_nobody_is_short_everywhere() {
        let game = club_with([0, 0, 0, 0]);
        assert_eq!(
            squad_shortfall(&game, "club"),
            vec![
                (GK, 2),
                (DEF, 4),
                (Shortage::Group(Position::Midfielder), 4),
                (FWD, 2),
                (Shortage::Seniors, 3),
            ]
        );
    }

    /// A player out on loan is registered to his borrower, and an injured one
    /// is still the club's to field. Granular positions count in their group.
    #[test]
    fn injured_players_count_and_loaned_out_players_do_not() {
        let mut game = club_with([2, 5, 5, 2]);
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

    /// Given a sound club whose second keeper is an academy player, it is one
    /// keeper short: the academy does not hold up the floor.
    #[test]
    fn youth_do_not_count_toward_the_floor() {
        let mut game = club_with([1, 5, 5, 4]);
        game.players
            .push(academy_player("academy_keeper", Position::Goalkeeper));
        assert_eq!(squad_shortfall(&game, "club"), vec![(GK, 1)]);
    }

    // --- Departures ------------------------------------------------------------

    #[test]
    fn a_player_cannot_leave_a_group_that_is_at_its_minimum() {
        let game = club_with([2, 6, 5, 3]);
        assert_eq!(departure_would_leave_short(&game, "Goalkeeper0"), Some(GK));
        assert_eq!(departure_would_leave_short(&game, "Defender0"), None);
        assert!(ensure_departure_keeps_floor(&game, "Defender0").is_ok());
        assert_eq!(
            ensure_departure_keeps_floor(&game, "Goalkeeper1"),
            Err(
                "be.error.squadFloor.wouldLeaveShort?group=common.positionGroups.Goalkeeper"
                    .to_string()
            )
        );
    }

    /// Given a club of exactly fifteen, every group above its minimum, no
    /// senior can leave: the club would be fourteen.
    #[test]
    fn a_player_cannot_leave_a_club_of_fifteen() {
        let game = club_with([3, 5, 4, 3]);
        assert_eq!(
            departure_would_leave_short(&game, "Defender0"),
            Some(Shortage::Seniors)
        );
        assert_eq!(
            ensure_departure_keeps_floor(&game, "Defender0"),
            Err("be.error.squadFloor.wouldLeaveSeniorsShort?floor=15".to_string())
        );
    }

    #[test]
    fn a_youth_player_can_always_be_sold() {
        let mut game = club_with(SOUND);
        game.players
            .push(academy_player("academy_forward", Position::Forward));
        assert_eq!(departure_would_leave_short(&game, "academy_forward"), None);
    }

    /// A player who is not at a club leaves nobody short.
    #[test]
    fn a_free_agent_leaves_nobody_short() {
        let mut game = club_with(SOUND);
        game.players
            .push(free_agent("agent", Position::Goalkeeper, 70));
        assert_eq!(departure_would_leave_short(&game, "agent"), None);
    }

    // --- The wage waiver's question --------------------------------------------

    /// Given a club at the floor in seniors, a senior keeper is needed for it
    /// and an academy player never is — so only the senior can have the board's
    /// wage policy waived for him.
    #[test]
    fn a_youth_renewal_gets_no_floor_waiver() {
        let mut game = club_with(SOUND);
        game.players
            .push(academy_player("academy_keeper", Position::Goalkeeper));
        let keeper = game.players[0].clone();
        let youngster = game.players.last().unwrap().clone();
        assert!(club_needs_him_for_the_floor(&game, "club", &keeper));
        assert!(!club_needs_him_for_the_floor(&game, "club", &youngster));
    }

    /// A free agent is needed when the club is short where he plays, and not
    /// when it is sound.
    #[test]
    fn a_free_agent_is_needed_only_where_the_club_is_short() {
        let game = club_with([1, 5, 5, 4]);
        assert!(club_needs_him_for_the_floor(
            &game,
            "club",
            &free_agent("keeper", Position::Goalkeeper, 60)
        ));
        assert!(!club_needs_him_for_the_floor(
            &game,
            "club",
            &free_agent("forward", Position::Forward, 60)
        ));
    }

    // --- The emergency top-up ----------------------------------------------------

    /// Given a club one keeper short with an academy keeper and a free-agent
    /// keeper, the academy keeper is promoted and the free agent left alone.
    #[test]
    fn a_short_club_promotes_its_own_youth_first() {
        let mut game = club_with([1, 5, 5, 4]);
        game.players
            .push(academy_player("academy_keeper", Position::Goalkeeper));
        game.players
            .push(free_agent("free_keeper", Position::Goalkeeper, 90));

        let top_up = restore_minimum_squad(&mut game, "club");

        assert_eq!(top_up.promoted, vec!["academy_keeper".to_string()]);
        assert!(top_up.signed.is_empty());
        assert_eq!(role_of(&game, "academy_keeper"), SquadRole::Senior);
        assert_eq!(team_of(&game, "free_keeper"), None);
        assert!(squad_shortfall(&game, "club").is_empty());
    }

    /// Given a club one keeper short, no academy keeper, several free agents
    /// and no money at all, the best keeper is signed on a real contract: the
    /// club pays his wages and its balance may go further negative.
    #[test]
    fn a_short_club_with_no_youth_signs_a_free_agent_even_in_debt() {
        let mut game = club_with([1, 5, 5, 4]);
        game.teams[0].finance = -1_000_000;
        game.players
            .push(free_agent("weak_keeper", Position::Goalkeeper, 55));
        game.players
            .push(free_agent("good_keeper", Position::Goalkeeper, 70));
        game.players
            .push(free_agent("forward", Position::Forward, 80));

        let top_up = restore_minimum_squad(&mut game, "club");

        assert_eq!(top_up.signed, vec!["good_keeper".to_string()]);
        let keeper = game.players.iter().find(|p| p.id == "good_keeper").unwrap();
        assert_eq!(keeper.team_id.as_deref(), Some("club"));
        assert!(keeper.contract_end.is_some(), "signed without a contract");
        assert!(keeper.wage > 0, "signed on no wage");
        assert_eq!(team_of(&game, "forward"), None);
        assert_eq!(
            game.teams[0].finance, -1_000_000,
            "money appeared from nowhere"
        );
    }

    #[test]
    fn a_retired_player_is_never_signed() {
        let mut game = club_with([1, 5, 5, 4]);
        let mut retired = free_agent("retired_keeper", Position::Goalkeeper, 90);
        retired.retired = true;
        game.players.push(retired);

        let top_up = restore_minimum_squad(&mut game, "club");

        assert!(top_up.brought_in().is_empty());
        assert_eq!(top_up.unfilled, vec![(GK, 1)]);
        assert_eq!(team_of(&game, "retired_keeper"), None);
    }

    /// Given a club of nobody, no academy and no free agents, nobody is
    /// signed, promoted or created, and the whole gap is returned.
    #[test]
    fn a_short_club_never_gets_a_generated_player() {
        let mut game = club_with([0, 0, 0, 0]);
        let players_before = game.players.len();

        let top_up = restore_minimum_squad(&mut game, "club");

        assert_eq!(game.players.len(), players_before, "a player was created");
        assert!(top_up.brought_in().is_empty());
        assert_eq!(top_up.unfilled, squad_shortfall(&game, "club"));
    }

    /// Given fourteen seniors at 2/4/6/2 and academy players in defence,
    /// midfield and attack, one is promoted from the thinnest group with an
    /// academy player — defence, before midfield's two to spare.
    #[test]
    fn a_club_short_overall_is_filled_from_the_thinnest_group() {
        let mut game = club_with([2, 4, 6, 2]);
        game.players
            .push(academy_player("academy_mid", Position::Midfielder));
        game.players
            .push(academy_player("academy_def", Position::Defender));

        let top_up = restore_minimum_squad(&mut game, "club");

        assert_eq!(top_up.promoted, vec!["academy_def".to_string()]);
        assert!(squad_shortfall(&game, "club").is_empty());
    }

    #[test]
    fn a_sound_club_brings_in_nobody() {
        let mut game = club_with(SOUND);
        game.players
            .push(free_agent("keeper", Position::Goalkeeper, 90));
        assert_eq!(restore_minimum_squad(&mut game, "club"), TopUp::default());
        assert!(game.squad_floor_top_ups.is_empty());
    }

    #[test]
    fn a_top_up_is_recorded() {
        let mut game = club_with([1, 5, 5, 4]);
        restore_minimum_squad(&mut game, "club");
        assert_eq!(
            game.squad_floor_top_ups,
            vec![SquadFloorTopUp {
                date: "2026-08-01".to_string(),
                team_id: "club".to_string(),
                short: vec![(GK, 1)],
                unfilled: vec![(GK, 1)],
            }]
        );
    }

    // --- The daily check -------------------------------------------------------

    #[test]
    fn the_daily_check_tops_up_an_ai_club_and_only_warns_the_player() {
        let mut game = users_club_with([1, 5, 5, 3]);
        game.teams.push(Team::new(
            "rival".to_string(),
            "Rival".to_string(),
            "RIV".to_string(),
            "England".to_string(),
            "Leeds".to_string(),
            "Park".to_string(),
            10_000,
        ));
        free_agent_pool(&mut game);
        game.players
            .push(free_agent("spare_keeper", Position::Goalkeeper, 50));

        keep_squads_at_the_floor(&mut game);

        assert!(
            squad_shortfall(&game, "rival").is_empty(),
            "the AI club was left short"
        );
        assert_eq!(
            squad_shortfall(&game, "club"),
            vec![(GK, 1)],
            "players were brought in for the player's club without asking"
        );
        assert_eq!(body_keys(&game), vec!["be.msg.squadBelowFloor.body"]);
        let warning = &game.messages[0].i18n_params;
        assert_eq!(
            warning.get("group").map(String::as_str),
            Some("common.positionGroups.Goalkeeper")
        );
        assert_eq!(warning.get("have").map(String::as_str), Some("1"));
        assert_eq!(warning.get("floor").map(String::as_str), Some("2"));
    }

    /// Given the player's club at fourteen seniors, every group at its minimum,
    /// the warning is the overall one, naming fourteen of fifteen.
    #[test]
    fn the_player_is_warned_when_the_squad_is_below_fifteen() {
        let mut game = users_club_with([2, 4, 6, 2]);

        keep_squads_at_the_floor(&mut game);

        assert_eq!(body_keys(&game), vec!["be.msg.squadBelowFloor.totalBody"]);
        let warning = &game.messages[0].i18n_params;
        assert_eq!(warning.get("have").map(String::as_str), Some("14"));
        assert_eq!(warning.get("floor").map(String::as_str), Some("15"));
    }

    /// A shortfall that lasts is one warning, not one a day; the same shortage
    /// getting worse is news again.
    #[test]
    fn the_player_is_warned_once_until_the_shortfall_gets_worse() {
        let mut game = users_club_with([1, 5, 5, 4]);
        keep_squads_at_the_floor(&mut game);
        game.clock.advance_days(1);
        keep_squads_at_the_floor(&mut game);
        assert_eq!(game.messages.len(), 1);

        game.messages.clear();
        game.players.retain(|player| player.id != "Goalkeeper0");
        keep_squads_at_the_floor(&mut game);
        assert!(
            game.messages
                .iter()
                .any(|m| m.i18n_params.get("have").map(String::as_str) == Some("0")),
            "a worse shortfall went unreported"
        );
    }

    // --- Kick-off -----------------------------------------------------------------

    /// Given the player's club with no keeper, one academy keeper and one
    /// free-agent keeper, both come in — the academy keeper first — and the
    /// message names both.
    #[test]
    fn the_players_kick_off_top_up_names_who_was_brought_in() {
        let mut game = users_club_with([0, 5, 5, 5]);
        game.players
            .push(academy_player("academy_keeper", Position::Goalkeeper));
        game.players
            .push(free_agent("free_keeper", Position::Goalkeeper, 70));

        ready_for_kick_off(&mut game, "club");

        assert!(squad_shortfall(&game, "club").is_empty());
        assert_eq!(body_keys(&game), vec!["be.msg.squadToppedUp.body"]);
        assert_eq!(
            game.messages[0]
                .i18n_params
                .get("players")
                .map(String::as_str),
            Some("academy_keeper, free_keeper")
        );
    }

    /// Eleven seniors and a keeper can play. Short of the floor is a reason to
    /// warn, not to bring someone in on the manager's behalf.
    #[test]
    fn at_kick_off_a_players_club_that_can_field_a_side_is_left_to_its_manager() {
        let mut game = users_club_with([1, 5, 5, 3]);
        game.players
            .push(free_agent("keeper", Position::Goalkeeper, 70));

        ready_for_kick_off(&mut game, "club");

        assert!(game.messages.is_empty());
        assert_eq!(team_of(&game, "keeper"), None);
    }

    /// Ten seniors, one short of a side, with a market to fill from.
    #[test]
    fn at_kick_off_a_players_club_of_ten_is_topped_up() {
        let mut game = users_club_with([1, 4, 4, 1]);
        free_agent_pool(&mut game);

        ready_for_kick_off(&mut game, "club");

        assert!(squad_shortfall(&game, "club").is_empty());
        assert_eq!(body_keys(&game), vec!["be.msg.squadToppedUp.body"]);
    }

    /// Given the player's club with no keeper, no academy keeper and no free
    /// agent anywhere, kick-off says, translated, that nobody could be found.
    #[test]
    fn the_players_club_is_told_when_it_cannot_be_filled() {
        let mut game = users_club_with([0, 5, 5, 5]);

        ready_for_kick_off(&mut game, "club");

        assert_eq!(body_keys(&game), vec!["be.msg.squadCannotBeFilled.body"]);
        assert_eq!(
            game.messages[0]
                .i18n_params
                .get("group")
                .map(String::as_str),
            Some("common.positionGroups.Goalkeeper")
        );
    }

    #[test]
    fn at_kick_off_an_ai_club_is_topped_up_without_a_message() {
        let mut game = club_with([0, 0, 0, 0]);
        free_agent_pool(&mut game);
        ready_for_kick_off(&mut game, "club");
        assert!(squad_shortfall(&game, "club").is_empty());
        assert!(game.messages.is_empty());
    }

    #[test]
    fn at_kick_off_a_sound_club_is_left_alone() {
        let mut game = users_club_with(SOUND);
        game.players
            .push(free_agent("keeper", Position::Goalkeeper, 70));
        ready_for_kick_off(&mut game, "club");
        assert!(game.messages.is_empty());
        assert_eq!(team_of(&game, "keeper"), None);
    }

    // --- Load repair -------------------------------------------------------------

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
        let mut quiet_day = users_club_with([1, 5, 5, 4]);
        with_fixture_on(&mut quiet_day, "2026-08-08");
        assert!(repair_squads_on_load(&mut quiet_day));
        assert_eq!(quiet_day.messages.len(), 1);
        assert!(!squad_shortfall(&quiet_day, "club").is_empty());

        let mut matchday = users_club_with([1, 5, 5, 4]);
        with_fixture_on(&mut matchday, "2026-08-01");
        assert!(!repair_squads_on_load(&mut matchday), "nothing was changed");
        assert!(matchday.messages.is_empty());
        assert!(!squad_shortfall(&matchday, "club").is_empty());
    }

    /// Given a save whose only short club is the player's, already warned,
    /// when it is loaded again, the repair reports no change — so the save is
    /// not rewritten on every load.
    #[test]
    fn load_repair_does_not_rewrite_a_save_it_did_not_change() {
        let mut game = users_club_with([1, 5, 5, 4]);
        assert!(repair_squads_on_load(&mut game), "the first load warns");
        assert!(
            !repair_squads_on_load(&mut game),
            "a second load changed nothing"
        );
    }

    /// Given an AI club with no keeper and nobody anywhere to bring in, loading
    /// changes nothing — so the save is not rewritten for a gap it cannot fill.
    #[test]
    fn load_repair_reports_no_change_when_nobody_could_be_brought_in() {
        let mut game = club_with([0, 5, 5, 5]);
        assert!(!repair_squads_on_load(&mut game));
    }

    #[test]
    fn load_repair_tops_up_ai_clubs_and_reports_nothing_to_do_when_all_are_sound() {
        let mut game = club_with([0, 5, 5, 5]);
        free_agent_pool(&mut game);
        assert!(repair_squads_on_load(&mut game));
        assert!(squad_shortfall(&game, "club").is_empty());
        assert!(
            !repair_squads_on_load(&mut game),
            "a sound world was rewritten"
        );
    }
}
