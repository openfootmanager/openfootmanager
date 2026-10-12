//! MCP tool implementations: help

use mcp_results::help::{Pong, ToolCategories, ToolCategory, ToolSearch, ToolSummary};

use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools::tool_catalog;
use std::sync::Arc;

pub fn ping() -> Pong {
    Pong {
        message: "Pong! OpenFoot Manager MCP server is alive.".to_string(),
    }
}

// ─── help_find_tool ─────────────────────────────────────────────────────────

pub fn help_find_tool(_ctx: Arc<McpContext>, query: String) -> Result<ToolSearch, String> {
    let query_lower = query.to_lowercase();

    let tools = tool_catalog()
        .into_iter()
        .filter(|(name, desc, _cat)| {
            name.contains(&query_lower) || desc.to_lowercase().contains(&query_lower)
        })
        .map(|(name, description, category)| ToolSummary {
            name: name.to_string(),
            category: category.to_string(),
            description: description.to_string(),
        })
        .collect();

    Ok(ToolSearch { query, tools })
}

// ─── help_list_categories ───────────────────────────────────────────────────

pub fn help_list_categories() -> ToolCategories {
    // Grouped by category in the order each is first seen.
    let mut categories: Vec<ToolCategory> = Vec::new();
    for (name, _desc, category) in tool_catalog() {
        match categories.iter_mut().find(|entry| entry.name == category) {
            Some(entry) => entry.tools.push(name.to_string()),
            None => categories.push(ToolCategory {
                name: category.to_string(),
                tools: vec![name.to_string()],
            }),
        }
    }
    ToolCategories { categories }
}
