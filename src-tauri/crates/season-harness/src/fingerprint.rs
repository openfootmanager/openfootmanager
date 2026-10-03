//! A world reduced to what a replay must reproduce, for comparing two runs.

use ofm_core::game::Game;

/// Everything a day's dice and a rollover's people decide, one line per fact, in a fixed order.
///
/// Not the whole `Game`: ids still minted at random for inbox messages, news and bids differ
/// between two runs and are not the dice. Fixture ids, player ids, results, tables, bodies,
/// money, staff and managers are all here, so a draw that escapes the game's seed shows up as
/// the first line two fingerprints disagree on (see [`first_difference`]).
pub fn fingerprint(game: &Game) -> String {
    let mut lines = vec![format!("date {}", game.clock.current_date.date_naive())];
    for competition in &game.competitions {
        lines.push(format!(
            "competition {} {} season{}",
            competition.id, competition.name, competition.season
        ));
        for row in &competition.standings {
            lines.push(format!(
                "  table {} p{} pts{} gf{} ga{}",
                row.team_id, row.played, row.points, row.goals_for, row.goals_against
            ));
        }
        for fixture in &competition.fixtures {
            let result = fixture
                .result
                .as_ref()
                .map(|result| format!("{}-{}", result.home_goals, result.away_goals));
            lines.push(format!(
                "  fixture {} {} {}-{} {:?} {:?}",
                fixture.id,
                fixture.date,
                fixture.home_team_id,
                fixture.away_team_id,
                fixture.status,
                result
            ));
        }
    }
    for national in &game.national_teams {
        for fixture in &national.fixtures {
            lines.push(format!(
                "national {} {} {}-{} {:?}",
                fixture.id,
                fixture.date,
                fixture.home_team_id,
                fixture.away_team_id,
                fixture.status
            ));
        }
    }
    for player in &game.players {
        lines.push(format!(
            "player {} team{:?} ovr{} pot{} cond{} fit{} morale{} inj{:?} retired{:?} {:?}",
            player.id,
            player.team_id,
            player.ovr,
            player.potential,
            player.condition,
            player.fitness,
            player.morale,
            player.injury,
            player.retired,
            player.stats
        ));
    }
    for team in &game.teams {
        lines.push(format!("team {} finance{}", team.id, team.finance));
    }
    for staff in &game.staff {
        lines.push(format!(
            "staff {} {:?} {:?}",
            staff.id, staff.team_id, staff.role
        ));
    }
    for manager in &game.managers {
        lines.push(format!("manager {} {:?}", manager.id, manager.team_id));
    }
    lines.join("\n")
}

/// The first line two fingerprints disagree on, so a failure names something to go and look at.
pub fn first_difference(first: &str, second: &str) -> String {
    first
        .lines()
        .zip(second.lines())
        .find(|(a, b)| a != b)
        .map(|(a, b)| format!("{a}\n  !=\n{b}"))
        .unwrap_or_else(|| "the fingerprints differ in length only".to_string())
}
