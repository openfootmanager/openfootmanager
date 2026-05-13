use ofm_core::game::Game;
use ofm_core::player_events::{pick_response_band, ResponseBandWeights, ResponseOutcomeBand};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

fn team_talk_action_key(tone: &str, context: &str) -> String {
    format!("team_talk:{}:{}", tone, context)
}

fn team_talk_personality_factor(player: &domain::player::Player) -> i32 {
    let composure = i32::from(player.attributes.composure);
    let leadership = i32::from(player.attributes.leadership);
    let aggression = i32::from(player.attributes.aggression);
    ((composure + leadership - aggression) / 6).clamp(-20, 20)
}

fn adjust_weight(weight: &mut u32, delta: i32) {
    *weight = (*weight as i32 + delta).max(0) as u32;
}

fn team_talk_weight_total(weights: &ResponseBandWeights) -> u32 {
    weights.strong_positive
        + weights.mild_positive
        + weights.neutral
        + weights.mild_negative
        + weights.strong_negative
}

fn build_team_talk_weights(
    player: &domain::player::Player,
    tone: &str,
    context: &str,
) -> ResponseBandWeights {
    let mut weights = match (tone, context) {
        ("calm", _) => ResponseBandWeights {
            strong_positive: 1,
            mild_positive: 4,
            neutral: 4,
            mild_negative: 1,
            strong_negative: 0,
        },
        ("motivational", "losing") => ResponseBandWeights {
            strong_positive: 4,
            mild_positive: 4,
            neutral: 1,
            mild_negative: 1,
            strong_negative: 0,
        },
        ("motivational", "drawing") => ResponseBandWeights {
            strong_positive: 2,
            mild_positive: 4,
            neutral: 2,
            mild_negative: 1,
            strong_negative: 0,
        },
        ("motivational", "winning") => ResponseBandWeights {
            strong_positive: 1,
            mild_positive: 3,
            neutral: 3,
            mild_negative: 1,
            strong_negative: 0,
        },
        ("assertive", "losing") => ResponseBandWeights {
            strong_positive: 2,
            mild_positive: 3,
            neutral: 2,
            mild_negative: 2,
            strong_negative: 1,
        },
        ("assertive", "drawing") => ResponseBandWeights {
            strong_positive: 1,
            mild_positive: 2,
            neutral: 3,
            mild_negative: 2,
            strong_negative: 1,
        },
        ("assertive", "winning") => ResponseBandWeights {
            strong_positive: 0,
            mild_positive: 2,
            neutral: 3,
            mild_negative: 3,
            strong_negative: 1,
        },
        ("aggressive", "losing") => ResponseBandWeights {
            strong_positive: 1,
            mild_positive: 3,
            neutral: 2,
            mild_negative: 2,
            strong_negative: 1,
        },
        ("aggressive", "drawing") => ResponseBandWeights {
            strong_positive: 0,
            mild_positive: 2,
            neutral: 2,
            mild_negative: 3,
            strong_negative: 2,
        },
        ("aggressive", "winning") => ResponseBandWeights {
            strong_positive: 0,
            mild_positive: 1,
            neutral: 2,
            mild_negative: 4,
            strong_negative: 3,
        },
        ("praise", "winning") => ResponseBandWeights {
            strong_positive: 4,
            mild_positive: 4,
            neutral: 1,
            mild_negative: 1,
            strong_negative: 0,
        },
        ("praise", "drawing") => ResponseBandWeights {
            strong_positive: 2,
            mild_positive: 3,
            neutral: 3,
            mild_negative: 1,
            strong_negative: 0,
        },
        ("praise", "losing") => ResponseBandWeights {
            strong_positive: 0,
            mild_positive: 1,
            neutral: 3,
            mild_negative: 3,
            strong_negative: 1,
        },
        ("disappointed", "losing") => ResponseBandWeights {
            strong_positive: 0,
            mild_positive: 1,
            neutral: 3,
            mild_negative: 3,
            strong_negative: 2,
        },
        ("disappointed", "drawing") => ResponseBandWeights {
            strong_positive: 0,
            mild_positive: 1,
            neutral: 2,
            mild_negative: 4,
            strong_negative: 2,
        },
        ("disappointed", "winning") => ResponseBandWeights {
            strong_positive: 0,
            mild_positive: 0,
            neutral: 2,
            mild_negative: 4,
            strong_negative: 3,
        },
        _ => ResponseBandWeights {
            strong_positive: 1,
            mild_positive: 2,
            neutral: 4,
            mild_negative: 1,
            strong_negative: 0,
        },
    };

    let trust = i32::from(player.morale_core.manager_trust);
    let leadership = i32::from(player.attributes.leadership);
    let personality = team_talk_personality_factor(player);
    let receptiveness = personality + (trust - 50) / 2 + (leadership - 50) / 3;
    let tone_bias = match tone {
        "aggressive" | "assertive" | "disappointed" => receptiveness - 20,
        "praise" | "motivational" | "calm" => receptiveness + 10,
        _ => receptiveness,
    };

    adjust_weight(&mut weights.strong_positive, tone_bias / 15);
    adjust_weight(&mut weights.mild_positive, tone_bias / 10);
    adjust_weight(&mut weights.mild_negative, -tone_bias / 12);
    adjust_weight(&mut weights.strong_negative, -tone_bias / 10);

    if let Some(issue) = player.morale_core.unresolved_issue.as_ref() {
        let severity = i32::from(issue.severity);
        if severity >= 50 {
            adjust_weight(&mut weights.strong_positive, -((severity - 40) / 15));
            adjust_weight(&mut weights.mild_positive, -((severity - 40) / 12));
            adjust_weight(&mut weights.neutral, 1);
        }
        if severity >= 70 {
            adjust_weight(&mut weights.mild_negative, 1);
            adjust_weight(&mut weights.strong_negative, 1);
        }
    }

    let action_key = team_talk_action_key(tone, context);
    if let Some(memory) = player.morale_core.recent_treatment.as_ref() {
        if memory.action_key == action_key {
            let penalty = i32::from(memory.times_recently_used) * 2;
            adjust_weight(&mut weights.strong_positive, -penalty);
            adjust_weight(&mut weights.mild_positive, -penalty);
            adjust_weight(&mut weights.neutral, i32::from(memory.times_recently_used));
            adjust_weight(
                &mut weights.mild_negative,
                i32::from(memory.times_recently_used),
            );
        }
    }

    if team_talk_weight_total(&weights) == 0 {
        weights.neutral = 1;
    }

    weights
}

fn team_talk_delta_for_band(tone: &str, band: ResponseOutcomeBand) -> i16 {
    match tone {
        "calm" => match band {
            ResponseOutcomeBand::StrongPositive => 5,
            ResponseOutcomeBand::MildPositive => 3,
            ResponseOutcomeBand::Neutral => 1,
            ResponseOutcomeBand::MildNegative => -1,
            ResponseOutcomeBand::StrongNegative => -3,
        },
        "motivational" => match band {
            ResponseOutcomeBand::StrongPositive => 9,
            ResponseOutcomeBand::MildPositive => 5,
            ResponseOutcomeBand::Neutral => 1,
            ResponseOutcomeBand::MildNegative => -2,
            ResponseOutcomeBand::StrongNegative => -5,
        },
        "assertive" => match band {
            ResponseOutcomeBand::StrongPositive => 6,
            ResponseOutcomeBand::MildPositive => 3,
            ResponseOutcomeBand::Neutral => 0,
            ResponseOutcomeBand::MildNegative => -3,
            ResponseOutcomeBand::StrongNegative => -6,
        },
        "aggressive" => match band {
            ResponseOutcomeBand::StrongPositive => 7,
            ResponseOutcomeBand::MildPositive => 2,
            ResponseOutcomeBand::Neutral => -1,
            ResponseOutcomeBand::MildNegative => -5,
            ResponseOutcomeBand::StrongNegative => -9,
        },
        "praise" => match band {
            ResponseOutcomeBand::StrongPositive => 8,
            ResponseOutcomeBand::MildPositive => 5,
            ResponseOutcomeBand::Neutral => 1,
            ResponseOutcomeBand::MildNegative => -2,
            ResponseOutcomeBand::StrongNegative => -4,
        },
        "disappointed" => match band {
            ResponseOutcomeBand::StrongPositive => 3,
            ResponseOutcomeBand::MildPositive => 1,
            ResponseOutcomeBand::Neutral => -2,
            ResponseOutcomeBand::MildNegative => -5,
            ResponseOutcomeBand::StrongNegative => -8,
        },
        _ => match band {
            ResponseOutcomeBand::StrongPositive => 4,
            ResponseOutcomeBand::MildPositive => 2,
            ResponseOutcomeBand::Neutral => 0,
            ResponseOutcomeBand::MildNegative => -2,
            ResponseOutcomeBand::StrongNegative => -4,
        },
    }
}

fn reduce_by_recent_team_talk(
    delta: i16,
    player: &domain::player::Player,
    action_key: &str,
) -> i16 {
    let Some(memory) = player.morale_core.recent_treatment.as_ref() else {
        return delta;
    };

    if memory.action_key != action_key || delta <= 0 {
        return delta;
    }

    delta - i16::from(memory.times_recently_used) * 3
}

fn cap_team_talk_delta(delta: i16, player: &domain::player::Player) -> i16 {
    let Some(issue) = player.morale_core.unresolved_issue.as_ref() else {
        return delta;
    };

    if delta <= 0 {
        return delta;
    }

    if issue.severity >= 75 {
        return 0;
    }

    if issue.severity >= 50 {
        return ((delta + 1) / 2).max(1);
    }

    delta
}

fn update_recent_team_talk(player: &mut domain::player::Player, action_key: &str) {
    match player.morale_core.recent_treatment.as_mut() {
        Some(memory) if memory.action_key == action_key => {
            memory.times_recently_used = memory.times_recently_used.saturating_add(1);
        }
        Some(memory) => {
            memory.action_key = action_key.to_string();
            memory.times_recently_used = 1;
        }
        None => {
            player.morale_core.recent_treatment = Some(domain::player::RecentTreatmentMemory {
                action_key: action_key.to_string(),
                times_recently_used: 1,
            });
        }
    }
}

pub fn apply_team_talk(
    game: &mut Game,
    tone: &str,
    context: &str,
    seed: u64,
) -> Result<Vec<serde_json::Value>, String> {
    let user_team_id = game.manager.team_id.clone().ok_or("be.error.noTeamAssigned")?;
    let mut rng = StdRng::seed_from_u64(seed);
    let action_key = team_talk_action_key(tone, context);
    let mut results: Vec<serde_json::Value> = Vec::new();

    for player in game.players.iter_mut() {
        if player.team_id.as_deref() != Some(&user_team_id) {
            continue;
        }

        let base_morale = i16::from(player.morale);
        let weights = build_team_talk_weights(player, tone, context);
        let roll = rng.random_range(0..team_talk_weight_total(&weights));
        let band = pick_response_band(&weights, roll);
        let delta = cap_team_talk_delta(
            reduce_by_recent_team_talk(team_talk_delta_for_band(tone, band), player, &action_key),
            player,
        )
        .clamp(-12, 12);

        let new_morale = (base_morale + delta).clamp(10, 100) as u8;
        let actual_delta = i16::from(new_morale) - base_morale;
        player.morale = new_morale;
        update_recent_team_talk(player, &action_key);

        results.push(serde_json::json!({
            "player_id": player.id,
            "player_name": player.match_name,
            "old_morale": base_morale,
            "new_morale": new_morale,
            "delta": actual_delta
        }));
    }

    Ok(results)
}
