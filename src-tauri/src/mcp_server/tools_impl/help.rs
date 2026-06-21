//! MCP tool implementations: help

use std::sync::Arc;
use crate::mcp_server::context::McpContext;
use crate::mcp_server::tools::tool_catalog;

// ─── help_find_tool ─────────────────────────────────────────────────────────

pub fn help_find_tool(_ctx: Arc<McpContext>, query: String) -> Result<String, String> {
    let query_lower = query.to_lowercase();
    let catalog = tool_catalog();

    let matches: Vec<_> = catalog.iter()
        .filter(|(name, desc, _cat)| name.contains(&query_lower) || desc.to_lowercase().contains(&query_lower))
        .collect();

    if matches.is_empty() {
        return Ok(format!("## Tool Search: '{}'\n\nNo tools found matching your query. Try `help_list_categories`.", query));
    }

    let mut output = format!("## Tool Search: '{}'\n\n| Tool | Category | Description |\n|------|----------|-------------|\n", query);
    for (name, desc, cat) in matches {
        output.push_str(&format!("| {} | {} | {} |\n", name, cat, desc));
    }

    Ok(output)
}

// ─── help_list_categories ───────────────────────────────────────────────────

pub fn help_list_categories() -> String {
    let catalog = tool_catalog();

    // Group by category preserving first-seen order
    let mut categories: Vec<(&str, Vec<(&str, &str)>)> = Vec::new();
    for (name, desc, cat) in &catalog {
        if let Some(entry) = categories.iter_mut().find(|(c, _)| c == cat) {
            entry.1.push((*name, *desc));
        } else {
            categories.push((*cat, vec![(*name, *desc)]));
        }
    }

    let mut output = String::from("## Tool Categories\n\n");
    for (cat, tools) in &categories {
        output.push_str(&format!("**{}** ({} tools): {}\n\n", cat, tools.len(), tools.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")));
    }

    output
}
