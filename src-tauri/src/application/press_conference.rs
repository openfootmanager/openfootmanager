//! The checks a press conference has to pass, shared by the two callers that hold one.
//!
//! The UI command and the MCP tool each apply a press conference their own way — different morale
//! ranges, different article wording, different error rendering — and those differences are real,
//! so this module deliberately holds none of them. What it holds is the part that was accidentally
//! duplicated: which match the conference is about, whose players may be named, and whether one
//! has already been held today. All three were fixed in the MCP tool first and stayed broken in the
//! command for as long as there were two copies to fix.
//!
//! Every function here returns data, never a message. The MCP tool renders prose for an agent and
//! the command renders `be.error.*` keys for the UI; neither wording belongs in a shared check.

use ofm_core::game::Game;

/// The match a press conference held today is about: the most recent fixture that has a result,
/// involving the team, across every competition it plays in.
///
/// Reading `game.league` instead would silently report the last *league* match after a cup tie:
/// that field mirrors one competition, and `Game` says so itself — "the legacy `league` mirror
/// misses cups and isn't reliable" (`game.rs`).
///
/// The filter is the result, not `FixtureStatus::Completed`, and deliberately so. A fixture
/// carrying a `MatchResult` has by definition been played, whereas the status is a workflow marker
/// that `league_repo::parse_fixture_status` defaults to `Scheduled` for any value it does not
/// recognise — so a status filter could hide a match that really was played. The two cannot
/// disagree in practice anyway: every write of `result` sets the status first (`turn/post_match.rs`,
/// `catchup.rs`), and `InProgress` is never assigned outside tests.
///
/// Returns `(home_team_id, away_team_id, home_goals, away_goals)`, owned so the caller can go on to
/// borrow `game` mutably.
pub fn last_completed_match(game: &Game, team_id: &str) -> Option<(String, String, u8, u8)> {
    let fixture = game
        .competitions
        .iter()
        .flat_map(|competition| competition.fixtures.iter())
        .filter(|f| f.result.is_some() && (f.home_team_id == team_id || f.away_team_id == team_id))
        .max_by(|a, b| a.date.cmp(&b.date))?;
    let result = fixture
        .result
        .as_ref()
        .expect("filtered to fixtures with a result");
    Some((
        fixture.home_team_id.clone(),
        fixture.away_team_id.clone(),
        result.home_goals,
        result.away_goals,
    ))
}

/// The first named player who is not in the team's squad, if there is one.
///
/// Both callers resolve a named player against the whole world before moving their morale, so
/// without this an opposition striker could be praised into a better mood — a morale boost handed
/// to a rival. The UI only ever offers the user's own squad; this is the backend saying so too.
///
/// Empty ids mean "no player named" and are skipped. Takes the ids as an iterator so the two
/// callers' different answer types both fit.
pub fn first_player_outside_squad<'a>(
    game: &Game,
    team_id: &str,
    player_ids: impl Iterator<Item = &'a str>,
) -> Option<String> {
    let squad: std::collections::HashSet<&str> = game
        .players
        .iter()
        .filter(|p| p.team_id.as_deref() == Some(team_id))
        .map(|p| p.id.as_str())
        .collect();
    player_ids
        .filter(|id| !id.is_empty())
        .find(|id| !squad.contains(id))
        .map(str::to_string)
}

/// The id the conference's news article will carry, and whether one already exists.
///
/// The id is derived from the date alone, so a second conference on the same day files an article
/// sharing the first one's id — the id the news list keys on and selects by, which makes the later
/// article unreachable — and re-applies the whole squad delta, walking every player towards maximum
/// morale. This is the guard every other generator of a date-derived article id already uses; see
/// `turn/news.rs`.
pub fn todays_article_id(game: &Game) -> (String, bool) {
    let article_id = format!("press_conf_{}", game.clock.current_date.format("%Y-%m-%d"));
    let already_held = game.news.iter().any(|article| article.id == article_id);
    (article_id, already_held)
}
