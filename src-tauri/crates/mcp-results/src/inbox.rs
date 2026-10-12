//! Results of the inbox tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

/// Messages the text lists before it summarises the rest; the structure always carries all of them.
const LISTED_IN_TEXT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxMessage {
    pub id: String,
    pub subject: String,
    pub category: String,
    pub read: bool,
    pub date: String,
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
    MessageMarkedRead,
    AllMessagesMarkedRead,
    MessageDeleted,
    OldMessagesCleared,
    ActionResolved,
);
