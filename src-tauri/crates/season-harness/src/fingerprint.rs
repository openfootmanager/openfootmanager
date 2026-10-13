//! A world reduced to what a replay must reproduce, for comparing two runs.

use domain::league::Fixture;
use domain::message::InboxMessage;
use domain::news::NewsArticle;
use domain::stats::StatsState;
use domain::team::Team;
use ofm_core::game::Game;

use crate::driver::SeasonObserver;

/// Everything a day's dice and a rollover's people decide, one line per fact, in a fixed order.
///
/// Not the whole `Game`: ids still minted at random for inbox messages, news and bids differ
/// between two runs and are not the dice. Fixture ids, player ids, results, tables, bodies,
/// money, staff and managers are all here, and so is what the daily subsystems decide for a
/// player (squad role, wage, contract end, a pending promise) and a club (tactics, training,
/// the starting eleven, per-player roles) and the inbox they write to, so a draw that escapes
/// the game's seed shows up as the first line two fingerprints disagree on (see
/// [`first_difference`]). Maps are written in key order, so no line depends on hashing.
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
            lines.push(format!(
                "  fixture {} {} {}-{} {:?} {}",
                fixture.id,
                fixture.date,
                fixture.home_team_id,
                fixture.away_team_id,
                fixture.status,
                result_of(fixture)
            ));
        }
    }
    for national in &game.national_teams {
        for fixture in &national.fixtures {
            lines.push(format!(
                "national {} {} {}-{} {:?} {}",
                fixture.id,
                fixture.date,
                fixture.home_team_id,
                fixture.away_team_id,
                fixture.status,
                result_of(fixture)
            ));
        }
    }
    for player in &game.players {
        lines.push(format!(
            "player {} team{:?} ovr{} pot{} cond{} fit{} morale{} inj{:?} retired{:?} {:?} role{:?} wage{} until{:?} morale_core{:?}",
            player.id,
            player.team_id,
            player.ovr,
            player.potential,
            player.condition,
            player.fitness,
            player.morale,
            player.injury,
            player.retired,
            player.stats,
            player.squad_role,
            player.wage(),
            player.contract_end(),
            player.morale_core
        ));
    }
    for team in &game.teams {
        lines.push(format!("team {} finance{}", team.id, team.finance));
        lines.push(team_decisions(team));
    }
    for message in &game.messages {
        lines.push(inbox_line(message));
    }
    for article in &game.news {
        lines.push(news_line(article));
    }
    for top_up in &game.squad_floor_top_ups {
        lines.push(format!("top_up {top_up:?}"));
    }
    lines.push(format!(
        "user_manager satisfaction{} fan_approval{}",
        game.manager.satisfaction, game.manager.fan_approval
    ));
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

/// What the club's tactics and training are set to, and who it starts.
fn team_decisions(team: &Team) -> String {
    let mut roles: Vec<_> = team.player_roles.iter().collect();
    roles.sort_unstable_by_key(|(player_id, _)| player_id.as_str());
    format!(
        "  decisions {} {} {:?} phase{:?} focus{:?} intensity{:?} schedule{:?} xi{:?} roles{:?} form{:?}",
        team.id,
        team.formation,
        team.play_style,
        team.tactics_phase,
        team.training_focus,
        team.training_intensity,
        team.training_schedule,
        team.starting_xi_ids,
        roles,
        team.form
    )
}

/// An article without its id, which is minted at random: everything it says, and the order the
/// articles are in, which is the order the day's matches were reported in.
fn news_line(article: &NewsArticle) -> String {
    let mut params: Vec<_> = article.i18n_params.iter().collect();
    params.sort_unstable();
    let mut without_id = article.clone();
    without_id.id.clear();
    // A `HashMap`: written in key order, like the inbox's.
    without_id.i18n_params.clear();
    format!("news {without_id:?} params{params:?}")
}

/// What the day's matches handed to the stats store (`on_capture`), which is not part of the game
/// but is written by the same code. A match's records come from a `HashMap`, so within one capture
/// they are put in player order; the order of the captures is the order the matches were applied
/// in and is kept, so a change to it is a change to the run.
#[derive(Debug, Default)]
pub struct StatsLog {
    lines: Vec<String>,
    captures: usize,
}

impl StatsLog {
    /// How many matches have been captured.
    pub fn captures(&self) -> usize {
        self.captures
    }

    /// Every capture seen, in the order the matches were applied: within a capture the player
    /// records by player id, then the team records as they came.
    pub fn fingerprint(&self) -> String {
        self.lines.join("\n")
    }
}

impl SeasonObserver for StatsLog {
    fn on_stats_captured(&mut self, stats: &StatsState) {
        let mut players: Vec<_> = stats.player_matches.iter().collect();
        players.sort_unstable_by(|a, b| a.player_id.cmp(&b.player_id));
        self.captures += 1;
        self.lines.push("capture".to_string());
        for record in players {
            self.lines.push(format!("  player_match {record:?}"));
        }
        for record in &stats.team_matches {
            self.lines.push(format!("  team_match {record:?}"));
        }
    }
}

/// A message without its id, which is minted at random: the day, who it is for and what it says.
fn inbox_line(message: &InboxMessage) -> String {
    let mut params: Vec<_> = message.i18n_params.iter().collect();
    params.sort_unstable();
    // Text fields in `{:?}` form, quoted and escaped, so no character inside one can read as the
    // boundary between two.
    format!(
        "message {} {:?} {:?} read{} from{:?}/{:?}/{:?}/{:?} {:?}/{:?} {:?} {:?} params{:?}",
        message.date,
        message.category,
        message.priority,
        message.read,
        message.sender,
        message.sender_role,
        message.sender_key,
        message.sender_role_key,
        message.subject_key,
        message.body_key,
        message.subject,
        message.body,
        params
    )
}

fn result_of(fixture: &Fixture) -> String {
    fixture.result.as_ref().map_or_else(
        || "unplayed".to_string(),
        |result| {
            format!(
                "{}-{} pens {:?}-{:?}",
                result.home_goals, result.away_goals, result.home_penalties, result.away_penalties
            )
        },
    )
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

#[cfg(test)]
mod tests {
    use domain::league::MatchResult;

    use domain::news::NewsArticle;
    use domain::stats::StatsState;

    use super::fingerprint;
    use crate::driver::SeasonObserver;
    use crate::worlds::WorldSpec;

    /// Given two worlds whose only difference is the penalty score of one drawn tie,
    /// When each is fingerprinted,
    /// Then the fingerprints differ — a shootout won on penalties is not the same result
    ///      as one lost, though regulation goals and the team advancing may agree.
    #[test]
    fn a_different_shootout_score_is_a_different_world() {
        let mut won = WorldSpec::gate(1).build().expect("the world builds");
        let fixture = &mut won.competitions[0].fixtures[0];
        fixture.result = Some(MatchResult {
            home_goals: 1,
            away_goals: 1,
            home_penalties: Some(4),
            away_penalties: Some(3),
            ..MatchResult::default()
        });
        let mut narrower = won.clone();
        let result = narrower.competitions[0].fixtures[0]
            .result
            .as_mut()
            .unwrap();
        result.home_penalties = Some(5);

        assert_ne!(fingerprint(&won), fingerprint(&narrower));
    }

    fn built() -> ofm_core::game::Game {
        WorldSpec::gate(1).build().expect("the world builds")
    }

    /// Given the same world built twice,
    /// When each is fingerprinted,
    /// Then the fingerprints are equal, so no field the fingerprint now covers depends on hashing
    ///      order or on a value minted at random.
    #[test]
    fn two_builds_of_one_world_have_one_fingerprint() {
        let (first, second) = (fingerprint(&built()), fingerprint(&built()));
        assert!(
            first == second,
            "{}",
            super::first_difference(&first, &second)
        );
    }

    /// Given a world, and the same world with one field of one player or club changed,
    /// When each is fingerprinted,
    /// Then they differ: the fingerprint sees that field.
    fn assert_sees(change: impl FnOnce(&mut ofm_core::game::Game)) {
        let before = built();
        let mut after = before.clone();
        change(&mut after);
        assert_ne!(fingerprint(&before), fingerprint(&after));
    }

    #[test]
    fn it_sees_a_players_squad_role() {
        assert_sees(|game| game.players[0].squad_role = domain::player::SquadRole::Youth);
    }

    /// Renews the first contracted player's deal with `change` applied to its terms, the way the
    /// game changes a contract: by recording a movement, since a recorded one cannot be staged over.
    fn renew_first_contracted_player(
        game: &mut ofm_core::game::Game,
        change: impl FnOnce(&mut domain::contract_ledger::ContractRecord),
    ) -> usize {
        let index = game
            .players
            .iter()
            .position(|player| player.contract_end().is_some())
            .expect("the world has a contracted player");
        let player = &mut game.players[index];
        let mut record = domain::contract_ledger::ContractRecord {
            start: player.contract_start().map(str::to_owned),
            end: player.contract_end().map(str::to_owned),
            weekly_wage: player.wage(),
            source: domain::contract_ledger::ContractSource::Renewal,
        };
        change(&mut record);
        let today = game.clock.current_date.format("%Y-%m-%d").to_string();
        let entry = domain::player::PlayerMovementEntry {
            contract: Some(record),
            ..domain::player::PlayerMovementEntry::new(
                today,
                domain::player::PlayerMovementKind::Renewal,
            )
        };
        player.record_movement(entry).expect("the renewal is valid");
        index
    }

    #[test]
    fn it_sees_a_players_wage() {
        assert_sees(|game| {
            let index = renew_first_contracted_player(game, |record| record.weekly_wage += 1);
            assert!(game.players[index].wage() > 0, "the renewal took");
        });
    }

    #[test]
    fn it_sees_a_players_contract_end() {
        assert_sees(|game| {
            let index = renew_first_contracted_player(game, |record| {
                record.end = Some("2099-06-30".to_string());
            });
            assert_eq!(game.players[index].contract_end(), Some("2099-06-30"));
        });
    }

    #[test]
    fn it_sees_a_pending_promise() {
        assert_sees(|game| {
            game.players[0].morale_core.pending_promise = Some(domain::player::PlayerPromise {
                kind: domain::player::PlayerPromiseKind::PlayingTime,
                matches_remaining: 3,
            });
        });
    }

    #[test]
    fn it_sees_a_clubs_tactics_phase() {
        assert_sees(|game| {
            game.teams[0].tactics_phase.tempo = domain::team::Tempo::Patient;
            game.teams[0].tactics_phase.width = domain::team::PitchWidth::Wide;
        });
    }

    #[test]
    fn it_sees_a_clubs_training_focus() {
        assert_sees(|game| {
            game.teams[0].training_focus = match game.teams[0].training_focus {
                domain::team::TrainingFocus::Recovery => domain::team::TrainingFocus::Attacking,
                _ => domain::team::TrainingFocus::Recovery,
            };
        });
    }

    #[test]
    fn it_sees_a_clubs_training_schedule() {
        assert_sees(|game| {
            game.teams[0].training_schedule = match game.teams[0].training_schedule {
                domain::team::TrainingSchedule::Light => domain::team::TrainingSchedule::Intense,
                _ => domain::team::TrainingSchedule::Light,
            };
        });
    }

    #[test]
    fn it_sees_a_clubs_formation() {
        assert_sees(|game| {
            game.teams[0].formation = if game.teams[0].formation == "4-4-2" {
                "3-5-2".to_string()
            } else {
                "4-4-2".to_string()
            };
        });
    }

    #[test]
    fn it_sees_a_clubs_play_style() {
        assert_sees(|game| {
            game.teams[0].play_style = match game.teams[0].play_style {
                domain::team::PlayStyle::Counter => domain::team::PlayStyle::HighPress,
                _ => domain::team::PlayStyle::Counter,
            };
        });
    }

    #[test]
    fn it_sees_a_clubs_training_intensity() {
        assert_sees(|game| {
            game.teams[0].training_intensity = match game.teams[0].training_intensity {
                domain::team::TrainingIntensity::High => domain::team::TrainingIntensity::Low,
                _ => domain::team::TrainingIntensity::High,
            };
        });
    }

    #[test]
    fn it_sees_a_players_tactical_role() {
        assert_sees(|game| {
            let player_id = game.players[0].id.clone();
            game.teams[0]
                .player_roles
                .insert(player_id, domain::team::PlayerRole::BallPlayingKeeper);
        });
    }

    #[test]
    fn it_sees_the_starting_eleven() {
        assert_sees(|game| {
            game.teams[0]
                .starting_xi_ids
                .push("a-player-id".to_string())
        });
    }

    #[test]
    fn it_sees_a_message_in_the_inbox() {
        assert_sees(|game| {
            game.messages.push(domain::message::InboxMessage::new(
                "any-id".to_string(),
                "Subject".to_string(),
                "Body".to_string(),
                "Sender".to_string(),
                "2033-07-02".to_string(),
            ));
        });
    }

    /// The order of a map must not matter: two clubs with the same roles inserted in another
    /// order are one world.
    #[test]
    fn it_does_not_see_the_order_roles_were_assigned_in() {
        let mut one = built();
        let mut other = one.clone();
        let roles = [
            ("p-1", domain::team::PlayerRole::BallPlayingKeeper),
            ("p-2", domain::team::PlayerRole::Standard),
            ("p-3", domain::team::PlayerRole::BallPlayingKeeper),
        ];
        for (id, role) in roles.iter().cloned() {
            one.teams[0].player_roles.insert(id.to_string(), role);
        }
        for (id, role) in roles.iter().rev().cloned() {
            other.teams[0].player_roles.insert(id.to_string(), role);
        }
        assert_eq!(fingerprint(&one), fingerprint(&other));
    }

    fn an_article(headline: &str) -> NewsArticle {
        let mut article = NewsArticle::new(
            "any-id".to_string(),
            headline.to_string(),
            "Body".to_string(),
            "Source".to_string(),
            "2033-07-02".to_string(),
            domain::news::NewsCategory::MatchReport,
        );
        article.id = format!("id-{headline}");
        article
    }

    #[test]
    fn it_sees_the_news_and_the_order_it_is_in() {
        assert_sees(|game| game.news.push(an_article("One")));
        let in_order = |first: &str, second: &str| {
            let mut game = built();
            game.news.push(an_article(first));
            game.news.push(an_article(second));
            fingerprint(&game)
        };
        assert_ne!(in_order("One", "Two"), in_order("Two", "One"));
    }

    /// The id is minted at random, so two runs differ in it and agree on everything else.
    #[test]
    fn it_does_not_see_a_news_articles_id() {
        let with_id = |id: &str| {
            let mut game = built();
            let mut article = an_article("One");
            article.id = id.to_string();
            game.news.push(article);
            fingerprint(&game)
        };
        assert_eq!(with_id("a"), with_id("b"));
    }

    #[test]
    fn it_sees_a_squad_floor_top_up() {
        assert_sees(|game| {
            game.squad_floor_top_ups
                .push(ofm_core::squad_floor::SquadFloorTopUp {
                    date: "2033-07-02".to_string(),
                    team_id: "any".to_string(),
                    short: vec![],
                    unfilled: vec![],
                })
        });
    }

    #[test]
    fn it_sees_the_users_satisfaction_and_fan_approval() {
        assert_sees(|game| game.manager.satisfaction -= 1);
        assert_sees(|game| game.manager.fan_approval -= 1);
    }

    #[test]
    fn it_sees_a_teams_form() {
        assert_sees(|game| game.teams[0].form.push("W".to_string()));
    }

    fn player_record(player_id: &str, goals: u8) -> domain::stats::PlayerMatchStatsRecord {
        domain::stats::PlayerMatchStatsRecord {
            fixture_id: "fixture".to_string(),
            season: 2033,
            matchday: 1,
            date: "2033-08-01".to_string(),
            competition: domain::league::FixtureCompetition::League,
            player_id: player_id.to_string(),
            team_id: "home".to_string(),
            opponent_team_id: "away".to_string(),
            home_team_id: "home".to_string(),
            away_team_id: "away".to_string(),
            home_goals: 1,
            away_goals: 0,
            minutes_played: 90,
            goals,
            assists: 0,
            shots: 0,
            shots_on_target: 0,
            passes_completed: 0,
            passes_attempted: 0,
            tackles_won: 0,
            interceptions: 0,
            fouls_committed: 0,
            yellow_cards: 0,
            red_cards: 0,
            rating: 6.0,
        }
    }

    fn logged(captures: &[Vec<domain::stats::PlayerMatchStatsRecord>]) -> String {
        let mut log = super::StatsLog::default();
        for records in captures {
            log.on_stats_captured(&StatsState {
                player_matches: records.clone(),
                team_matches: vec![],
            });
        }
        log.fingerprint()
    }

    /// A report's player stats come out of a `HashMap`, in no particular order.
    #[test]
    fn the_stats_log_ignores_the_order_inside_one_capture() {
        let (a, b) = (player_record("a", 1), player_record("b", 0));
        assert_eq!(logged(&[vec![a.clone(), b.clone()]]), logged(&[vec![b, a]]));
    }

    #[test]
    fn the_stats_log_sees_a_record_and_the_order_of_the_captures() {
        let (a, b) = (player_record("a", 1), player_record("b", 0));
        assert_ne!(
            logged(&[vec![a.clone()]]),
            logged(&[vec![player_record("a", 2)]])
        );
        assert_ne!(
            logged(&[vec![a.clone()], vec![b.clone()]]),
            logged(&[vec![b], vec![a]])
        );
    }

    fn a_message(subject: &str, body: &str, sender: &str) -> domain::message::InboxMessage {
        domain::message::InboxMessage::new(
            "any-id".to_string(),
            subject.to_string(),
            body.to_string(),
            sender.to_string(),
            "2033-07-02".to_string(),
        )
    }

    #[test]
    fn it_sees_who_a_message_is_from() {
        let with_sender = |sender: &str| {
            let mut game = built();
            game.messages.push(a_message("Subject", "Body", sender));
            fingerprint(&game)
        };
        assert_ne!(with_sender("Assistant"), with_sender("Chairman"));
    }

    /// A pipe inside the subject must not read as the boundary between subject and body.
    #[test]
    fn two_messages_that_split_their_text_differently_are_two_messages() {
        let with = |subject: &str, body: &str| {
            let mut game = built();
            game.messages.push(a_message(subject, body, "Assistant"));
            fingerprint(&game)
        };
        assert_ne!(with("A|B", "C"), with("A", "B|C"));
    }
}
