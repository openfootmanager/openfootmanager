//! The world a run plays in: generated from a seed, given a pyramid by the same
//! builder every new career uses, with the manager in charge of a club.

use domain::manager::Manager;
use domain::stats::StatsState;
use ofm_core::career::{CareerScope, begin_career};
use ofm_core::clock::GameClock;
use ofm_core::game::Game;
use ofm_core::generator::{
    DefinitionSources, WorldGenConfig, generate_world_data_seeded_with,
    repair_opening_youth_academies,
};
use ofm_core::world::start_date_for_year;

/// The shipped world's nations that between them cover every shape that has
/// broken the ladder: England's two tiers, Portugal's single tier, and
/// Argentina's split season, all berthing into one continental cup.
pub const GATE_NATIONS: &[&str] = &["ENG", "PT", "AR"];

/// A world to play: the shipped one, minus some nations.
///
/// Divisions keep their real size. Shrinking them is tempting — a season costs
/// more the more clubs there are — but an 8-club league finishes in October, so
/// the rollover fires before that year's international windows have happened and
/// the calendar the world runs on is not the game's. Fewer *nations* keeps every
/// shape and the real calendar: England (40) + Portugal (20) + Argentina (20) is
/// 80 clubs and plays two seasons in about eight seconds.
#[derive(Debug, Clone)]
pub struct WorldSpec {
    pub seed: u64,
    pub nations: &'static [&'static str],
    /// The manager takes this nation's strongest club.
    pub user_nation: &'static str,
    pub start_year: i32,
}

impl WorldSpec {
    /// England, Portugal and Argentina from 2033, which puts a World Cup summer
    /// on the first rollover.
    pub fn gate(seed: u64) -> Self {
        Self {
            seed,
            nations: GATE_NATIONS,
            user_nation: "ENG",
            start_year: 2033,
        }
    }

    pub fn build(&self) -> Result<Game, String> {
        let sources = DefinitionSources::embedded_only();
        let mut config = WorldGenConfig::standard_from(&sources);
        config
            .nations
            .retain(|nation| self.nations.contains(&nation.code.as_str()));
        // Dated for the year the clock opens in. Left to default it follows the
        // wall clock, which makes a run differ by the day it is made and opens
        // a 2033 game with every contract a few years expired.
        config.opening_year = u32::try_from(self.start_year).ok();
        let world = generate_world_data_seeded_with(self.seed, &config, &sources);

        let start = start_date_for_year(self.start_year)?;
        let manager = Manager::new(
            "season-harness-manager".to_string(),
            "Season".to_string(),
            "Harness".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let mut game = Game::new(
            GameClock::new(start),
            manager,
            world.teams,
            world.players,
            world.staff,
            vec![],
        );
        game.available_staff_market_last_activity_date = Some(start.format("%Y-%m-%d").to_string());
        repair_opening_youth_academies(&mut game);

        let club = self.strongest_club(&game)?;
        begin_career(
            &mut game,
            &club,
            CareerScope::default(),
            StatsState::default(),
        )?;
        Ok(game)
    }

    fn strongest_club(&self, game: &Game) -> Result<String, String> {
        game.teams
            .iter()
            .filter(|team| team.football_nation == self.user_nation)
            .max_by(|a, b| {
                a.reputation
                    .cmp(&b.reputation)
                    .then_with(|| b.id.cmp(&a.id))
            })
            .map(|team| team.id.clone())
            .ok_or_else(|| format!("the world has no {} club", self.user_nation))
    }
}
