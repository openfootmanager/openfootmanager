use super::indices::StandingByTeam;
use super::{TeamCard, TeamCardColors, TeamCardMedia, TeamCardTeam, TeamStanding};
use domain::league::StandingEntry;
use domain::player::Player;
use domain::team::{PlayStyle, Team};
use std::collections::HashMap;

pub(super) fn project_card(
    team: &Team,
    players_by_team: &HashMap<&str, Vec<&Player>>,
    standing_by_team: &StandingByTeam,
) -> TeamCard {
    let roster: &[&Player] = players_by_team
        .get(team.id.as_str())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let roster_size = roster.len();
    let total_value: u64 = roster.iter().map(|p| p.market_value).sum();
    let avg_ovr = if roster.is_empty() {
        0
    } else {
        let total: u32 = roster.iter().map(|p| u32::from(p.ovr)).sum();
        (total / roster.len() as u32) as u8
    };

    let (league_pos, standing) = match standing_by_team.get(team.id.as_str()) {
        Some((pos, entry)) => (*pos, Some(project_standing(entry))),
        None => (0, None),
    };

    TeamCard {
        team: project_team_data(team),
        roster_size,
        avg_ovr,
        total_value,
        league_pos,
        standing,
    }
}

fn project_team_data(team: &Team) -> TeamCardTeam {
    TeamCardTeam {
        id: team.id.clone(),
        name: team.name.clone(),
        short_name: team.short_name.clone(),
        city: team.city.clone(),
        country: team.country.clone(),
        colors: TeamCardColors {
            primary: team.colors.primary.clone(),
            secondary: team.colors.secondary.clone(),
        },
        formation: team.formation.clone(),
        play_style: play_style_name(&team.play_style),
        founded_year: team.founded_year,
        reputation: team.reputation,
        media: TeamCardMedia { logo: team.media.logo.clone() },
    }
}

fn project_standing(entry: &StandingEntry) -> TeamStanding {
    TeamStanding {
        played: entry.played,
        won: entry.won,
        drawn: entry.drawn,
        lost: entry.lost,
        goals_for: entry.goals_for,
        goals_against: entry.goals_against,
        points: entry.points,
    }
}

fn play_style_name(style: &PlayStyle) -> String {
    match style {
        PlayStyle::Balanced => "Balanced",
        PlayStyle::Attacking => "Attacking",
        PlayStyle::Defensive => "Defensive",
        PlayStyle::Possession => "Possession",
        PlayStyle::Counter => "Counter",
        PlayStyle::HighPress => "HighPress",
    }
    .to_string()
}
