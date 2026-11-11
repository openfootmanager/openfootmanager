//! Results of the inbox tools.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool_results;

/// Messages the text lists before it summarises the rest; the structure always carries all of them.
const LISTED_IN_TEXT: usize = 20;

/// One choice of a `choose_option` action: pass `id` as `option_id` to `inbox_resolve_action`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxActionOption {
    pub id: String,
    pub label: String,
    pub description: String,
}

/// Something a message asks the manager to do. Pass the message's `id` and this `id` to
/// `inbox_resolve_action`; a `resolved` action has been answered and is refused a second time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxAction {
    pub id: String,
    pub label: String,
    /// `acknowledge`, `navigate`, `choose_option` or `dismiss`.
    pub kind: String,
    pub resolved: bool,
    /// The choices of a `choose_option` action; empty for the other kinds.
    #[serde(default)]
    pub options: Vec<InboxActionOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxMessage {
    pub id: String,
    pub subject: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub sender: String,
    #[serde(default)]
    pub sender_role: String,
    pub category: String,
    pub read: bool,
    pub date: String,
    #[serde(default)]
    pub actions: Vec<InboxAction>,
    /// The structured params of a message whose text carries a list the game builds as data (a
    /// roundup's results, a standings table). The sentence does not repeat them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub details: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxMessages {
    pub messages: Vec<InboxMessage>,
}

impl fmt::Display for InboxMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.messages.is_empty() {
            return write!(f, "## Inbox\n\nNo messages.");
        }
        write!(
            f,
            "## Inbox ({} messages)\n\n| ID | Subject | Category | Read | Date |\n|----|---------|----------|------|------|\n",
            self.messages.len()
        )?;
        for message in self.messages.iter().take(LISTED_IN_TEXT) {
            let read_marker = if message.read { "✓" } else { "●" };
            writeln!(
                f,
                "| {} | {} | {} | {} | {} |",
                message.id, message.subject, message.category, read_marker, message.date
            )?;
        }
        for message in self.messages.iter().take(LISTED_IN_TEXT) {
            write_message(f, message)?;
        }
        if self.messages.len() > LISTED_IN_TEXT {
            write!(
                f,
                "\n... and {} more.",
                self.messages.len() - LISTED_IN_TEXT
            )?;
        }
        Ok(())
    }
}

/// One message in full: who wrote it, what it says, and what it asks the manager to do.
fn write_message(f: &mut fmt::Formatter<'_>, message: &InboxMessage) -> fmt::Result {
    write!(f, "\n### {} ({})\n", message.subject, message.id)?;
    if !message.sender.is_empty() {
        write!(f, "\nFrom {}", message.sender)?;
        if !message.sender_role.is_empty() {
            write!(f, ", {}", message.sender_role)?;
        }
        writeln!(f)?;
    }
    write!(f, "\n{}\n", message.body)?;
    for (name, value) in &message.details {
        write!(f, "\nDetails `{name}`: {value}\n")?;
    }
    for action in &message.actions {
        let answered = if action.resolved {
            " (already answered)"
        } else {
            ""
        };
        write!(
            f,
            "\nAction `{}` [{}]{}: {}\n",
            action.id, action.kind, answered, action.label
        )?;
        for option in &action.options {
            writeln!(
                f,
                "  - option `{}`: {} — {}",
                option.id, option.label, option.description
            )?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivedMessage {
    pub id: String,
    pub subject: String,
    pub body: String,
    pub category: String,
    pub date: String,
}

/// A page of archived mail. When `next_before_date` is set, pass it with `next_before_id` as
/// `before_date` and `before_id` to read the next page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivedMessages {
    pub messages: Vec<ArchivedMessage>,
    pub next_before_date: Option<String>,
    pub next_before_id: Option<String>,
}

impl fmt::Display for ArchivedMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.messages.is_empty() {
            return write!(f, "## Archived inbox\n\nNo archived messages.");
        }
        write!(
            f,
            "## Archived inbox ({} messages)\n\n| ID | Subject | Category | Date |\n|----|---------|----------|------|\n",
            self.messages.len()
        )?;
        for message in &self.messages {
            writeln!(
                f,
                "| {} | {} | {} | {} |",
                message.id, message.subject, message.category, message.date
            )?;
        }
        for message in &self.messages {
            write!(
                f,
                "\n### {} ({})\n\n{}\n",
                message.subject, message.id, message.body
            )?;
        }
        if let (Some(date), Some(id)) = (&self.next_before_date, &self.next_before_id) {
            write!(
                f,
                "\nMore: call again with before_date={date} before_id={id}."
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageMarkedRead {
    pub message_id: String,
}

impl fmt::Display for MessageMarkedRead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message marked as read.")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllMessagesMarkedRead {}

impl fmt::Display for AllMessagesMarkedRead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "All messages marked as read.")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDeleted {
    pub message_id: String,
}

impl fmt::Display for MessageDeleted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message deleted.")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OldMessagesCleared {}

impl fmt::Display for OldMessagesCleared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Old messages cleared.")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionResolved {
    pub message_id: String,
    pub action_id: String,
}

impl fmt::Display for ActionResolved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "## Action Resolved\n\nMessage {} — action {} completed.",
            self.message_id, self.action_id
        )
    }
}

tool_results!(
    InboxMessages,
    ArchivedMessages,
    MessageMarkedRead,
    AllMessagesMarkedRead,
    MessageDeleted,
    OldMessagesCleared,
    ActionResolved,
);

#[cfg(test)]
mod tests {
    use super::*;

    /// Given a listed message with a choice that is still open
    /// When the inbox is rendered as text
    /// Then the text carries the message id, the action id and each option id an agent passes to
    ///      `inbox_resolve_action`, and the message's own words
    #[test]
    fn the_text_names_every_id_inbox_resolve_action_takes() {
        let inbox = InboxMessages {
            messages: vec![InboxMessage {
                id: "sponsor_1".to_string(),
                subject: "Sponsor offer".to_string(),
                body: "Acme wants to sponsor the club.".to_string(),
                sender: "Commercial Director".to_string(),
                sender_role: "Commercial Director".to_string(),
                category: "Finance".to_string(),
                read: false,
                date: "2026-07-30".to_string(),
                actions: vec![InboxAction {
                    id: "respond".to_string(),
                    label: "Respond".to_string(),
                    kind: "choose_option".to_string(),
                    resolved: false,
                    options: vec![InboxActionOption {
                        id: "accept".to_string(),
                        label: "Accept".to_string(),
                        description: "Sign the deal.".to_string(),
                    }],
                }],
                details: BTreeMap::new(),
            }],
        };

        let text = inbox.to_string();

        assert!(text.contains("Acme wants to sponsor the club."), "{text}");
        assert!(text.contains("sponsor_1"), "{text}");
        assert!(text.contains("Action `respond` [choose_option]"), "{text}");
        assert!(
            text.contains("option `accept`: Accept — Sign the deal."),
            "{text}"
        );
    }
}
