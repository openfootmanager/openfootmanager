//! The transfer market: the AI-driven engine that scores targets, opens
//! negotiations and generates incoming offers for the user's players. Sits at
//! the top of the transfer call graph, driving the bid/loan/execution clusters.
//! Byte-faithful extraction from the original transfers.rs; the MarketCandidate /
//! MarketTarget carrier types and the cross-cluster shared helpers remain in the
//! parent module. The two public entry points stay resolvable as
//! `ofm_core::transfers::*`.

use super::*;

/// Broad position group index (0=GK, 1=DEF, 2=MID, 3=FWD) for squad-depth maths.
pub(crate) fn position_group_index(position: &domain::player::Position) -> usize {
    match position.to_group_position() {
        domain::player::Position::Goalkeeper => 0,
        domain::player::Position::Defender => 1,
        domain::player::Position::Midfielder => 2,
        _ => 3,
    }
}

/// Whether a club has a realistic reason to pursue a target: it isn't far below
/// the player's current club in stature, and it isn't already overloaded in the
/// player's position group.
pub(crate) fn buyer_has_genuine_interest(
    buyer_reputation: u32,
    owner_reputation: u32,
    buyer_position_depth: usize,
) -> bool {
    let reputation_deficit = owner_reputation as i32 - buyer_reputation as i32;
    reputation_deficit <= MAX_BUYER_REPUTATION_DEFICIT
        && buyer_position_depth < POSITION_GROUP_SURPLUS_THRESHOLD
}

/// Current squad depth per club and broad position group, computed once so the
/// market sweep doesn't re-scan every roster.
pub(crate) fn squad_position_depths(game: &Game) -> std::collections::HashMap<String, [usize; 4]> {
    let mut depths: std::collections::HashMap<String, [usize; 4]> =
        std::collections::HashMap::new();
    for player in &game.players {
        let Some(team_id) = player.team_id.as_deref() else {
            continue;
        };
        let slot = position_group_index(&player.natural_position);
        depths.entry(team_id.to_string()).or_default()[slot] += 1;
    }
    depths
}

pub(crate) fn contract_days_remaining(
    current_date: NaiveDate,
    contract_end: Option<&str>,
) -> Option<i64> {
    let contract_end = contract_end?;
    let contract_end_date = NaiveDate::parse_from_str(contract_end, "%Y-%m-%d").ok()?;
    Some((contract_end_date - current_date).num_days())
}

pub(crate) fn infer_player_importance(
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
) -> PlayerImportance {
    if owner_team.starting_xi_ids.iter().any(|id| id == &player.id) {
        return PlayerImportance::Key;
    }

    if player.market_value >= 1_500_000 {
        return PlayerImportance::Regular;
    }

    PlayerImportance::Fringe
}

pub(crate) fn minimum_acceptable_fee(
    current_date: NaiveDate,
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
    buyer_team: &domain::team::Team,
) -> u64 {
    let mut multiplier: f64 = if player.transfer_listed { 0.8 } else { 1.2 };

    if let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    {
        if days_remaining <= 60 {
            multiplier -= 0.25;
        } else if days_remaining <= 180 {
            multiplier -= 0.15;
        } else if days_remaining <= 365 {
            multiplier -= 0.05;
        }
    }

    match infer_player_importance(player, owner_team) {
        PlayerImportance::Key => multiplier += 0.2,
        PlayerImportance::Regular => multiplier += 0.1,
        PlayerImportance::Fringe => {}
    }

    if player.morale <= 40 {
        multiplier -= 0.05;
    }

    let openness_score = player_move_openness_score(current_date, player, owner_team, buyer_team);
    if openness_score >= 60 {
        multiplier -= 0.20;
    } else if openness_score >= 40 {
        multiplier -= 0.10;
    }

    let multiplier = multiplier.clamp(0.55, 1.6);
    ((player.market_value as f64) * multiplier).round() as u64
}

pub(crate) fn player_move_openness_score(
    current_date: NaiveDate,
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
    buyer_team: &domain::team::Team,
) -> i32 {
    let mut score = 0;

    if player.morale <= 45 {
        score += 20;
    } else if player.morale <= 60 {
        score += 10;
    }

    if player.stats.appearances <= 2 {
        score += 15;
    } else if player.stats.appearances <= 5 {
        score += 8;
    }

    if let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    {
        if days_remaining <= 180 {
            score += 20;
        } else if days_remaining <= 365 {
            score += 10;
        }
    }

    let reputation_gap = buyer_team.reputation as i32 - owner_team.reputation as i32;
    if reputation_gap >= 200 {
        score += 25;
    } else if reputation_gap >= 75 {
        score += 15;
    }

    if player.transfer_listed {
        score += 10;
    }

    score
}

pub(crate) fn apply_blocked_move_consequences(
    player: &mut domain::player::Player,
    openness_score: i32,
) {
    if openness_score < 40 {
        return;
    }

    let morale_drop = if openness_score >= 60 { 10 } else { 6 };
    player.morale = (i16::from(player.morale) - morale_drop).clamp(0, 100) as u8;
    player.morale_core.manager_trust =
        (i16::from(player.morale_core.manager_trust) - 5).clamp(0, 100) as u8;
    player.morale_core.unresolved_issue = Some(domain::player::PlayerIssue {
        category: domain::player::PlayerIssueCategory::Contract,
        severity: if openness_score >= 60 { 75 } else { 60 },
    });
}

pub(crate) fn award_leaderboard_player_ids(game: &Game) -> HashSet<String> {
    let awards = crate::season_awards::compute_season_awards(game);

    awards
        .golden_boot
        .iter()
        .chain(awards.assist_king.iter())
        .chain(awards.player_of_year.iter())
        .chain(awards.clean_sheet_king.iter())
        .chain(awards.most_appearances.iter())
        .chain(awards.young_player.iter())
        .map(|entry| entry.player_id.clone())
        .collect()
}

pub fn evaluate_transfer_market(game: &mut Game) {
    expire_stale_transfer_offers(game);
    expire_stale_loan_offers(game);

    if !transfer_window_is_open(game) {
        return;
    }

    let user_team_id = game.manager.team_id.clone();

    let current_date = game.clock.current_date.date_naive();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let award_leaderboards = award_leaderboard_player_ids(game);
    let team_reputation: std::collections::HashMap<String, u32> = game
        .teams
        .iter()
        .map(|team| (team.id.clone(), team.reputation))
        .collect();
    let position_depths = squad_position_depths(game);

    // In a multi-competition world only the player's active scope shops the
    // market each day; dormant clubs are handled by lighter periodic passes.
    // `None` means no scope is configured, so every club is a potential buyer.
    let active_team_ids = game.active_team_ids();
    let buyer_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|team| Some(team.id.as_str()) != user_team_id.as_deref())
        .filter(|team| {
            active_team_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(&team.id))
        })
        .map(|team| team.id.clone())
        .collect();
    let mut completed_ai_transfers = 0_usize;
    let mut moved_player_ids: HashSet<String> = HashSet::new();
    // New incoming offers opened to user players today, tracked to throttle the
    // inbox: at most one new club per player and a hard squad-wide ceiling.
    let mut new_offers_per_player: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut new_loan_offers_per_player: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut new_user_offers_today = 0_usize;
    let mut new_user_loan_offers_today = 0_usize;

    // A player's transfer appeal and asking fee don't depend on who's buying, so
    // score every player once and keep only the genuinely attractive targets.
    // Each club then scans this short, score-sorted list instead of the whole
    // world, turning an O(clubs × players) sweep into O(players + clubs × shortlist).
    let mut shortlist: Vec<MarketTarget> = Vec::new();
    for player in &game.players {
        let Some(owner_team_id) = player.team_id.as_deref() else {
            continue;
        };
        if player_has_pending_registration(player) {
            continue;
        }
        let mut score = incoming_interest_score(current_date, player);
        if award_leaderboards.contains(&player.id) {
            score += AWARD_LEADERBOARD_INTEREST_BONUS;
        }
        if score < 35 {
            continue;
        }
        let is_user_owned = Some(owner_team_id) == user_team_id.as_deref();
        let pending_offer_clubs: HashSet<String> = if is_user_owned {
            player
                .transfer_offers
                .iter()
                .filter(|offer| offer.status == TransferOfferStatus::Pending)
                .map(|offer| offer.from_team_id.clone())
                .collect()
        } else {
            HashSet::new()
        };
        shortlist.push(MarketTarget {
            player_id: player.id.clone(),
            owner_team_id: owner_team_id.to_string(),
            is_user_owned,
            score,
            fee: suggested_incoming_fee(current_date, player),
            position_group_index: position_group_index(&player.natural_position),
            owner_reputation: team_reputation.get(owner_team_id).copied().unwrap_or(0),
            pending_offer_clubs,
        });
    }
    // Highest appeal first; a stable sort preserves the original ordering among
    // equally appealing targets, so selection is unchanged.
    shortlist.sort_by(|a, b| b.score.cmp(&a.score));

    for buyer_id in buyer_ids {
        let Some(buyer_team) = game.teams.iter().find(|team| team.id == buyer_id).cloned() else {
            continue;
        };
        let buyer_depths = position_depths.get(&buyer_id).copied().unwrap_or([0; 4]);

        let loan_offer_player_id = if let Some(user_team_id) = user_team_id.as_deref() {
            if new_user_loan_offers_today < MAX_NEW_INCOMING_USER_OFFERS_PER_DAY {
                create_incoming_user_loan_offer_if_any(
                    game,
                    user_team_id,
                    &buyer_id,
                    &buyer_team.name,
                    &today,
                    current_date,
                    &new_loan_offers_per_player,
                )
            } else {
                None
            }
        } else {
            None
        };
        if let Some(player_id) = loan_offer_player_id.as_ref() {
            *new_loan_offers_per_player
                .entry(player_id.clone())
                .or_insert(0) += 1;
            new_user_loan_offers_today += 1;
        }

        // The list is score-sorted, so the first target clearing this club's
        // filters is its highest-appeal eligible signing.
        let chosen = shortlist.iter().find(|target| {
            if target.owner_team_id == buyer_id || moved_player_ids.contains(&target.player_id) {
                return false;
            }
            if loan_offer_player_id.as_deref() == Some(target.player_id.as_str()) {
                return false;
            }
            if target.is_user_owned {
                if target.pending_offer_clubs.contains(&buyer_id)
                    || new_user_offers_today >= MAX_NEW_INCOMING_USER_OFFERS_PER_DAY
                    || new_offers_per_player
                        .get(&target.player_id)
                        .copied()
                        .unwrap_or(0)
                        >= MAX_NEW_INCOMING_OFFERS_PER_USER_PLAYER_PER_DAY
                {
                    return false;
                }
            } else if completed_ai_transfers >= MAX_COMPLETED_AI_TRANSFERS_PER_DAY {
                return false;
            }
            // Clubs only chase players that fit their stature and a position they
            // actually need, so a single star doesn't draw the whole division.
            if !buyer_has_genuine_interest(
                buyer_team.reputation,
                target.owner_reputation,
                buyer_depths[target.position_group_index],
            ) {
                return false;
            }
            buyer_team.transfer_budget >= target.fee as i64
                && buyer_team.finance >= target.fee as i64
        });

        let Some(target) = chosen else {
            continue;
        };
        let candidate = MarketCandidate {
            player_id: target.player_id.clone(),
            owner_team_id: target.owner_team_id.clone(),
            score: target.score,
            fee: target.fee,
        };

        if Some(candidate.owner_team_id.as_str()) == user_team_id.as_deref() {
            create_incoming_user_offer(game, &candidate, &buyer_id, &buyer_team.name, &today);
            *new_offers_per_player
                .entry(candidate.player_id.clone())
                .or_insert(0) += 1;
            new_user_offers_today += 1;
            continue;
        }

        if candidate.score <= 60 || completed_ai_transfers >= MAX_COMPLETED_AI_TRANSFERS_PER_DAY {
            continue;
        }

        if execute_transfer(
            game,
            &candidate.player_id,
            &buyer_id,
            &candidate.owner_team_id,
            candidate.fee,
        )
        .is_ok()
        {
            moved_player_ids.insert(candidate.player_id);
            completed_ai_transfers += 1;
        }
    }
}

pub fn generate_incoming_transfer_offers(game: &mut Game) {
    evaluate_transfer_market(game);
}
