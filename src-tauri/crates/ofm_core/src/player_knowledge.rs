//! What the manager's club knows about a player: the one rule every screen that shows a player's
//! rating, potential, condition or wage reads, so none of them decides it for itself.

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use domain::player::{Player, PlayerMovementKind};
use serde::Serialize;

use crate::game::{Game, ScoutedPlayer};
use crate::potential_opinion::{OpinionSource, PotentialOpinion, judged_range};
use crate::slices::scouted_report::is_out_of_date;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlayerKnowledge {
    /// The club's own player, including one it has loaned out or loaned in.
    Own,
    /// A player at a club in the manager's own division: the staff see him every season.
    Known,
    /// A player the club has a scout's report on, and whether that report still describes him.
    Scouted { scouted_on: String, current: bool },
    /// Nothing beyond what is public.
    Unknown,
}

/// The strongest thing the club knows, in this order: its own player, then a player in its division,
/// then one a scout reported on, then nothing. A scouted player at a club in the manager's division is
/// `Known` (live), not `Scouted` (a snapshot). An unemployed manager has no club and no division, so
/// he knows only the players his scouts reported on.
pub fn player_knowledge(game: &Game, player: &Player) -> PlayerKnowledge {
    KnowledgeView::of(game).knowledge(player)
}

/// What the manager's club can see, worked out once per request rather than once per player: the
/// club, the clubs in its division, the clubs it meets today, and its scouts' reports by player. A
/// list asks it about every row, so no row searches the reports.
pub struct KnowledgeView<'a> {
    club: Option<&'a str>,
    known_clubs: HashSet<&'a str>,
    opponents_today: HashSet<&'a str>,
    reports: HashMap<&'a str, &'a ScoutedPlayer>,
    last_rollover: Option<&'a str>,
}

impl<'a> KnowledgeView<'a> {
    pub fn of(game: &'a Game) -> Self {
        let club = game.manager.team_id.as_deref();
        Self {
            club,
            known_clubs: club.map_or_else(HashSet::new, |club| clubs_in_division_of(game, club)),
            opponents_today: club.map_or_else(HashSet::new, |club| opponents_today(game, club)),
            reports: reports_by_player(game),
            last_rollover: game.last_season_rollover_on.as_deref(),
        }
    }

    /// The strongest thing the club knows about the player (see [`player_knowledge`]).
    pub fn knowledge(&self, player: &Player) -> PlayerKnowledge {
        if self
            .club
            .is_some_and(|club| is_at_or_owned_by(player, club))
        {
            return PlayerKnowledge::Own;
        }
        if player
            .team_id
            .as_deref()
            .is_some_and(|club| self.known_clubs.contains(club))
        {
            return PlayerKnowledge::Known;
        }
        match self.reports.get(player.id.as_str()) {
            Some(report) => PlayerKnowledge::Scouted {
                scouted_on: report.scouted_on.clone(),
                current: !is_out_of_date(report, self.last_rollover),
            },
            None => PlayerKnowledge::Unknown,
        }
    }

    /// The club knows the squad at `club_id`: its own, or a rival in its division, whose players
    /// it sees every season. A figure about a whole squad, such as its average rating, shows only
    /// then; a few scouted players would make a skewed figure that looks authoritative.
    pub fn knows_squad_at(&self, club_id: &str) -> bool {
        self.club == Some(club_id) || self.known_clubs.contains(club_id)
    }

    fn wage_for(&self, knowledge: &PlayerKnowledge, player: &Player) -> VisibleWage {
        match knowledge {
            PlayerKnowledge::Own => VisibleWage::Exact {
                weekly: player.wage(),
            },
            PlayerKnowledge::Scouted { .. } => VisibleWage::Approximate {
                weekly: two_significant_figures(player.wage()),
            },
            PlayerKnowledge::Known | PlayerKnowledge::Unknown => VisibleWage::Hidden,
        }
    }

    fn condition_for(&self, knowledge: &PlayerKnowledge, player: &Player) -> Option<u8> {
        let meets_today = player
            .team_id
            .as_deref()
            .is_some_and(|club| self.opponents_today.contains(club));
        (*knowledge == PlayerKnowledge::Own || meets_today).then_some(player.condition)
    }
}

/// Every club whose division is the same table as `club`'s. A club's division is the table
/// `club_league_table` picks for it, so only that table's own clubs can share it.
fn clubs_in_division_of<'a>(game: &'a Game, club: &str) -> HashSet<&'a str> {
    let Some(division) = division_of(game, club) else {
        return HashSet::new();
    };
    let Some(table) = game.competitions.iter().find(|table| table.id == division) else {
        return HashSet::new();
    };
    table
        .participant_ids
        .iter()
        .map(String::as_str)
        .chain(table.standings.iter().map(|row| row.team_id.as_str()))
        .filter(|candidate| division_of(game, candidate) == Some(division))
        .collect()
}

/// The clubs `club` meets today, in any competition.
fn opponents_today<'a>(game: &'a Game, club: &str) -> HashSet<&'a str> {
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    game.competitions
        .iter()
        .flat_map(|competition| &competition.fixtures)
        .filter(|fixture| fixture.date == today)
        .filter_map(|fixture| {
            if fixture.home_team_id == club {
                Some(fixture.away_team_id.as_str())
            } else if fixture.away_team_id == club {
                Some(fixture.home_team_id.as_str())
            } else {
                None
            }
        })
        .collect()
}

/// The club holds his contract (a loaned-out player's `team_id` is the borrower) or he plays for it
/// (a loaned-in player's `team_id` is the club).
fn is_at_or_owned_by(player: &Player, club_id: &str) -> bool {
    player.team_id.as_deref() == Some(club_id)
        || player
            .active_loan
            .as_ref()
            .is_some_and(|loan| loan.parent_team_id == club_id)
}

/// The league table a club contests. The legacy `game.league` never stands in for one.
fn division_of<'a>(game: &'a Game, club: &str) -> Option<&'a str> {
    crate::end_of_season::club_league_table(game, club).map(|league| league.id.as_str())
}

/// Each scouted player's report. The first report for a player is his, as
/// `query_scouted_report` reads it; scouting a player again replaces his report anyway.
fn reports_by_player(game: &Game) -> HashMap<&str, &ScoutedPlayer> {
    let mut reports = HashMap::with_capacity(game.scouted_players.len());
    for report in &game.scouted_players {
        reports.entry(report.player_id.as_str()).or_insert(report);
    }
    reports
}

/// The weekly wage the club may see: exact for its own player, rounded for one its scout reported
/// on, and nothing for anyone else. A known rival's wage is his club's business, like his condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisibleWage {
    Exact {
        weekly: u32,
    },
    /// Rounded to two significant figures: what a scout would hear, not the contract's figure.
    Approximate {
        weekly: u32,
    },
    Hidden,
}

impl VisibleWage {
    /// The figure the club may see, exact or rounded; none when the wage is hidden.
    pub fn value(&self) -> Option<u32> {
        match self {
            VisibleWage::Exact { weekly } | VisibleWage::Approximate { weekly } => Some(*weekly),
            VisibleWage::Hidden => None,
        }
    }
}

/// What a list row may show about a player: the club's knowledge of him, and his rating, wage and
/// condition as that knowledge allows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerVisibility {
    pub knowledge: PlayerKnowledge,
    /// His rating for the club's own player and a division rival; none for a scouted or unknown one
    /// until a scout's estimate exists.
    pub ovr: Option<u8>,
    pub wage: VisibleWage,
    pub condition: Option<u8>,
}

impl KnowledgeView<'_> {
    pub fn of_player(&self, player: &Player) -> PlayerVisibility {
        let knowledge = self.knowledge(player);
        let knows_rating = matches!(knowledge, PlayerKnowledge::Own | PlayerKnowledge::Known);
        PlayerVisibility {
            ovr: knows_rating.then_some(player.ovr),
            wage: self.wage_for(&knowledge, player),
            condition: self.condition_for(&knowledge, player),
            knowledge,
        }
    }
}

/// Whether a visible rating passes a min/max filter. An unknown rating fails any bound that is set
/// (maintainer, 29 Oct): a filter on a rating the club does not know would tell it.
pub fn passes_ovr_filter(ovr: Option<u8>, min: Option<u8>, max: Option<u8>) -> bool {
    if min.is_none() && max.is_none() {
        return true;
    }
    ovr.is_some_and(|ovr| min.is_none_or(|min| ovr >= min) && max.is_none_or(|max| ovr <= max))
}

/// Whether a visible wage passes a maximum. A hidden wage fails a set maximum, as an unknown rating
/// fails a rating bound, so a price filter is never an oracle for a wage the club cannot see.
pub fn passes_wage_filter(wage: &VisibleWage, max: Option<u32>) -> bool {
    max.is_none_or(|max| wage.value().is_some_and(|weekly| weekly <= max))
}

/// Orders two visible ratings for a list sorted by rating, ascending or descending: an unknown
/// rating sorts after every known one either way (maintainer, 29 Oct).
pub fn compare_visible_ovr(a: Option<u8>, b: Option<u8>, ascending: bool) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Some(a), Some(b)) if ascending => a.cmp(&b),
        (Some(a), Some(b)) => b.cmp(&a),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

pub fn visible_wage(game: &Game, player: &Player) -> VisibleWage {
    let view = KnowledgeView::of(game);
    view.wage_for(&view.knowledge(player), player)
}

/// The player's condition as the club may see it. Its own player's always; an opponent's on the
/// day the two clubs meet, in any competition, as the team sheet and the warm-up show it
/// (maintainer, 29 Oct), a loanee counting for the club he plays for; nobody else's, a rival in
/// the club's own division included.
pub fn visible_condition(game: &Game, player: &Player) -> Option<u8> {
    let view = KnowledgeView::of(game);
    view.condition_for(&view.knowledge(player), player)
}

fn two_significant_figures(value: u32) -> u32 {
    let mut step = 1;
    while value / step >= 100 {
        step *= 10;
    }
    // Round half up to the step: 1,250 → 1,300, 15,843 → 16,000.
    value.saturating_add(step / 2) / step * step
}

/// The knowledge of each requested player. An id no player has is left out of the answer.
pub fn players_knowledge(game: &Game, player_ids: &[String]) -> HashMap<String, PlayerKnowledge> {
    let wanted: HashSet<&str> = player_ids.iter().map(String::as_str).collect();
    let view = KnowledgeView::of(game);
    game.players
        .iter()
        .filter(|player| wanted.contains(player.id.as_str()))
        .map(|player| (player.id.clone(), view.knowledge(player)))
        .collect()
}

/// Six weeks of daily sessions: how long the club's staff watch one of its players train before
/// they hold an opinion of his potential (maintainer, B3).
pub const COACHES_OPINION_AFTER_DAYS: i64 = 42;

/// What the club may read of a player's potential: an opinion, as a range, never the number.
///
/// The club's staff hold one about a player who has trained with the club for six weeks (judged by
/// its best eye for potential, worked out from today's potential, so it follows his development).
/// Otherwise it is the opinion a scout wrote into his report, as of the report's date. Nobody else
/// is judged: knowing a division rival's rating is not knowing his potential. A player whose
/// potential is not stored (0) has none.
pub fn potential_opinion(game: &Game, player: &Player) -> Option<PotentialOpinion> {
    if player.potential == 0 {
        return None;
    }
    coaches_opinion(game, player).or_else(|| scouts_opinion(game, player))
}

fn coaches_opinion(game: &Game, player: &Player) -> Option<PotentialOpinion> {
    let club = game.manager.team_id.as_deref()?;
    let since = trains_with_club_since(player, club)?;
    let since = NaiveDate::parse_from_str(since, "%Y-%m-%d").ok()?;
    let today = game.clock.current_date.date_naive();
    if (today - since).num_days() < COACHES_OPINION_AFTER_DAYS {
        return None;
    }
    // The club's best eye; between equals the first by id, so the judge (whose id seeds the
    // range) never depends on the order a save lists the staff in.
    let judge = game
        .staff
        .iter()
        .filter(|staff| staff.team_id.as_deref() == Some(club))
        .max_by(|left, right| {
            left.attributes
                .judging_potential
                .cmp(&right.attributes.judging_potential)
                .then_with(|| right.id.cmp(&left.id))
        })?;
    let range = judged_range(
        &player.id,
        &judge.id,
        player.potential,
        judge.attributes.judging_potential,
    );
    Some(PotentialOpinion::of(range, OpinionSource::Coaches))
}

/// The day since which the player has trained with `club` every day: the day he arrived, or for a
/// loanee the day his loan began.
///
/// A player the club has loaned out trains at the borrowing club, so its staff are not watching
/// him: while he is away there is none, and on his return the count starts again from the day he
/// came back (maintainer, B3-Q3).
fn trains_with_club_since<'a>(player: &'a Player, club: &str) -> Option<&'a str> {
    if player.team_id.as_deref() != Some(club) {
        return None;
    }
    if let Some(loan) = player.active_loan.as_ref()
        && loan.loan_team_id == club
    {
        return Some(&loan.start_date);
    }
    let last_return = player
        .movement_history
        .iter()
        .filter(|entry| {
            entry.kind == PlayerMovementKind::LoanReturn
                && entry.to_team_id.as_deref() == Some(club)
        })
        .map(|entry| entry.date.as_str())
        .max();
    let arrived = player
        .joined_current_club_on()
        .map(|arrival| opening_contract_start(player, club, arrival).unwrap_or(arrival));
    // ISO dates order like the days they name, so the later of the two is the larger string.
    arrived.into_iter().chain(last_return).max()
}

/// When the arrival on `arrival` is the contract a career opened with, the day that contract
/// began. The opening entry is dated the career's first day, but a player already at the club
/// joined when his contract began, so his coaches have known him since then, not since the
/// career opened. None for any other arrival.
fn opening_contract_start<'a>(player: &'a Player, club: &str, arrival: &str) -> Option<&'a str> {
    player
        .movement_history
        .iter()
        .find(|entry| {
            entry.kind == PlayerMovementKind::InitialContract
                && entry.date == arrival
                && entry.to_team_id.as_deref() == Some(club)
        })?
        .contract
        .as_ref()?
        .start
        .as_deref()
}

fn scouts_opinion(game: &Game, player: &Player) -> Option<PotentialOpinion> {
    let report = game
        .scouted_players
        .iter()
        .find(|report| report.player_id == player.id)?;
    let range = report.potential?;
    Some(PotentialOpinion::of(
        range,
        OpinionSource::Scout {
            scout_name: report.scout_name.clone(),
            scouted_on: report.scouted_on.clone(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameClock;
    use crate::test_support::uniform_attributes;
    use chrono::{TimeZone, Utc};
    use domain::league::League;
    use domain::manager::Manager;
    use domain::player::{ActiveLoan, PlayerMovementEntry, Position};
    use domain::staff::{Staff, StaffAttributes, StaffRole};

    const MY_CLUB: &str = "mine";

    fn player(id: &str, team_id: Option<&str>) -> Player {
        let mut player = Player::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "2000-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            uniform_attributes(60),
        );
        player.team_id = team_id.map(str::to_string);
        player
    }

    fn loan(parent: &str, borrower: &str) -> ActiveLoan {
        ActiveLoan {
            parent_team_id: parent.to_string(),
            loan_team_id: borrower.to_string(),
            start_date: "2026-07-01".to_string(),
            end_date: "2027-06-30".to_string(),
            wage_contribution_pct: 100,
            buy_option_fee: None,
            loan_start_minutes: 0,
            loan_start_appearances: 0,
            development_reported_minutes: 0,
            development_reported_appearances: 0,
        }
    }

    fn division(id: &str, clubs: &[&str]) -> League {
        let clubs: Vec<String> = clubs.iter().map(|club| club.to_string()).collect();
        League::new(id.to_string(), id.to_string(), 2026, &clubs)
    }

    /// A knockout cup: a competition that is not anyone's division.
    fn cup(id: &str, clubs: &[&str]) -> League {
        let mut cup = division(id, clubs);
        cup.kind = domain::league::CompetitionType::Cup;
        cup.rules.format = domain::league::CompetitionFormat::Knockout;
        cup
    }

    /// My club and "rival" play in division one; "faraway" plays in division two.
    fn world(manager_club: Option<&str>, players: Vec<Player>) -> Game {
        let mut manager = Manager::new(
            "m1".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "GB".to_string(),
        );
        manager.team_id = manager_club.map(str::to_string);
        let clock = GameClock::new(Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap());
        let mut game = Game::new(clock, manager, vec![], players, vec![], vec![]);
        game.competitions = vec![
            division("division-one", &[MY_CLUB, "rival"]),
            division("division-two", &["faraway", "other"]),
        ];
        game
    }

    fn scouted(game: &mut Game, player_id: &str, scouted_on: &str) {
        game.scouted_players.push(ScoutedPlayer {
            player_id: player_id.to_string(),
            scouted_on: scouted_on.to_string(),
            attributes: uniform_attributes(60),
            potential: None,
            scout_name: None,
        });
    }

    fn knowledge_of(game: &Game, player_id: &str) -> PlayerKnowledge {
        let player = game.players.iter().find(|p| p.id == player_id).unwrap();
        player_knowledge(game, player)
    }

    /// Given a player at the manager's club
    /// When the club's knowledge of him is asked
    /// Then he is the club's own
    #[test]
    fn a_player_at_the_club_is_its_own() {
        let game = world(Some(MY_CLUB), vec![player("p", Some(MY_CLUB))]);

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Own);
    }

    /// Given a player the club has loaned out to a club in another division
    /// When the club's knowledge of him is asked
    /// Then he is still its own: it holds his contract
    #[test]
    fn a_player_the_club_loaned_out_is_still_its_own() {
        let mut loanee = player("p", Some("faraway"));
        loanee.active_loan = Some(loan(MY_CLUB, "faraway"));
        let game = world(Some(MY_CLUB), vec![loanee]);

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Own);
    }

    /// Given a player another division's club has loaned to the manager's club
    /// When the club's knowledge of him is asked
    /// Then he is its own: he trains with it every day
    #[test]
    fn a_player_loaned_in_is_the_clubs_own() {
        let mut loanee = player("p", Some(MY_CLUB));
        loanee.active_loan = Some(loan("faraway", MY_CLUB));
        let game = world(Some(MY_CLUB), vec![loanee]);

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Own);
    }

    /// Given a player at a club in the manager's own division
    /// When the club's knowledge of him is asked
    /// Then he is known, without a scout
    #[test]
    fn a_rival_in_the_same_division_is_known() {
        let game = world(Some(MY_CLUB), vec![player("p", Some("rival"))]);

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Known);
    }

    /// Given a player at a club in another division, never scouted
    /// When the club's knowledge of him is asked
    /// Then nothing is known
    #[test]
    fn a_player_in_another_division_is_unknown() {
        let game = world(Some(MY_CLUB), vec![player("p", Some("faraway"))]);

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Unknown);
    }

    /// Given "mover", still in division one's standings but now playing division two (its table
    ///      with the later finished fixture, which is the one a club's division is read from)
    /// When the club's knowledge of a mover player is asked
    /// Then he is unknown: a club is known by the division it plays in now, not by a table it
    ///      still appears in
    #[test]
    fn a_club_listed_in_the_division_but_playing_another_is_not_known() {
        let mut game = world(Some(MY_CLUB), vec![player("m", Some("mover"))]);
        game.competitions[0]
            .standings
            .push(domain::league::StandingEntry {
                team_id: "mover".to_string(),
                played: 0,
                won: 0,
                drawn: 0,
                lost: 0,
                goals_for: 0,
                goals_against: 0,
                points: 0,
            });
        game.competitions[1]
            .participant_ids
            .push("mover".to_string());
        fixture_in(&mut game, "division-two", "2026-09-01", "mover", "faraway");
        game.competitions[1].fixtures[0].status = domain::league::FixtureStatus::Completed;

        assert_eq!(knowledge_of(&game, "m"), PlayerKnowledge::Unknown);
    }

    /// Given a player in another division whom a scout reported on
    /// When the club's knowledge of him is asked
    /// Then it is the scout's report, dated, current or not exactly as the report itself says
    #[test]
    fn a_scouted_player_is_known_through_the_report_and_its_own_verdict() {
        for scouted_on in ["2026-03-10", "2026-09-01"] {
            let mut game = world(Some(MY_CLUB), vec![player("p", Some("faraway"))]);
            game.season_context.season_start = Some("2026-08-01".to_string());
            scouted(&mut game, "p", scouted_on);
            let report_is_out_of_date =
                crate::slices::scouted_report::query_scouted_report(&game, "p")
                    .unwrap()
                    .out_of_date;

            assert_eq!(
                knowledge_of(&game, "p"),
                PlayerKnowledge::Scouted {
                    scouted_on: scouted_on.to_string(),
                    current: !report_is_out_of_date,
                }
            );
        }
    }

    /// Given three players in another division after the season ended on 25 May: one scouted in
    ///       March, before it, one scouted in September, after it, and one never scouted
    /// When one view is asked about each of them, as a list asks about every row
    /// Then each is answered from his own report: out of date, current, and unknown
    #[test]
    fn one_view_answers_each_scouted_player_from_his_own_report() {
        let mut game = world(
            Some(MY_CLUB),
            vec![
                player("stale", Some("faraway")),
                player("fresh", Some("faraway")),
                player("never", Some("faraway")),
            ],
        );
        game.last_season_rollover_on = Some("2026-05-25".to_string());
        scouted(&mut game, "stale", "2026-03-10");
        scouted(&mut game, "fresh", "2026-09-01");
        let view = KnowledgeView::of(&game);
        let knowledge_of =
            |id: &str| view.knowledge(game.players.iter().find(|p| p.id == id).unwrap());

        assert_eq!(
            knowledge_of("stale"),
            PlayerKnowledge::Scouted {
                scouted_on: "2026-03-10".to_string(),
                current: false,
            }
        );
        assert_eq!(
            knowledge_of("fresh"),
            PlayerKnowledge::Scouted {
                scouted_on: "2026-09-01".to_string(),
                current: true,
            }
        );
        assert_eq!(knowledge_of("never"), PlayerKnowledge::Unknown);
    }

    /// Given a free agent, never scouted, and one a scout reported on
    /// When the club's knowledge of each is asked
    /// Then the first is unknown (no club puts him in anyone's division) and the second is scouted
    #[test]
    fn a_free_agent_is_unknown_unless_scouted() {
        let mut game = world(
            Some(MY_CLUB),
            vec![player("free", None), player("free-scouted", None)],
        );
        scouted(&mut game, "free-scouted", "2026-09-01");

        assert_eq!(knowledge_of(&game, "free"), PlayerKnowledge::Unknown);
        assert!(matches!(
            knowledge_of(&game, "free-scouted"),
            PlayerKnowledge::Scouted { .. }
        ));
    }

    /// Given an unemployed manager
    /// When his knowledge of a player in division one, and of a scouted one, is asked
    /// Then no division is his, so the first is unknown, and the scouted one is still scouted
    #[test]
    fn an_unemployed_manager_knows_only_what_was_scouted() {
        let mut game = world(
            None,
            vec![player("p", Some("rival")), player("s", Some("faraway"))],
        );
        scouted(&mut game, "s", "2026-09-01");

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Unknown);
        assert!(matches!(
            knowledge_of(&game, "s"),
            PlayerKnowledge::Scouted { .. }
        ));
    }

    /// Given two clubs in no league table, sharing only the old single league a legacy save keeps
    /// When the knowledge of the other club's player is asked
    /// Then the old league does not stand in for a division: he is unknown
    #[test]
    fn a_club_outside_every_table_knows_no_division() {
        let mut game = world(Some("homeless"), vec![player("p", Some("neighbour"))]);
        game.league = Some(division("legacy", &["homeless", "neighbour"]));

        assert_eq!(knowledge_of(&game, "p"), PlayerKnowledge::Unknown);
    }

    /// Given the manager's club, a rival in his division and a club in another division
    /// When the view is asked whether the club knows each squad
    /// Then it knows its own and its rival's, and not the other division's
    #[test]
    fn the_club_knows_its_own_squad_and_its_division_rivals() {
        let game = world(Some(MY_CLUB), vec![]);
        let view = KnowledgeView::of(&game);

        assert!(view.knows_squad_at(MY_CLUB));
        assert!(view.knows_squad_at("rival"));
        assert!(!view.knows_squad_at("faraway"));
    }

    /// Given a club in no league table, as a legacy save can have
    /// When the view is asked whether the club knows its own squad
    /// Then it does: no division is needed to know one's own players
    #[test]
    fn a_club_outside_every_table_still_knows_its_own_squad() {
        let game = world(Some("homeless"), vec![]);
        let view = KnowledgeView::of(&game);

        assert!(view.knows_squad_at("homeless"));
        assert!(!view.knows_squad_at("rival"));
    }

    /// Given a manager without a club
    /// When the view is asked whether the club knows any squad
    /// Then it knows none
    #[test]
    fn an_unemployed_manager_knows_no_squad() {
        let game = world(None, vec![]);
        let view = KnowledgeView::of(&game);

        for club in [MY_CLUB, "rival", "faraway"] {
            assert!(!view.knows_squad_at(club), "{club}");
        }
    }

    fn paid(mut player: Player, weekly: u32) -> Player {
        player.stage_wage(weekly);
        player
    }

    /// Given the club's own player on 15,843 a week
    /// When the wage it may see is asked
    /// Then it is the exact figure
    #[test]
    fn the_clubs_own_player_shows_his_exact_wage() {
        let game = world(
            Some(MY_CLUB),
            vec![paid(player("p", Some(MY_CLUB)), 15_843)],
        );
        let player = &game.players[0];

        assert_eq!(
            visible_wage(&game, player),
            VisibleWage::Exact { weekly: 15_843 }
        );
    }

    /// Given a player in another division on 15,843 a week, whom a scout reported on
    /// When the wage the club may see is asked
    /// Then it is about 16,000: two significant figures, never the contract's figure
    #[test]
    fn a_scouted_player_shows_an_approximate_wage() {
        let mut game = world(
            Some(MY_CLUB),
            vec![paid(player("p", Some("faraway")), 15_843)],
        );
        scouted(&mut game, "p", "2026-09-01");
        let player = &game.players[0];

        assert_eq!(
            visible_wage(&game, player),
            VisibleWage::Approximate { weekly: 16_000 }
        );
    }

    /// Given a rival in the manager's own division, and a player elsewhere nobody scouted
    /// When the wage the club may see is asked
    /// Then neither shows one: a known rival's wage is his club's business
    #[test]
    fn a_known_or_unknown_players_wage_is_hidden() {
        let game = world(
            Some(MY_CLUB),
            vec![
                paid(player("known", Some("rival")), 15_843),
                paid(player("unknown", Some("faraway")), 15_843),
            ],
        );

        assert_eq!(visible_wage(&game, &game.players[0]), VisibleWage::Hidden);
        assert_eq!(visible_wage(&game, &game.players[1]), VisibleWage::Hidden);
    }

    /// Given wages of 999, 1,250, 12,345 and 7 a week
    /// When they are rounded for a scout's ear
    /// Then they keep two significant figures: 1,000, 1,300, 12,000 and 7
    #[test]
    fn an_approximate_wage_keeps_two_significant_figures() {
        let rounded = |weekly| {
            let mut game = world(Some(MY_CLUB), vec![paid(player("p", None), weekly)]);
            scouted(&mut game, "p", "2026-09-01");
            visible_wage(&game, &game.players[0])
        };

        assert_eq!(rounded(999), VisibleWage::Approximate { weekly: 1_000 });
        assert_eq!(rounded(1_250), VisibleWage::Approximate { weekly: 1_300 });
        assert_eq!(rounded(12_345), VisibleWage::Approximate { weekly: 12_000 });
        assert_eq!(rounded(7), VisibleWage::Approximate { weekly: 7 });
    }

    /// Given a request for several players, one of them an id no player has
    /// When their knowledge is asked in one go
    /// Then each real player is answered and the unknown id is left out
    #[test]
    fn a_bulk_request_answers_each_player_and_skips_unknown_ids() {
        let game = world(
            Some(MY_CLUB),
            vec![
                player("own", Some(MY_CLUB)),
                player("rival-player", Some("rival")),
            ],
        );
        let ids = ["own", "rival-player", "nobody"].map(str::to_string);

        let answer = players_knowledge(&game, &ids);

        assert_eq!(answer.len(), 2);
        assert_eq!(answer["own"], PlayerKnowledge::Own);
        assert_eq!(answer["rival-player"], PlayerKnowledge::Known);
    }

    /// A player on `condition`.
    fn tired(mut player: Player, condition: u8) -> Player {
        player.condition = condition;
        player
    }

    /// Schedules `home` against `away` in division one on `date`.
    fn fixture_on(game: &mut Game, date: &str, home: &str, away: &str) {
        fixture_in(game, "division-one", date, home, away);
    }

    /// Schedules `home` against `away` in `competition_id` on `date`.
    fn fixture_in(game: &mut Game, competition_id: &str, date: &str, home: &str, away: &str) {
        let competition = game
            .competitions
            .iter_mut()
            .find(|competition| competition.id == competition_id)
            .unwrap();
        competition.fixtures.push(domain::league::Fixture {
            id: format!("{home}-{away}-{date}"),
            competition_id: competition_id.to_string(),
            matchday: 1,
            date: date.to_string(),
            home_team_id: home.to_string(),
            away_team_id: away.to_string(),
            competition: domain::league::FixtureCompetition::League,
            status: domain::league::FixtureStatus::Scheduled,
            result: None,
            postponement: None,
        });
    }

    fn condition_of(game: &Game, player_id: &str) -> Option<u8> {
        let player = game.players.iter().find(|p| p.id == player_id).unwrap();
        visible_condition(game, player)
    }

    /// Given the club's own player on 64% condition, with no match today
    /// When his condition is asked
    /// Then it shows: the club's own player's condition is always the club's to see
    #[test]
    fn the_clubs_own_players_condition_always_shows() {
        let game = world(Some(MY_CLUB), vec![tired(player("p", Some(MY_CLUB)), 64)]);

        assert_eq!(condition_of(&game, "p"), Some(64));
    }

    /// Given "rival" playing the club today (15 Sep, away), and one of rival's players on 71%
    /// When his condition is asked
    /// Then it shows, as the team sheet and the warm-up would show it
    #[test]
    fn an_opponents_condition_shows_on_the_day_they_meet() {
        let mut game = world(Some(MY_CLUB), vec![tired(player("r", Some("rival")), 71)]);
        fixture_on(&mut game, "2026-09-15", "rival", MY_CLUB);

        assert_eq!(condition_of(&game, "r"), Some(71));
    }

    /// Given the club drawn at home today against "faraway", a club from another division, in a
    ///      cup (a competition other than the club's division), and one of faraway's players on 58%
    /// When his condition is asked
    /// Then it shows: any competition's matchday hands over the team sheet, not the league's alone
    #[test]
    fn an_opponents_condition_shows_on_a_cup_tie_too() {
        let mut game = world(Some(MY_CLUB), vec![tired(player("f", Some("faraway")), 58)]);
        game.competitions
            .push(cup("cup", &[MY_CLUB, "rival", "faraway", "other"]));
        fixture_in(&mut game, "cup", "2026-09-15", MY_CLUB, "faraway");

        assert_eq!(condition_of(&game, "f"), Some(58));
    }

    /// Given "rival" playing the club tomorrow, not today
    /// When one of rival's players' condition is asked today
    /// Then it does not show: a division rival is known, but his condition is his club's business
    #[test]
    fn an_opponents_condition_is_hidden_before_matchday() {
        let mut game = world(Some(MY_CLUB), vec![tired(player("r", Some("rival")), 71)]);
        fixture_on(&mut game, "2026-09-16", MY_CLUB, "rival");

        assert_eq!(condition_of(&game, "r"), None);
    }

    /// Given the club meeting "rival" today
    /// When a player of "faraway", which the club does not meet, is asked about
    /// Then his condition stays hidden: a matchday hands over the opponent's team sheet only, not
    ///      every club's
    #[test]
    fn a_matchday_shows_only_the_opponents_condition() {
        let mut game = world(Some(MY_CLUB), vec![tired(player("f", Some("faraway")), 71)]);
        fixture_on(&mut game, "2026-09-15", MY_CLUB, "rival");

        assert_eq!(condition_of(&game, "f"), None);
    }

    /// Given "rival" and "faraway" meeting each other today, the club not playing them
    /// When a "faraway" player's condition is asked
    /// Then it does not show: only the club's own opponent's team sheet is in its hands
    #[test]
    fn a_matchday_between_two_other_clubs_shows_nothing() {
        let mut game = world(Some(MY_CLUB), vec![tired(player("f", Some("faraway")), 71)]);
        fixture_on(&mut game, "2026-09-15", "rival", "faraway");

        assert_eq!(condition_of(&game, "f"), None);
    }

    /// Given a manager with no club, and "rival" playing "faraway" today
    /// When a rival player's condition is asked
    /// Then it does not show: a manager without a club has no opponent
    #[test]
    fn an_unemployed_manager_sees_no_opponents_condition() {
        let mut game = world(None, vec![tired(player("r", Some("rival")), 71)]);
        fixture_on(&mut game, "2026-09-15", "rival", "faraway");

        assert_eq!(condition_of(&game, "r"), None);
    }

    /// A player rated `ovr` on `weekly` a week and 64% condition.
    fn rated(id: &str, team_id: Option<&str>, ovr: u8, weekly: u32) -> Player {
        let mut player = tired(paid(player(id, team_id), weekly), 64);
        player.ovr = ovr;
        player
    }

    fn visibility_of(game: &Game, player_id: &str) -> PlayerVisibility {
        let player = game.players.iter().find(|p| p.id == player_id).unwrap();
        KnowledgeView::of(game).of_player(player)
    }

    /// Given the club's own player, rated 74 on 15,843 a week and 64% condition
    /// When a list row asks what it may show
    /// Then everything: his rating, his exact wage and his condition
    #[test]
    fn a_list_row_shows_everything_about_the_clubs_own_player() {
        let game = world(Some(MY_CLUB), vec![rated("p", Some(MY_CLUB), 74, 15_843)]);

        assert_eq!(
            visibility_of(&game, "p"),
            PlayerVisibility {
                knowledge: PlayerKnowledge::Own,
                ovr: Some(74),
                wage: VisibleWage::Exact { weekly: 15_843 },
                condition: Some(64),
            }
        );
    }

    /// Given a division rival rated 74
    /// When a list row asks what it may show
    /// Then his rating, but not his wage or condition
    #[test]
    fn a_list_row_shows_a_known_rivals_rating_only() {
        let game = world(Some(MY_CLUB), vec![rated("r", Some("rival"), 74, 15_843)]);

        assert_eq!(
            visibility_of(&game, "r"),
            PlayerVisibility {
                knowledge: PlayerKnowledge::Known,
                ovr: Some(74),
                wage: VisibleWage::Hidden,
                condition: None,
            }
        );
    }

    /// Given a player in another division whom a scout reported on
    /// When a list row asks what it may show
    /// Then his wage as about the figure, but no rating (until a scout's estimate exists) and no
    ///      condition
    #[test]
    fn a_list_row_shows_a_scouted_players_approximate_wage_only() {
        let mut game = world(Some(MY_CLUB), vec![rated("f", Some("faraway"), 74, 15_843)]);
        scouted(&mut game, "f", "2026-09-01");

        let visibility = visibility_of(&game, "f");

        assert_eq!(visibility.ovr, None);
        assert_eq!(visibility.wage, VisibleWage::Approximate { weekly: 16_000 });
        assert_eq!(visibility.condition, None);
    }

    /// Given a player in another division, never scouted
    /// When a list row asks what it may show
    /// Then nothing beyond what is public
    #[test]
    fn a_list_row_shows_nothing_private_about_an_unknown_player() {
        let game = world(Some(MY_CLUB), vec![rated("f", Some("faraway"), 74, 15_843)]);

        assert_eq!(
            visibility_of(&game, "f"),
            PlayerVisibility {
                knowledge: PlayerKnowledge::Unknown,
                ovr: None,
                wage: VisibleWage::Hidden,
                condition: None,
            }
        );
    }

    /// Given the club meeting "faraway" today
    /// When a list row asks what it may show about one of faraway's players
    /// Then his condition, as the team sheet shows it, but still no rating or wage
    #[test]
    fn a_list_row_shows_an_opponents_condition_on_matchday() {
        let mut game = world(Some(MY_CLUB), vec![rated("f", Some("faraway"), 74, 15_843)]);
        game.competitions.push(cup("cup", &[MY_CLUB, "faraway"]));
        fixture_in(&mut game, "cup", "2026-09-15", "faraway", MY_CLUB);

        let visibility = visibility_of(&game, "f");

        assert_eq!(visibility.condition, Some(64));
        assert_eq!(visibility.ovr, None);
        assert_eq!(visibility.wage, VisibleWage::Hidden);
    }

    /// Given an exact, an approximate and a hidden wage
    /// When their figure is asked
    /// Then the first two give theirs and the hidden one gives none
    #[test]
    fn a_visible_wage_gives_its_figure_unless_hidden() {
        assert_eq!(VisibleWage::Exact { weekly: 15_843 }.value(), Some(15_843));
        assert_eq!(
            VisibleWage::Approximate { weekly: 16_000 }.value(),
            Some(16_000)
        );
        assert_eq!(VisibleWage::Hidden.value(), None);
    }

    /// Given a min/max rating filter
    /// When a known and an unknown rating are tested against it
    /// Then the known one passes inside the bounds and fails outside; the unknown one fails any set
    ///      bound and passes only when none is set
    #[test]
    fn an_unknown_rating_fails_any_set_rating_bound() {
        assert!(passes_ovr_filter(Some(74), Some(70), Some(80)));
        assert!(!passes_ovr_filter(Some(69), Some(70), None));
        assert!(!passes_ovr_filter(Some(81), None, Some(80)));
        assert!(!passes_ovr_filter(None, Some(1), None));
        assert!(!passes_ovr_filter(None, None, Some(99)));
        assert!(passes_ovr_filter(None, None, None));
    }

    /// Given a maximum wage filter
    /// When exact, approximate and hidden wages are tested against it
    /// Then a visible figure passes at or under the maximum; a hidden wage fails any set maximum
    ///      and passes only when none is set
    #[test]
    fn a_hidden_wage_fails_a_set_wage_maximum() {
        assert!(passes_wage_filter(
            &VisibleWage::Exact { weekly: 15_000 },
            Some(15_000)
        ));
        assert!(!passes_wage_filter(
            &VisibleWage::Approximate { weekly: 16_000 },
            Some(15_000)
        ));
        assert!(!passes_wage_filter(&VisibleWage::Hidden, Some(1_000_000)));
        assert!(passes_wage_filter(&VisibleWage::Hidden, None));
    }

    /// Given ratings 80, unknown and 70
    /// When they are sorted by rating, descending and then ascending
    /// Then the known ones run in that direction and the unknown one comes last both times
    #[test]
    fn an_unknown_rating_sorts_after_every_known_one_either_way() {
        let mut ratings = vec![Some(70), None, Some(80)];

        ratings.sort_by(|a, b| compare_visible_ovr(*a, *b, false));
        assert_eq!(ratings, vec![Some(80), Some(70), None]);

        ratings.sort_by(|a, b| compare_visible_ovr(*a, *b, true));
        assert_eq!(ratings, vec![Some(70), Some(80), None]);
    }

    // The world's date is 15 Sep 2026: 4 Aug is six weeks (42 days) before it, 5 Aug a day short.
    const SIX_WEEKS_AGO: &str = "2026-08-04";
    const A_DAY_SHORT_OF_SIX_WEEKS: &str = "2026-08-05";

    /// `player`, his potential `potential`, after he arrived at `club` on `date`.
    fn arrived(mut player: Player, club: &str, date: &str, potential: u8) -> Player {
        player.potential = potential;
        player
            .record_movement(PlayerMovementEntry {
                to_team_id: Some(club.to_string()),
                ..PlayerMovementEntry::new(date, PlayerMovementKind::PermanentTransfer)
            })
            .unwrap();
        player
    }

    /// A coach at `club` whose eye for potential is `judging_potential`.
    fn staff_at(id: &str, club: &str, judging_potential: u8) -> Staff {
        let mut staff = Staff::new(
            id.to_string(),
            id.to_string(),
            id.to_string(),
            "1970-01-01".to_string(),
            StaffRole::Coach,
            StaffAttributes {
                coaching: 50,
                judging_ability: 50,
                judging_potential,
                physiotherapy: 50,
            },
        );
        staff.team_id = Some(club.to_string());
        staff
    }

    fn opinion_of(game: &Game, player_id: &str) -> Option<PotentialOpinion> {
        let player = game.players.iter().find(|p| p.id == player_id).unwrap();
        potential_opinion(game, player)
    }

    fn width(opinion: &PotentialOpinion) -> u8 {
        opinion.high - opinion.low
    }

    /// Given a player in another division whose scout, Ann Scout, wrote 76-84 on 1 Sep
    /// When the club's opinion of his potential is asked
    /// Then it is her range, with its band (the tier of 80, A), how sure she was, and her name
    ///      and date
    #[test]
    fn a_scouted_player_shows_the_opinion_his_scout_wrote() {
        let mut far = player("far", Some("faraway"));
        far.potential = 80;
        let mut game = world(Some(MY_CLUB), vec![far]);
        game.scouted_players.push(ScoutedPlayer {
            player_id: "far".to_string(),
            scouted_on: "2026-09-01".to_string(),
            attributes: uniform_attributes(60),
            potential: Some(crate::game::PotentialRange { low: 76, high: 84 }),
            scout_name: Some("Ann Scout".to_string()),
        });

        assert_eq!(
            opinion_of(&game, "far"),
            Some(PotentialOpinion {
                low: 76,
                high: 84,
                band: crate::player_tier::PlayerTier::A,
                certainty: crate::potential_opinion::Certainty::FairlySure,
                source: OpinionSource::Scout {
                    scout_name: Some("Ann Scout".to_string()),
                    scouted_on: "2026-09-01".to_string(),
                },
            })
        );
    }

    /// Given a player scouted before reports carried an opinion of his potential
    /// When the club's opinion is asked
    /// Then there is none, until he is scouted again
    #[test]
    fn a_report_written_before_opinions_shows_none() {
        let mut far = player("far", Some("faraway"));
        far.potential = 80;
        let mut game = world(Some(MY_CLUB), vec![far]);
        scouted(&mut game, "far", "2026-09-01");

        assert_eq!(opinion_of(&game, "far"), None);
    }

    /// Given a rival in the manager's division and a player in another, neither scouted
    /// When the club's opinion of each one's potential is asked
    /// Then there is none: knowing a rival's rating is not knowing his potential
    #[test]
    fn a_known_rival_or_an_unscouted_player_shows_no_opinion() {
        let mut rival = player("rival-player", Some("rival"));
        rival.potential = 80;
        let mut far = player("far", Some("faraway"));
        far.potential = 80;
        let mut game = world(Some(MY_CLUB), vec![rival, far]);
        game.staff = vec![staff_at("coach", MY_CLUB, 90)];

        assert_eq!(opinion_of(&game, "rival-player"), None);
        assert_eq!(opinion_of(&game, "far"), None);
    }

    /// Given the club's own player who arrived a day short of six weeks ago, and a coach
    /// When the club's opinion of his potential is asked
    /// Then there is none yet: the staff have not watched him long enough
    #[test]
    fn the_clubs_own_player_has_no_opinion_before_six_weeks_with_it() {
        let own = arrived(
            player("own", Some(MY_CLUB)),
            MY_CLUB,
            A_DAY_SHORT_OF_SIX_WEEKS,
            80,
        );
        let mut game = world(Some(MY_CLUB), vec![own]);
        game.staff = vec![staff_at("coach", MY_CLUB, 90)];

        assert_eq!(opinion_of(&game, "own"), None);
    }

    /// Given the club's own player who arrived six weeks ago, his true potential 80
    /// When the club's opinion of his potential is asked
    /// Then it is the coaches' opinion, and its range holds his true potential
    #[test]
    fn after_six_weeks_the_clubs_own_player_has_the_coaches_opinion() {
        let own = arrived(player("own", Some(MY_CLUB)), MY_CLUB, SIX_WEEKS_AGO, 80);
        let mut game = world(Some(MY_CLUB), vec![own]);
        game.staff = vec![staff_at("coach", MY_CLUB, 90)];

        let opinion = opinion_of(&game, "own").expect("six weeks of training");

        assert_eq!(opinion.source, OpinionSource::Coaches);
        assert!(opinion.low <= 80 && 80 <= opinion.high, "{opinion:?}");
    }

    /// Given the club's own player, its two coaches with eyes of 30 and 70, and a 95 at another club
    /// When the club's opinion of his potential is asked
    /// Then it is the 70's: 8 points wide, the club's best eye and nobody else's
    #[test]
    fn the_coaches_opinion_comes_from_the_clubs_best_eye() {
        let own = arrived(player("own", Some(MY_CLUB)), MY_CLUB, SIX_WEEKS_AGO, 80);
        let mut game = world(Some(MY_CLUB), vec![own]);
        game.staff = vec![
            staff_at("poor", MY_CLUB, 30),
            staff_at("good", MY_CLUB, 70),
            staff_at("elsewhere", "rival", 95),
        ];

        assert_eq!(width(&opinion_of(&game, "own").unwrap()), 8);
    }

    /// Given the club's own player of six weeks and more, scouted before he arrived
    /// When the club's opinion of his potential is asked
    /// Then it is the coaches', who see him every day, not the old report's
    #[test]
    fn the_coaches_opinion_wins_over_a_scouts_report_for_the_clubs_own_player() {
        let own = arrived(player("own", Some(MY_CLUB)), MY_CLUB, SIX_WEEKS_AGO, 80);
        let mut game = world(Some(MY_CLUB), vec![own]);
        game.staff = vec![staff_at("coach", MY_CLUB, 90)];
        game.scouted_players.push(ScoutedPlayer {
            player_id: "own".to_string(),
            scouted_on: "2026-01-10".to_string(),
            attributes: uniform_attributes(60),
            potential: Some(crate::game::PotentialRange { low: 70, high: 88 }),
            scout_name: Some("Ann Scout".to_string()),
        });

        assert_eq!(
            opinion_of(&game, "own").unwrap().source,
            OpinionSource::Coaches
        );
    }

    /// Given the club's own player of six weeks and more, at a club with no staff
    /// When the club's opinion of his potential is asked
    /// Then there is none: nobody at the club judges potential
    #[test]
    fn a_club_with_no_staff_holds_no_opinion() {
        let own = arrived(player("own", Some(MY_CLUB)), MY_CLUB, SIX_WEEKS_AGO, 80);
        let game = world(Some(MY_CLUB), vec![own]);

        assert_eq!(opinion_of(&game, "own"), None);
    }

    /// Given a player scouted with a range, whose potential is not stored (0)
    /// When the club's opinion of his potential is asked
    /// Then there is none: there is nothing for the opinion to be about
    #[test]
    fn a_player_with_no_stored_potential_has_no_opinion() {
        let far = player("far", Some("faraway"));
        let mut game = world(Some(MY_CLUB), vec![far]);
        game.scouted_players.push(ScoutedPlayer {
            player_id: "far".to_string(),
            scouted_on: "2026-09-01".to_string(),
            attributes: uniform_attributes(60),
            potential: Some(crate::game::PotentialRange { low: 76, high: 84 }),
            scout_name: None,
        });

        assert_eq!(game.players[0].potential, 0);
        assert_eq!(opinion_of(&game, "far"), None);
    }

    /// Given a player on loan at the club since 1 Jul, and one whose loan began on 1 Sep
    /// When the club's opinion of each one's potential is asked
    /// Then the first, past six weeks of his loan, has the coaches' opinion, and the second none
    #[test]
    fn a_player_on_loan_at_the_club_counts_six_weeks_from_his_loan_start() {
        let mut settled = player("settled", Some(MY_CLUB));
        settled.potential = 80;
        settled.active_loan = Some(loan("rival", MY_CLUB));
        let mut fresh = player("fresh", Some(MY_CLUB));
        fresh.potential = 80;
        fresh.active_loan = Some(ActiveLoan {
            start_date: "2026-09-01".to_string(),
            ..loan("rival", MY_CLUB)
        });
        let mut game = world(Some(MY_CLUB), vec![settled, fresh]);
        game.staff = vec![staff_at("coach", MY_CLUB, 90)];

        assert_eq!(
            opinion_of(&game, "settled").map(|opinion| opinion.source),
            Some(OpinionSource::Coaches)
        );
        assert_eq!(opinion_of(&game, "fresh"), None);
    }

    /// Given the club's own player, at the club for years, now out on loan at a rival
    /// When the club's opinion of his potential is asked
    /// Then there is none: he trains at the borrowing club, where the club's staff can't watch him
    #[test]
    fn a_player_the_club_has_loaned_out_has_no_coaches_opinion_while_away() {
        let mut away = arrived(player("away", Some(MY_CLUB)), MY_CLUB, "2024-07-01", 80);
        away.team_id = Some("rival".to_string());
        away.active_loan = Some(loan(MY_CLUB, "rival"));
        let mut game = world(Some(MY_CLUB), vec![away]);
        game.staff = vec![staff_at("coach", MY_CLUB, 90)];

        assert_eq!(opinion_of(&game, "away"), None);
    }

    /// Given two of the club's own players back from loan, one since 20 Aug and one since 1 Jul
    /// When the club's opinion of each one's potential is asked
    /// Then the six weeks count from his return: the first has none yet, the second the coaches'
    #[test]
    fn a_player_back_from_loan_counts_six_weeks_from_his_return() {
        let back_from = |id: &str, returned_on: &str| {
            let mut back = arrived(player(id, Some(MY_CLUB)), MY_CLUB, "2024-07-01", 80);
            back.record_movement(PlayerMovementEntry {
                from_team_id: Some("rival".to_string()),
                to_team_id: Some(MY_CLUB.to_string()),
                ..PlayerMovementEntry::new(returned_on, PlayerMovementKind::LoanReturn)
            })
            .unwrap();
            back
        };
        let mut game = world(
            Some(MY_CLUB),
            vec![
                back_from("recent", "2026-08-20"),
                back_from("settled", "2026-07-01"),
            ],
        );
        game.staff = vec![staff_at("coach", MY_CLUB, 90)];

        assert_eq!(opinion_of(&game, "recent"), None);
        assert_eq!(
            opinion_of(&game, "settled").map(|opinion| opinion.source),
            Some(OpinionSource::Coaches)
        );
    }

    /// `player` at `club` when the career opened on `opened_on`, on a contract that began on
    /// `contract_start`, as `record_opening_contracts` writes it.
    fn there_when_the_career_opened(
        mut player: Player,
        club: &str,
        opened_on: &str,
        contract_start: &str,
    ) -> Player {
        player.potential = 80;
        player
            .record_movement(PlayerMovementEntry {
                to_team_id: Some(club.to_string()),
                contract: Some(domain::contract_ledger::ContractRecord {
                    start: Some(contract_start.to_string()),
                    end: Some("2029-06-30".to_string()),
                    weekly_wage: 5_000,
                    source: domain::contract_ledger::ContractSource::Initial,
                }),
                ..PlayerMovementEntry::new(opened_on, PlayerMovementKind::InitialContract)
            })
            .unwrap();
        player
    }

    /// Given a career that opened five days ago, its club's player on a contract that began in 2024,
    ///       and another whose contract began a week before the career opened
    /// When the club's opinion of each one's potential is asked
    /// Then the first has the coaches' opinion at once: he joined when his contract began, not on
    ///      the career's first day. The second, at the club for under six weeks, has none yet
    #[test]
    fn a_player_at_the_club_when_the_career_opened_counts_from_his_contract_start() {
        let mut game = world(
            Some(MY_CLUB),
            vec![
                there_when_the_career_opened(
                    player("veteran", Some(MY_CLUB)),
                    MY_CLUB,
                    "2026-09-10",
                    "2024-07-01",
                ),
                there_when_the_career_opened(
                    player("newcomer", Some(MY_CLUB)),
                    MY_CLUB,
                    "2026-09-10",
                    "2026-09-03",
                ),
            ],
        );
        game.staff = vec![staff_at("coach", MY_CLUB, 90)];

        assert_eq!(
            opinion_of(&game, "veteran").map(|opinion| opinion.source),
            Some(OpinionSource::Coaches)
        );
        assert_eq!(opinion_of(&game, "newcomer"), None);
    }

    /// Given eight of the club's own players, and two coaches with the same eye for potential,
    ///       listed in either order (as a reload may list them)
    /// When the club's opinion of each player's potential is asked
    /// Then it is the same range both ways, the first coach by id's: the judge between equals is
    ///      picked by id, not by the list's order, so the opinion never shifts on reload
    #[test]
    fn equally_good_coaches_give_the_same_opinion_in_any_order() {
        let ids: Vec<String> = (0..8).map(|n| format!("own-{n}")).collect();
        // Two judges agree on a player by chance about one time in nine; across eight players
        // they must differ somewhere, or this test could not tell which of them judged.
        assert!(
            ids.iter().any(
                |id| judged_range(id, "a-coach", 80, 70) != judged_range(id, "b-coach", 80, 70)
            ),
            "the two coaches give every player the same range"
        );
        let squad = ids
            .iter()
            .map(|id| arrived(player(id, Some(MY_CLUB)), MY_CLUB, SIX_WEEKS_AGO, 80))
            .collect();
        let mut game = world(Some(MY_CLUB), squad);
        game.staff = vec![
            staff_at("b-coach", MY_CLUB, 70),
            staff_at("a-coach", MY_CLUB, 70),
        ];
        let ranges = |game: &Game| -> Vec<(u8, u8)> {
            ids.iter()
                .map(|id| opinion_of(game, id).unwrap())
                .map(|opinion| (opinion.low, opinion.high))
                .collect()
        };
        let one_way = ranges(&game);
        game.staff.reverse();
        let other_way = ranges(&game);
        let first_by_id: Vec<(u8, u8)> = ids
            .iter()
            .map(|id| judged_range(id, "a-coach", 80, 70))
            .map(|range| (range.low, range.high))
            .collect();

        assert_eq!(one_way, other_way);
        assert_eq!(one_way, first_by_id, "the coach with the first id judges");
    }
}
