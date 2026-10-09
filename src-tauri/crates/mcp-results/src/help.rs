//! Results of the help and utility tools.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tool_results;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    pub message: String,
}

impl fmt::Display for Pong {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSummary {
    pub name: String,
    pub category: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearch {
    pub query: String,
    pub tools: Vec<ToolSummary>,
}

impl fmt::Display for ToolSearch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.tools.is_empty() {
            return write!(
                f,
                "## Tool Search: '{}'\n\nNo tools found matching your query. Try `help_list_categories`.",
                self.query
            );
        }
        write!(
            f,
            "## Tool Search: '{}'\n\n| Tool | Category | Description |\n|------|----------|-------------|\n",
            self.query
        )?;
        for tool in &self.tools {
            writeln!(
                f,
                "| {} | {} | {} |",
                tool.name, tool.category, tool.description
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCategory {
    pub name: String,
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCategories {
    pub categories: Vec<ToolCategory>,
}

impl fmt::Display for ToolCategories {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "## Tool Categories\n\n")?;
        for category in &self.categories {
            write!(
                f,
                "**{}** ({} tools): {}\n\n",
                category.name,
                category.tools.len(),
                category.tools.join(", ")
            )?;
        }
        Ok(())
    }
}

tool_results!(Pong, ToolSearch, ToolCategories);
