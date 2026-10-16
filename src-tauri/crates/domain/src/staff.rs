use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Staff {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub nationality: String,
    #[serde(default)]
    pub football_nation: String,
    #[serde(default)]
    pub birth_country: Option<String>,
    pub role: StaffRole,

    // Attributes 0-100
    pub attributes: StaffAttributes,
    pub team_id: Option<String>,

    // Coaching specialization — boosts one training focus area
    #[serde(default)]
    pub specialization: Option<CoachingSpecialization>,

    // Contract & finances
    #[serde(default)]
    pub wage: u32,
    #[serde(default)]
    pub contract_end: Option<String>,

    /// What they are called when it differs from their name ("Pep" for Josep Guardiola i Sala).
    #[serde(default)]
    pub known_as: Option<String>,
    /// The one name to show for them: `known_as` if set, else first and last name. Kept up to
    /// date by [`Staff::new`], [`Staff::set_known_as`] and the loader, so a reader renders it
    /// as it is and never decides between the names itself.
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StaffRole {
    AssistantManager,
    Coach,
    Scout,
    Physio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CoachingSpecialization {
    Fitness,     // Boosts Physical training
    Technique,   // Boosts Technical training
    Tactics,     // Boosts Tactical training
    Defending,   // Boosts Defending training
    Attacking,   // Boosts Attacking training
    GoalKeeping, // Boosts GK-specific development
    Youth,       // Boosts young player development
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// camelCase matches the package convention (see SCHEMA_REFERENCE) and the World
// Editor frontend, so authored staff attributes round-trip both ways. The
// snake_case aliases keep existing saves (serialized before this change) and
// snake_case-authored packages loading.
#[serde(rename_all = "camelCase")]
pub struct StaffAttributes {
    pub coaching: u8,
    #[serde(alias = "judging_ability")]
    pub judging_ability: u8,
    #[serde(alias = "judging_potential")]
    pub judging_potential: u8,
    pub physiotherapy: u8,
}

impl Staff {
    pub fn new(
        id: String,
        first_name: String,
        last_name: String,
        date_of_birth: String,
        role: StaffRole,
        attributes: StaffAttributes,
    ) -> Self {
        Self {
            id,
            first_name,
            last_name,
            date_of_birth,
            nationality: String::new(),
            football_nation: String::new(),
            birth_country: None,
            role,
            attributes,
            team_id: None,
            specialization: None,
            wage: 0,
            contract_end: None,
            known_as: None,
            display_name: String::new(),
        }
        .with_display_name_refreshed()
    }

    fn with_display_name_refreshed(mut self) -> Self {
        self.refresh_display_name();
        self
    }

    /// First and last name together.
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }

    /// Sets what they are called; blank means nothing in particular.
    pub fn set_known_as(&mut self, known_as: Option<String>) {
        self.known_as = known_as
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty());
        self.refresh_display_name();
    }

    /// Recomputes `display_name` from the names it depends on.
    pub fn refresh_display_name(&mut self) {
        self.display_name = self.known_as.clone().unwrap_or_else(|| self.full_name());
    }
}

#[cfg(test)]
mod tests {
    use super::{Staff, StaffAttributes, StaffRole};

    fn staff() -> Staff {
        Staff::new(
            "s1".into(),
            "Josep".into(),
            "Guardiola i Sala".into(),
            "1971-01-18".into(),
            StaffRole::Coach,
            StaffAttributes {
                coaching: 80,
                judging_ability: 50,
                judging_potential: 50,
                physiotherapy: 10,
            },
        )
    }

    /// Given a staff member with and without a known-as name (#557)
    /// When their display name is read
    /// Then it is the known-as name when set, else the full name, and a blank known-as is none.
    #[test]
    fn the_display_name_is_the_known_as_name_or_the_full_name() {
        let mut member = staff();
        assert_eq!(member.display_name, "Josep Guardiola i Sala");

        member.set_known_as(Some("Pep Guardiola".into()));
        assert_eq!(member.display_name, "Pep Guardiola");
        assert_eq!(member.full_name(), "Josep Guardiola i Sala");

        member.set_known_as(Some("   ".into()));
        assert_eq!(member.known_as, None);
        assert_eq!(member.display_name, "Josep Guardiola i Sala");
    }

    #[test]
    fn deserializes_camelcase_attributes() {
        let json =
            r#"{"coaching":70,"judgingAbility":65,"judgingPotential":60,"physiotherapy":40}"#;
        let attrs: StaffAttributes = serde_json::from_str(json).expect("camelCase should parse");
        assert_eq!(attrs.judging_ability, 65);
        assert_eq!(attrs.judging_potential, 60);
    }

    #[test]
    fn still_deserializes_snakecase_attributes_from_existing_saves() {
        let json =
            r#"{"coaching":70,"judging_ability":65,"judging_potential":60,"physiotherapy":40}"#;
        let attrs: StaffAttributes =
            serde_json::from_str(json).expect("snake_case alias should parse");
        assert_eq!(attrs.judging_ability, 65);
        assert_eq!(attrs.judging_potential, 60);
    }

    #[test]
    fn serializes_as_camelcase_so_the_editor_can_read_it_back() {
        let attrs = StaffAttributes {
            coaching: 70,
            judging_ability: 65,
            judging_potential: 60,
            physiotherapy: 40,
        };
        let json = serde_json::to_string(&attrs).expect("should serialize");
        assert!(
            json.contains("\"judgingAbility\":65"),
            "serialized as: {json}"
        );
        assert!(
            json.contains("\"judgingPotential\":60"),
            "serialized as: {json}"
        );
        // Round-trips back to the same struct.
        let back: StaffAttributes = serde_json::from_str(&json).expect("round-trip");
        assert_eq!(back.judging_ability, 65);
    }
}
