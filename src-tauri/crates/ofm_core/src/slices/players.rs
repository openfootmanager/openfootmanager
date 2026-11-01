use super::search_text::{contains_folded, fold_needle};
use crate::game::Game;
use crate::player_knowledge::{
    KnowledgeView, PlayerVisibility, VisibleWage, compare_visible_ovr, passes_ovr_filter,
};
use crate::transfers::{
    can_afford_fee, has_pending_transfer_registration, player_has_active_or_pending_loan,
};
use chrono::NaiveDate;
use domain::age::age_on_or_unknown;
use domain::player::{Player, Position};
use domain::team::Team;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayerSortKey {
    Name,
    Position,
    Age,
    Ovr,
    Value,
    Team,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayerStatusFilter {
    All,
    Transfer,
    Loan,
    /// Not under contract to any club.
    FreeAgent,
    /// Anyone a club can approach: listed for transfer or for loan, or without a club.
    Available,
}

/// Which players a list is about, before any other filter narrows it.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayerScope {
    /// Everyone in the world.
    #[default]
    All,
    /// Players the user's club could still approach: not retired, not on loan, not already agreed
    /// to move, and not in the user's own club.
    Market,
    /// Players a scout can be sent to watch: not retired and not in the user's own club. One
    /// already being scouted stays listed, so the screen can say so; `send_scout` refuses a second.
    Scoutable,
    /// The user's own players put up for transfer or loan and still free to move.
    UserListed,
    /// Players the user's club has dealings over: offers on its own players, and offers its club
    /// has made for others'.
    WithOffers,
}

/// One position test: an exact position, or every position in a group (a defender of any kind).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PositionFilter {
    Group(Position),
    Exact(Position),
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayersPageQuery {
    pub search: Option<String>,
    pub position: Option<Position>,
    /// Exact positions and group shorthands; a player matching any of them passes. Empty means no
    /// position filter beyond `position`.
    #[serde(default)]
    pub positions: Vec<PositionFilter>,
    #[serde(default)]
    pub age_min: Option<i32>,
    #[serde(default)]
    pub age_max: Option<i32>,
    #[serde(default)]
    pub ovr_min: Option<u8>,
    #[serde(default)]
    pub ovr_max: Option<u8>,
    #[serde(default)]
    pub value_min: Option<u64>,
    #[serde(default)]
    pub value_max: Option<u64>,
    /// Only players the user's club can pay for out of cash and transfer budget.
    #[serde(default)]
    pub in_budget: bool,
    #[serde(default)]
    pub scope: PlayerScope,
    pub team_id: Option<String>,
    pub status: PlayerStatusFilter,
    pub sort_key: PlayerSortKey,
    pub sort_asc: bool,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct PlayerSummary {
    pub id: String,
    pub full_name: String,
    pub match_name: String,
    pub date_of_birth: String,
    /// Whole years on the game's current date.
    pub age: i32,
    /// Weekly wage as the club may see it: exact for its own player, about the figure for one its
    /// scout reported on, hidden otherwise (`player_knowledge`).
    pub wage: VisibleWage,
    pub nationality: String,
    pub position: Position,
    pub natural_position: Position,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    pub market_value: u64,
    /// His rating when the club knows it (its own player, a division rival); none otherwise.
    pub ovr: Option<u8>,
    pub transfer_listed: bool,
    pub loan_listed: bool,
    pub injured: bool,
    pub retired: bool,
    pub media: PlayerSummaryMedia,
}

/// Artwork paths for a row in the players table.
///
/// Kept separate from `domain::player::PlayerMedia` so the wire shape belongs to
/// this projection: the domain type skips `face` when it is empty, which would
/// leave the key absent rather than null. Mirrors `TeamCardMedia` in the teams
/// slice.
#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct PlayerSummaryMedia {
    pub face: Option<String>,
}

/// How many players each status chip would show, with every filter but status as the query has it.
#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct StatusCounts {
    pub all: usize,
    pub transfer: usize,
    pub loan: usize,
    pub free_agent: usize,
    pub available: usize,
}

impl StatusCounts {
    fn of(rows: &[Row]) -> Self {
        let count_of = |status| {
            rows.iter()
                .filter(|(player, _)| matches_status(player, status))
                .count()
        };
        Self {
            all: count_of(PlayerStatusFilter::All),
            transfer: count_of(PlayerStatusFilter::Transfer),
            loan: count_of(PlayerStatusFilter::Loan),
            free_agent: count_of(PlayerStatusFilter::FreeAgent),
            available: count_of(PlayerStatusFilter::Available),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PlayersPage {
    pub items: Vec<PlayerSummary>,
    pub status_counts: StatusCounts,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

type TeamLookup<'a> = HashMap<&'a str, &'a str>;

/// A player in the list, with what the club may see of him, worked out once per query.
type Row<'a> = (&'a Player, PlayerVisibility);

pub fn query_page(game: &Game, query: &PlayersPageQuery) -> PlayersPage {
    let teams: TeamLookup = game
        .teams
        .iter()
        .map(|team| (team.id.as_str(), team.name.as_str()))
        .collect();

    let needle = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(fold_needle);

    let context = ListContext::new(game);
    let view = KnowledgeView::of(game);
    // The rating filter reads what the club may see: an unknown rating fails any set bound
    // (maintainer, 29 Oct), so the filter tells nothing the club does not know.
    let candidates: Vec<Row> = game
        .players
        .iter()
        .filter(|player| matches_all_but_status(player, query, needle.as_deref(), &context, &teams))
        .map(|player| (player, view.of_player(player)))
        .filter(|(_, seen)| passes_ovr_filter(seen.ovr, query.ovr_min, query.ovr_max))
        .collect();
    let status_counts = StatusCounts::of(&candidates);
    let mut matches: Vec<Row> = candidates
        .into_iter()
        .filter(|(player, _)| matches_status(player, query.status))
        .collect();

    sort_matches(&mut matches, query, &teams, context.today);

    let total = matches.len();
    let page = query.page.max(1);
    let start = (page - 1).saturating_mul(query.page_size);
    let end = start.saturating_add(query.page_size).min(total);
    let items = matches
        .get(start..end)
        .unwrap_or(&[])
        .iter()
        .map(|(player, seen)| project_summary(player, seen, &teams, context.today))
        .collect();

    PlayersPage {
        items,
        status_counts,
        total,
        page,
        page_size: query.page_size,
    }
}

/// What a filter needs to know about the game besides the player in front of it.
struct ListContext<'a> {
    today: NaiveDate,
    user_team: Option<&'a Team>,
}

impl<'a> ListContext<'a> {
    fn new(game: &'a Game) -> Self {
        let user_team = game
            .manager
            .team_id
            .as_deref()
            .and_then(|id| game.teams.iter().find(|team| team.id == id));
        Self {
            today: game.clock.current_date.date_naive(),
            user_team,
        }
    }

    fn is_users_own(&self, player: &Player) -> bool {
        self.user_team
            .is_some_and(|team| player.team_id.as_deref() == Some(team.id.as_str()))
    }
}

fn matches_scope(player: &Player, scope: PlayerScope, context: &ListContext) -> bool {
    match scope {
        PlayerScope::All => true,
        PlayerScope::Market => {
            !player.retired
                && !context.is_users_own(player)
                && !player_has_active_or_pending_loan(player)
                && !has_pending_transfer_registration(player)
        }
        PlayerScope::Scoutable => !player.retired && !context.is_users_own(player),
        PlayerScope::UserListed => {
            context.is_users_own(player)
                && (player.transfer_listed || player.loan_listed)
                && !player_has_active_or_pending_loan(player)
                && !has_pending_transfer_registration(player)
        }
        PlayerScope::WithOffers => context
            .user_team
            .is_some_and(|team| has_offer_involving(player, &team.id)),
    }
}

/// An offer concerns a club when it made it, when it owns the player the transfer offer is for, or
/// when it is the parent club of a loan offer.
fn has_offer_involving(player: &Player, club_id: &str) -> bool {
    let owns_player = player.team_id.as_deref() == Some(club_id);
    player
        .transfer_offers
        .iter()
        .any(|offer| owns_player || offer.from_team_id == club_id)
        || player
            .loan_offers
            .iter()
            .any(|offer| offer.parent_team_id == club_id || offer.from_team_id == club_id)
}

fn matches_position(player: &Player, filters: &[PositionFilter]) -> bool {
    filters.is_empty()
        || filters.iter().any(|filter| match filter {
            PositionFilter::Exact(position) => player.natural_position == *position,
            PositionFilter::Group(group) => {
                player.natural_position.to_group_position() == group.to_group_position()
            }
        })
}

/// A free agent costs no fee and a loan-only player is not bought, so neither is held to the
/// user's club's means. Without a user club there is nothing to afford against.
fn within_budget(player: &Player, context: &ListContext) -> bool {
    let Some(team) = context.user_team else {
        return true;
    };
    let loan_only = player.loan_listed && !player.transfer_listed;
    player.team_id.is_none() || loan_only || can_afford_fee(team, player.market_value)
}

/// Every filter of the query except status, which is counted separately for the status chips.
fn matches_all_but_status(
    player: &Player,
    query: &PlayersPageQuery,
    needle: Option<&str>,
    context: &ListContext,
    teams: &TeamLookup,
) -> bool {
    if let Some(needle) = needle
        && !matches_search(player, needle, teams)
    {
        return false;
    }

    if !matches_scope(player, query.scope, context) || !matches_position(player, &query.positions) {
        return false;
    }

    let age = age_on_or_unknown(context.today, &player.date_of_birth);
    if query.age_min.is_some_and(|min| age < min) || query.age_max.is_some_and(|max| age > max) {
        return false;
    }
    if query.value_min.is_some_and(|min| player.market_value < min)
        || query.value_max.is_some_and(|max| player.market_value > max)
    {
        return false;
    }

    if query.in_budget && !within_budget(player, context) {
        return false;
    }

    if let Some(filter_pos) = &query.position
        && player.natural_position.to_group_position() != filter_pos.to_group_position()
    {
        return false;
    }

    if let Some(filter_team) = &query.team_id
        && player.team_id.as_deref() != Some(filter_team.as_str())
    {
        return false;
    }

    true
}

fn matches_status(player: &Player, status: PlayerStatusFilter) -> bool {
    match status {
        PlayerStatusFilter::All => true,
        PlayerStatusFilter::Transfer => player.transfer_listed,
        PlayerStatusFilter::Loan => player.loan_listed,
        PlayerStatusFilter::FreeAgent => player.team_id.is_none(),
        PlayerStatusFilter::Available => {
            player.team_id.is_none() || player.transfer_listed || player.loan_listed
        }
    }
}

/// A name, a nationality or the player's club: the scouting search has always found by club, and
/// every list shares the one rule.
fn matches_search(player: &Player, needle: &str, teams: &TeamLookup) -> bool {
    let club = team_name_for(player, teams);
    [
        player.full_name.as_str(),
        player.match_name.as_str(),
        player.nationality.as_str(),
        club,
    ]
    .iter()
    .any(|haystack| contains_folded(haystack, needle))
}

fn sort_matches(
    matches: &mut [Row],
    query: &PlayersPageQuery,
    teams: &TeamLookup,
    today: NaiveDate,
) {
    matches.sort_by(|(a, a_seen), (b, b_seen)| {
        let primary = match query.sort_key {
            // The rating the club may see, in the list's direction, an unknown one after every
            // known one either way (maintainer, 29 Oct).
            PlayerSortKey::Ovr => compare_visible_ovr(a_seen.ovr, b_seen.ovr, query.sort_asc),
            key => {
                let ascending = compare_by_key(a, b, key, teams, today);
                if query.sort_asc {
                    ascending
                } else {
                    ascending.reverse()
                }
            }
        };
        // Ties keep one order whichever way the list runs, so a page boundary never reshuffles.
        primary
            .then_with(|| a.full_name.cmp(&b.full_name))
            .then_with(|| a.id.cmp(&b.id))
    });
}

/// The ascending order of two players by a public key. The rating is not public, so its order is
/// `compare_visible_ovr`'s, on what the club may see.
fn compare_by_key(
    a: &Player,
    b: &Player,
    sort_key: PlayerSortKey,
    teams: &TeamLookup,
    today: NaiveDate,
) -> Ordering {
    match sort_key {
        PlayerSortKey::Name => a.full_name.cmp(&b.full_name),
        PlayerSortKey::Position => {
            position_rank(&a.natural_position).cmp(&position_rank(&b.natural_position))
        }
        // Ascending age = youngest first. Within one age the later birth date is the younger, so
        // the whole date breaks the tie rather than the year alone.
        PlayerSortKey::Age => age_on_or_unknown(today, &a.date_of_birth)
            .cmp(&age_on_or_unknown(today, &b.date_of_birth))
            .then_with(|| b.date_of_birth.cmp(&a.date_of_birth)),
        PlayerSortKey::Ovr => Ordering::Equal,
        PlayerSortKey::Value => a.market_value.cmp(&b.market_value),
        PlayerSortKey::Team => team_name_for(a, teams).cmp(team_name_for(b, teams)),
    }
}

fn project_summary(
    player: &Player,
    seen: &PlayerVisibility,
    teams: &TeamLookup,
    today: NaiveDate,
) -> PlayerSummary {
    let team_name = player
        .team_id
        .as_deref()
        .and_then(|id| teams.get(id).copied())
        .map(str::to_string);
    PlayerSummary {
        id: player.id.clone(),
        full_name: player.full_name.clone(),
        match_name: player.match_name.clone(),
        date_of_birth: player.date_of_birth.clone(),
        age: age_on_or_unknown(today, &player.date_of_birth),
        wage: seen.wage.clone(),
        nationality: player.nationality.clone(),
        position: player.position.clone(),
        natural_position: player.natural_position.clone(),
        team_id: player.team_id.clone(),
        team_name,
        market_value: player.market_value,
        ovr: seen.ovr,
        transfer_listed: player.transfer_listed,
        loan_listed: player.loan_listed,
        injured: player.injury.is_some(),
        retired: player.retired,
        media: PlayerSummaryMedia {
            face: player.media.face.clone(),
        },
    }
}

fn team_name_for<'a>(player: &'a Player, teams: &'a TeamLookup) -> &'a str {
    player
        .team_id
        .as_deref()
        .and_then(|id| teams.get(id).copied())
        .unwrap_or("")
}

fn position_rank(pos: &Position) -> u8 {
    use Position::*;
    match pos {
        Goalkeeper => 1,
        Defender | RightBack | CenterBack | LeftBack | RightWingBack | LeftWingBack => 2,
        Midfielder | DefensiveMidfielder | CentralMidfielder | AttackingMidfielder
        | RightMidfielder | LeftMidfielder => 3,
        Forward | RightWinger | LeftWinger | Striker => 4,
    }
}
