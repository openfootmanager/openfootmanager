use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::player::Player;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageCategory {
    Welcome,
    LeagueInfo,
    MatchPreview,
    MatchResult,
    Transfer,
    BoardDirective,
    PlayerMorale,
    Injury,
    Training,
    Finance,
    Contract,
    ScoutReport,
    Media,
    System,
    JobOffer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAction {
    pub id: String,
    pub label: String,
    pub action_type: ActionType,
    pub resolved: bool,
    /// Optional i18n key for the action label (frontend resolves via t())
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Acknowledge,
    NavigateTo { route: String },
    ChooseOption { options: Vec<ActionOption> },
    Dismiss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOption {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessage {
    pub id: String,
    pub subject: String,
    pub body: String,
    pub sender: String,
    pub sender_role: String,
    pub date: String,
    pub read: bool,
    pub category: MessageCategory,
    pub priority: MessagePriority,
    pub actions: Vec<MessageAction>,
    /// Optional references to entities relevant to this message
    pub context: MessageContext,
    /// Optional i18n key for the subject (frontend resolves via t())
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_key: Option<String>,
    /// Optional i18n key for the body (frontend resolves via t())
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_key: Option<String>,
    /// Optional i18n key for the sender name (frontend resolves via t())
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_key: Option<String>,
    /// Optional i18n key for the sender role (frontend resolves via t())
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_role_key: Option<String>,
    /// Interpolation parameters for the i18n keys (shared by subject/body/sender)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub i18n_params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageContext {
    pub team_id: Option<String>,
    pub player_id: Option<String>,
    pub fixture_id: Option<String>,
    pub match_result: Option<ContextMatchResult>,
    #[serde(default)]
    pub youth_target_position: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub youth_search_region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub youth_search_objective: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub youth_prospects: Option<Vec<Player>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scout_report: Option<ScoutReportData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegated_renewal_report: Option<DelegatedRenewalReportData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedRenewalReportData {
    pub success_count: u32,
    pub failure_count: u32,
    pub stalled_count: u32,
    pub cases: Vec<DelegatedRenewalCaseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedRenewalCaseData {
    pub player_id: String,
    pub player_name: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreed_wage: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreed_years: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_key: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub note_params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutReportData {
    pub player_id: String,
    pub player_name: String,
    pub position: String,
    pub nationality: String,
    pub dob: String,
    pub team_name: Option<String>,
    /// Fuzzed attributes — None means not discovered by this scout
    pub pace: Option<u8>,
    pub shooting: Option<u8>,
    pub passing: Option<u8>,
    pub dribbling: Option<u8>,
    pub defending: Option<u8>,
    pub physical: Option<u8>,
    pub condition: Option<u8>,
    pub morale: Option<u8>,
    /// Approximate overall rating (fuzzed average)
    pub avg_rating: Option<u32>,
    /// i18n key for overall rating description
    pub rating_key: String,
    /// i18n key for potential assessment
    pub potential_key: String,
    /// i18n key for report confidence level
    pub confidence_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMatchResult {
    pub home_team_id: String,
    pub away_team_id: String,
    pub home_goals: u8,
    pub away_goals: u8,
}

impl InboxMessage {
    pub fn new(id: String, subject: String, body: String, sender: String, date: String) -> Self {
        Self {
            id,
            subject,
            body,
            sender,
            sender_role: String::new(),
            date,
            read: false,
            category: MessageCategory::System,
            priority: MessagePriority::Normal,
            actions: vec![],
            context: MessageContext::default(),
            subject_key: None,
            body_key: None,
            sender_key: None,
            sender_role_key: None,
            i18n_params: HashMap::new(),
        }
    }

    pub fn with_category(mut self, category: MessageCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_sender_role(mut self, role: &str) -> Self {
        self.sender_role = role.to_string();
        self
    }

    pub fn with_action(mut self, action: MessageAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_context(mut self, context: MessageContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_i18n(
        mut self,
        subject_key: &str,
        body_key: &str,
        params: HashMap<String, String>,
    ) -> Self {
        self.subject_key = Some(subject_key.to_string());
        self.body_key = Some(body_key.to_string());
        self.i18n_params = params;
        self
    }

    pub fn with_sender_i18n(mut self, sender_key: &str, role_key: &str) -> Self {
        self.sender_key = Some(sender_key.to_string());
        self.sender_role_key = Some(role_key.to_string());
        self
    }
}
