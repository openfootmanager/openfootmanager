use domain::player::PlayerAttributes;
use domain::team::TrainingFocus;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum TrainedAttribute {
    Pace,
    Stamina,
    Strength,
    Agility,
    Passing,
    Shooting,
    Dribbling,
    Positioning,
    Vision,
    Decisions,
    Composure,
    Tackling,
    Defending,
    Handling,
    Reflexes,
}

impl TrainedAttribute {
    pub(super) fn of_mut(self, attrs: &mut PlayerAttributes) -> &mut u8 {
        match self {
            Self::Pace => &mut attrs.pace,
            Self::Stamina => &mut attrs.stamina,
            Self::Strength => &mut attrs.strength,
            Self::Agility => &mut attrs.agility,
            Self::Passing => &mut attrs.passing,
            Self::Shooting => &mut attrs.shooting,
            Self::Dribbling => &mut attrs.dribbling,
            Self::Positioning => &mut attrs.positioning,
            Self::Vision => &mut attrs.vision,
            Self::Decisions => &mut attrs.decisions,
            Self::Composure => &mut attrs.composure,
            Self::Tackling => &mut attrs.tackling,
            Self::Defending => &mut attrs.defending,
            Self::Handling => &mut attrs.handling,
            Self::Reflexes => &mut attrs.reflexes,
        }
    }

    /// Matches the `PlayerAttributes` field name, which the frontend uses as its label key.
    fn key(self) -> &'static str {
        match self {
            Self::Pace => "pace",
            Self::Stamina => "stamina",
            Self::Strength => "strength",
            Self::Agility => "agility",
            Self::Passing => "passing",
            Self::Shooting => "shooting",
            Self::Dribbling => "dribbling",
            Self::Positioning => "positioning",
            Self::Vision => "vision",
            Self::Decisions => "decisions",
            Self::Composure => "composure",
            Self::Tackling => "tackling",
            Self::Defending => "defending",
            Self::Handling => "handling",
            Self::Reflexes => "reflexes",
        }
    }
}

/// `rate` scales the session's base gain probability for this attribute.
#[derive(Debug, Clone, Copy)]
pub(super) struct FocusGain {
    pub(super) attribute: TrainedAttribute,
    pub(super) rate: f64,
}

const fn full(attribute: TrainedAttribute) -> FocusGain {
    FocusGain {
        attribute,
        rate: 1.0,
    }
}

const fn half(attribute: TrainedAttribute) -> FocusGain {
    FocusGain {
        attribute,
        rate: 0.5,
    }
}

use TrainedAttribute as A;

const PHYSICAL: &[FocusGain] = &[
    full(A::Pace),
    full(A::Stamina),
    full(A::Strength),
    full(A::Agility),
];
const TECHNICAL_OUTFIELD: &[FocusGain] = &[full(A::Passing), full(A::Shooting), full(A::Dribbling)];
const TECHNICAL_GOALKEEPER: &[FocusGain] =
    &[full(A::Passing), full(A::Handling), full(A::Reflexes)];
const TACTICAL: &[FocusGain] = &[
    full(A::Positioning),
    full(A::Vision),
    full(A::Decisions),
    full(A::Composure),
];
const DEFENDING_OUTFIELD: &[FocusGain] = &[
    full(A::Tackling),
    full(A::Defending),
    half(A::Strength),
    half(A::Positioning),
];
const DEFENDING_GOALKEEPER: &[FocusGain] = &[
    full(A::Handling),
    full(A::Reflexes),
    half(A::Strength),
    half(A::Positioning),
];
const ATTACKING: &[FocusGain] = &[full(A::Shooting), full(A::Dribbling), half(A::Pace)];

/// The order is the order gain rolls are drawn, which seeded training results depend on.
pub(super) fn focus_gains(focus: &TrainingFocus, is_goalkeeper: bool) -> &'static [FocusGain] {
    match (focus, is_goalkeeper) {
        (TrainingFocus::Physical, _) => PHYSICAL,
        (TrainingFocus::Technical, false) => TECHNICAL_OUTFIELD,
        (TrainingFocus::Technical, true) => TECHNICAL_GOALKEEPER,
        (TrainingFocus::Tactical, _) => TACTICAL,
        (TrainingFocus::Defending, false) => DEFENDING_OUTFIELD,
        (TrainingFocus::Defending, true) => DEFENDING_GOALKEEPER,
        (TrainingFocus::Attacking, _) => ATTACKING,
        (TrainingFocus::Recovery, _) => &[],
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TrainingFocusAttributes {
    pub focus: TrainingFocus,
    pub outfield: Vec<&'static str>,
    pub goalkeeper: Vec<&'static str>,
}

const ALL_FOCUSES: [TrainingFocus; 6] = [
    TrainingFocus::Physical,
    TrainingFocus::Technical,
    TrainingFocus::Tactical,
    TrainingFocus::Defending,
    TrainingFocus::Attacking,
    TrainingFocus::Recovery,
];

fn attribute_keys(gains: &[FocusGain]) -> Vec<&'static str> {
    gains.iter().map(|gain| gain.attribute.key()).collect()
}

pub fn training_focus_attributes() -> Vec<TrainingFocusAttributes> {
    ALL_FOCUSES
        .into_iter()
        .map(|focus| TrainingFocusAttributes {
            outfield: attribute_keys(focus_gains(&focus, false)),
            goalkeeper: attribute_keys(focus_gains(&focus, true)),
            focus,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes_for(focus: TrainingFocus) -> TrainingFocusAttributes {
        training_focus_attributes()
            .into_iter()
            .find(|entry| entry.focus == focus)
            .unwrap()
    }

    /// Given a goalkeeper, when Technical is listed, then handling and reflexes replace shooting and dribbling.
    #[test]
    fn a_goalkeepers_technical_focus_lists_handling_reflexes_and_passing() {
        let technical = attributes_for(TrainingFocus::Technical);
        assert_eq!(technical.goalkeeper, ["passing", "handling", "reflexes"]);
    }

    /// Given an outfield player, when Technical is listed, then the list is unchanged.
    #[test]
    fn an_outfield_players_technical_focus_is_unchanged() {
        let technical = attributes_for(TrainingFocus::Technical);
        assert_eq!(technical.outfield, ["passing", "shooting", "dribbling"]);
    }

    /// Given a goalkeeper and Defending, then keeper skills replace tackling and defending but the secondary gains stay.
    #[test]
    fn a_goalkeepers_defending_focus_lists_keeper_skills_and_the_secondary_gains() {
        let defending = attributes_for(TrainingFocus::Defending);
        assert_eq!(
            defending.goalkeeper,
            ["handling", "reflexes", "strength", "positioning"]
        );
        assert_eq!(
            defending.outfield,
            ["tackling", "defending", "strength", "positioning"]
        );
    }

    /// Given every focus, when listed, then Recovery trains nothing and the focus names serialize as the frontend ids.
    #[test]
    fn every_focus_is_listed_once_with_its_frontend_id() {
        let listed: Vec<_> = training_focus_attributes()
            .iter()
            .map(|entry| serde_json::to_value(&entry.focus).unwrap())
            .collect();
        assert_eq!(
            listed,
            [
                "Physical",
                "Technical",
                "Tactical",
                "Defending",
                "Attacking",
                "Recovery"
            ]
        );
        assert!(attributes_for(TrainingFocus::Recovery).outfield.is_empty());
    }

    fn sequence(focus: TrainingFocus, is_goalkeeper: bool) -> Vec<(&'static str, f64)> {
        focus_gains(&focus, is_goalkeeper)
            .iter()
            .map(|gain| (gain.attribute.key(), gain.rate))
            .collect()
    }

    /// Given each focus, when its gains are read for outfield players and keepers, then the attribute order and rates are exactly the ones seeded training draws its rolls in.
    #[test]
    fn each_focus_gains_keep_their_roll_order_and_rates() {
        use TrainingFocus::*;
        let physical = vec![
            ("pace", 1.0),
            ("stamina", 1.0),
            ("strength", 1.0),
            ("agility", 1.0),
        ];
        let tactical = vec![
            ("positioning", 1.0),
            ("vision", 1.0),
            ("decisions", 1.0),
            ("composure", 1.0),
        ];
        let attacking = vec![("shooting", 1.0), ("dribbling", 1.0), ("pace", 0.5)];
        for is_goalkeeper in [false, true] {
            assert_eq!(sequence(Physical, is_goalkeeper), physical);
            assert_eq!(sequence(Tactical, is_goalkeeper), tactical);
            assert_eq!(sequence(Attacking, is_goalkeeper), attacking);
            assert!(sequence(Recovery, is_goalkeeper).is_empty());
        }
        assert_eq!(
            sequence(Technical, false),
            [("passing", 1.0), ("shooting", 1.0), ("dribbling", 1.0)]
        );
        assert_eq!(
            sequence(Technical, true),
            [("passing", 1.0), ("handling", 1.0), ("reflexes", 1.0)]
        );
        assert_eq!(
            sequence(Defending, false),
            [
                ("tackling", 1.0),
                ("defending", 1.0),
                ("strength", 0.5),
                ("positioning", 0.5)
            ]
        );
        assert_eq!(
            sequence(Defending, true),
            [
                ("handling", 1.0),
                ("reflexes", 1.0),
                ("strength", 0.5),
                ("positioning", 0.5)
            ]
        );
    }
}
