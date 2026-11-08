//! Youngsters joining a club's academy: the one youth-recruit generator,
//! behind both a youth scout's prospects and the annual intake.

use super::*;
use domain::player::{Player, Position};
use domain::team::Team;

/// Generate a youth prospect who is joining **now**.
///
/// `current_year` is the year the recruit arrives, not the year the world opened:
/// a prospect scouted five seasons into a career is fifteen in *that* season. The
/// two coincide only for the opening intake, which is why the distinction is
/// worth naming — a call site that passed the world's opening year here would
/// quietly produce a squad of players five years too old.
///
/// `reputations` is the scale of the world's clubs as they stand now
/// ([`ReputationScale::of_clubs`] over the game's clubs): the club's
/// standard, which the recruit is drawn on, is its reputation against theirs.
pub fn generate_youth_academy_recruit(
    team: &Team,
    reputations: &ReputationScale,
    target_position: Option<&Position>,
    current_year: u32,
    rng: &mut impl rand::Rng,
) -> Player {
    generate_youth_academy_recruit_with_nationality(
        team,
        reputations,
        target_position,
        None,
        current_year,
        rng,
    )
}

/// As [`generate_youth_academy_recruit`], with the prospect's nationality forced
/// rather than drawn from the club's country. See there for `current_year`.
pub fn generate_youth_academy_recruit_with_nationality(
    team: &Team,
    reputations: &ReputationScale,
    target_position: Option<&Position>,
    nationality_override: Option<&str>,
    current_year: u32,
    rng: &mut impl rand::Rng,
) -> Player {
    youth_recruit(
        team,
        reputations,
        target_position,
        nationality_override,
        current_year,
        None,
        rng,
    )
}

/// A youngster joining `team`'s academy in its annual intake: of `group`, aged
/// `age`, drawn from `rng`. The same recruit a youth scout finds, at the age a
/// club takes one in rather than the age it scouts one.
pub(crate) fn generate_youth_intake_recruit(
    team: &Team,
    reputations: &ReputationScale,
    group: &Position,
    age: u32,
    current_year: u32,
    rng: &mut impl rand::Rng,
) -> Player {
    youth_recruit(
        team,
        reputations,
        Some(group),
        None,
        current_year,
        Some(age),
        rng,
    )
}

fn youth_recruit(
    team: &Team,
    reputations: &ReputationScale,
    target_position: Option<&Position>,
    nationality_override: Option<&str>,
    current_year: u32,
    age: Option<u32>,
    rng: &mut impl rand::Rng,
) -> Player {
    use domain::player::SquadRole;

    let names_def = default_names_definition();
    let country_codes = generation::nationality_distribution();
    let nationality = nationality_override
        .map(generation::canonicalize_generated_nationality)
        .unwrap_or_else(|| {
            // `team_local_nationality`, not `team.country`: a club carries both a
            // location and a football identity, and where they differ the
            // football identity is the one a youth intake should draw on.
            pick_nationality_from_def(team_local_nationality(team), country_codes, rng)
        });
    let youth_slots = youth_slots_for_target(target_position.map(Position::to_group_position));
    let slot_index = youth_slots[rng.random_range(0..youth_slots.len())];
    let mut player = generate_random_player_from_def(
        &team.id,
        slot_index,
        &nationality,
        current_year,
        PlayerRoll::for_academy(team.reputation, age, reputations),
        &names_def,
        rng,
    );
    player.squad_role = SquadRole::Youth;
    player.transfer_listed = false;
    player.loan_listed = false;
    // A specific position (LB, CM, ...) from the day he arrives: the upgrade used to run only on
    // a new game's squads and on load, so a signed prospect showed the generic group until a reload.
    crate::player_identity::upgrade_player_identity(&mut player, None);
    player
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    const RECRUITS: usize = 4_000;
    const AGE: u32 = 16;
    const YEAR: u32 = 2030;

    fn club(reputation: u32) -> Team {
        let mut team = Team::new(
            format!("club{reputation}"),
            format!("Club {reputation}"),
            "CLB".to_string(),
            "ENG".to_string(),
            "London".to_string(),
            "Ground".to_string(),
            20_000,
        );
        team.reputation = reputation;
        team
    }

    /// A world whose clubs' reputations centre on 500, a spread of 150 apart.
    fn world_scale() -> ReputationScale {
        ReputationScale::new(500.0, 150.0)
    }

    /// The ceilings of `RECRUITS` intake recruits of `team` in [`world_scale`]'s world, from a
    /// fixed seed.
    fn academy_ceilings(team: &Team, seed: u64) -> Vec<f64> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let reputations = world_scale();
        (0..RECRUITS)
            .map(|_| {
                let recruit = generate_youth_intake_recruit(
                    team,
                    &reputations,
                    &Position::Midfielder,
                    AGE,
                    YEAR,
                    &mut rng,
                );
                f64::from(recruit.potential)
            })
            .collect()
    }

    /// The same recruits rolled on the intake's bell, from the same seed: the bell the world's
    /// academies make together, and every academy's draws before academies had a standard.
    fn intake_bell_ceilings(seed: u64) -> Vec<f64> {
        intake_bell_ceilings_n(seed, RECRUITS)
    }

    fn intake_bell_ceilings_n(seed: u64, count: usize) -> Vec<f64> {
        intake_bell_recruits_n(seed, count)
            .into_iter()
            .map(|(potential, _)| f64::from(potential))
            .collect()
    }

    /// The intake's recruits as `(potential, ovr)`: the two a wonderkid is judged by.
    fn intake_bell_recruits_n(seed: u64, count: usize) -> Vec<(u8, u8)> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let names = default_names_definition();
        let slot = youth_slots_for_target(Some(Position::Midfielder))[0];
        (0..count)
            .map(|_| {
                let recruit = generate_random_player_from_def(
                    "club",
                    slot,
                    "ENG",
                    YEAR,
                    PlayerRoll::for_intake(Some(AGE)),
                    &names,
                    &mut rng,
                );
                (recruit.potential, recruit.ovr)
            })
            .collect()
    }

    fn wonderkids_among(recruits: &[(u8, u8)]) -> usize {
        recruits
            .iter()
            .filter(|&&(potential, ovr)| {
                crate::player_rating::qualifies_for_wonderkid(AGE, potential, ovr)
            })
            .count()
    }

    fn mean(values: &[f64]) -> f64 {
        values.iter().sum::<f64>() / values.len() as f64
    }

    fn share_at_or_above(values: &[f64], ceiling: f64) -> f64 {
        values.iter().filter(|&&value| value >= ceiling).count() as f64 / values.len() as f64
    }

    /// Given a big club and a small one taking recruits from the same seeds
    /// When each takes its intake
    /// Then the big club's recruits have clearly higher ceilings: an academy draws on its club's
    ///      standing
    #[test]
    fn a_big_clubs_academy_recruits_higher_ceilings_than_a_small_clubs() {
        let big = mean(&academy_ceilings(&club(800), 1));
        let small = mean(&academy_ceilings(&club(200), 1));

        assert!(
            big - small > 2.0,
            "big club {big:.2} against small club {small:.2}"
        );
    }

    /// Given a club of average reputation
    /// When it takes its intake
    /// Then its recruits' ceilings average the intake's: no lift is added, so the world's
    ///      academies do not inflate its ratings season after season
    #[test]
    fn an_average_clubs_academy_recruits_on_the_intakes_mean() {
        let average = mean(&academy_ceilings(&club(500), 2));
        let intake = mean(&intake_bell_ceilings(2));

        assert!(
            (average - intake).abs() < 0.4,
            "average club {average:.2} against the intake {intake:.2}"
        );
    }

    fn variance(values: &[f64]) -> f64 {
        let centre = mean(values);
        values
            .iter()
            .map(|value| (value - centre).powi(2))
            .sum::<f64>()
            / values.len() as f64
    }

    /// The shipped world's 440 clubs, each at the middle of its reputation range.
    fn shipped_reputations() -> Vec<u32> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        clubs::generate_club_defs(&WorldGenConfig::standard(), &mut rng)
            .iter()
            .filter_map(|def| def.reputation_range)
            .map(|[low, high]| (low + high) / 2)
            .collect()
    }

    /// Given every club of the shipped world, at its real reputation
    /// When each takes recruits
    /// Then the world's recruits are spread like the intake's bell at their age, and reach the
    ///      S tier no more often: the academies sort the talent, they do not add to it
    #[test]
    fn the_shipped_worlds_academies_add_no_stars() {
        const PER_CLUB: usize = 50;
        let reputations = shipped_reputations();
        assert!(reputations.len() >= 400, "the shipped world has its clubs");
        // The scale an intake builds from the running world's clubs.
        let scale = ReputationScale::of_reputations(reputations.iter().copied());
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let academies: Vec<f64> = reputations
            .iter()
            .flat_map(|&reputation| {
                let team = club(reputation);
                (0..PER_CLUB)
                    .map(|_| {
                        let recruit = generate_youth_intake_recruit(
                            &team,
                            &scale,
                            &Position::Midfielder,
                            AGE,
                            YEAR,
                            &mut rng,
                        );
                        f64::from(recruit.potential)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let intake: Vec<f64> = (0..reputations.len() as u64)
            .flat_map(|seed| intake_bell_ceilings_n(1_000 + seed, PER_CLUB))
            .collect();

        let spread_ratio = variance(&academies) / variance(&intake);
        assert!(
            (0.97..=1.03).contains(&spread_ratio),
            "academies' variance is {spread_ratio:.3} of the intake's"
        );
        let star_ratio = share_at_or_above(&academies, 81.0) / share_at_or_above(&intake, 81.0);
        assert!(
            star_ratio <= 1.1,
            "academies reach the S tier {star_ratio:.2} times as often as the intake"
        );
    }

    /// Given every club of the shipped world, at its real reputation
    /// When each takes recruits
    /// Then the young high-ceiling tail the maintainer wants rare is no fatter than the intake's:
    ///      the academy standard sorts the wonderkids into the big clubs, it does not make more
    #[test]
    fn the_shipped_worlds_academies_add_no_wonderkids() {
        const PER_CLUB: usize = 50;
        let reputations = shipped_reputations();
        let scale = ReputationScale::of_reputations(reputations.iter().copied());
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let academies: Vec<(u8, u8)> = reputations
            .iter()
            .flat_map(|&reputation| {
                let team = club(reputation);
                (0..PER_CLUB)
                    .map(|_| {
                        let recruit = generate_youth_intake_recruit(
                            &team,
                            &scale,
                            &Position::Midfielder,
                            AGE,
                            YEAR,
                            &mut rng,
                        );
                        (recruit.potential, recruit.ovr)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let intake: Vec<(u8, u8)> = (0..reputations.len() as u64)
            .flat_map(|seed| intake_bell_recruits_n(1_000 + seed, PER_CLUB))
            .collect();

        let bell = wonderkids_among(&intake);
        let academy = wonderkids_among(&academies);
        assert!(
            bell >= 50,
            "the intake must produce enough wonderkids for a ratio to mean anything: {bell}"
        );
        assert!(
            academy as f64 <= bell as f64 * 1.25,
            "academies made {academy} wonderkids against the intake's {bell} from the same number of recruits"
        );
    }

    /// Given clubs spread across the world's reputations
    /// When they all take their intake
    /// Then they produce S-tier ceilings at the intake's rate, give or take sampling: the best
    ///      prospects gather at the big clubs, but there are neither more nor fewer of them
    #[test]
    fn the_worlds_academies_produce_s_tier_ceilings_at_the_intakes_rate() {
        let pooled: Vec<f64> = [275, 425, 500, 575, 725]
            .into_iter()
            .enumerate()
            .flat_map(|(index, reputation)| academy_ceilings(&club(reputation), 10 + index as u64))
            .collect();
        let intake: Vec<f64> = (10..15).flat_map(intake_bell_ceilings).collect();

        let academies = share_at_or_above(&pooled, 81.0);
        let bell = share_at_or_above(&intake, 81.0);
        assert!(
            academies <= bell * 1.2 && academies >= bell * 0.8,
            "academies {academies:.4} against the intake {bell:.4}"
        );
    }
}
