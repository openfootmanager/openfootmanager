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
pub fn generate_youth_academy_recruit(
    team: &Team,
    target_position: Option<&Position>,
    current_year: u32,
    rng: &mut impl rand::Rng,
) -> Player {
    generate_youth_academy_recruit_with_nationality(team, target_position, None, current_year, rng)
}

/// As [`generate_youth_academy_recruit`], with the prospect's nationality forced
/// rather than drawn from the club's country. See there for `current_year`.
pub fn generate_youth_academy_recruit_with_nationality(
    team: &Team,
    target_position: Option<&Position>,
    nationality_override: Option<&str>,
    current_year: u32,
    rng: &mut impl rand::Rng,
) -> Player {
    youth_recruit(
        team,
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
    group: &Position,
    age: u32,
    current_year: u32,
    rng: &mut impl rand::Rng,
) -> Player {
    youth_recruit(team, Some(group), None, current_year, Some(age), rng)
}

fn youth_recruit(
    team: &Team,
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
        age,
        &names_def,
        rng,
    );
    player.squad_role = SquadRole::Youth;
    player.transfer_listed = false;
    player.loan_listed = false;
    player
}
