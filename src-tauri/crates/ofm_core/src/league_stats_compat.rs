//! One compatibility upgrade for old save blobs and world snapshots, before Player discards
//! the retired league counter. The competition map is the only live league-statistics source.

use crate::end_of_season::club_league_table_in;
use domain::league::League;
use domain::player::PlayerSeasonStats;
use serde_json::Value;

fn needs_upgrade(player: &Value) -> bool {
    player.get("league_stats_by_competition").is_none()
}

/// Upgrade a single-file world or legacy game using its own competition context.
pub fn upgrade_world_json(world: &mut Value) -> Result<(), serde_json::Error> {
    if !world
        .get("players")
        .and_then(Value::as_array)
        .is_some_and(|players| players.iter().any(needs_upgrade))
    {
        return Ok(());
    }
    let mut competitions: Vec<League> = world
        .get("competitions")
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()?
        .unwrap_or_default();
    if competitions.is_empty()
        && let Some(league) = world.get("league").filter(|value| !value.is_null())
    {
        competitions.push(serde_json::from_value(league.clone())?);
    }
    if let Some(players) = world.get_mut("players") {
        upgrade_players_json(players, &competitions)?;
    }
    Ok(())
}

/// Upgrade players from a manifest shard, or from a single-file document above.
/// An explicit map, including {}, always wins over any stale legacy counter.
pub fn upgrade_players_json(
    players: &mut Value,
    competitions: &[League],
) -> Result<(), serde_json::Error> {
    for player in players
        .as_array_mut()
        .into_iter()
        .flatten()
        .filter(|player| needs_upgrade(player))
    {
        let Some(club) = player.get("team_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(league) = club_league_table_in(competitions, club) else {
            continue;
        };
        let Some(old) = player
            .get("league_stats")
            .filter(|value| !value.is_null())
            .or_else(|| player.get("stats"))
        else {
            continue;
        };
        let stats: PlayerSeasonStats = serde_json::from_value(old.clone())?;
        let by_competition = std::collections::BTreeMap::from([(league.id.clone(), stats)]);
        if let Some(fields) = player.as_object_mut() {
            fields.insert(
                "league_stats_by_competition".into(),
                serde_json::to_value(by_competition)?,
            );
        }
    }
    Ok(())
}
