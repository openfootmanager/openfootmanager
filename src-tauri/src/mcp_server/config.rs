/// Configuration for the MCP server, parsed from CLI arguments.
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Port to listen on for MCP SSE connections.
    pub port: u16,

    /// Tool restriction mode: "sandbox" or "competition".
    pub mode: McpMode,

    /// Individual tools to disable (on top of mode restrictions).
    pub disabled_tools: Vec<String>,

    /// Path to world JSON + team ID for auto-bootstrap before MCP starts.
    /// Format: "world.json,team_abc123"
    pub auto_start: Option<AutoStartConfig>,

    /// Start without a GUI window (headless).
    pub no_gui: bool,

    /// Minimum delay between time_advance calls (ms).
    pub min_tick_delay_ms: u64,

    /// Auto-save every N in-game days (0 = disabled).
    pub auto_save_interval_days: u32,

    /// Manager first name for auto-start (default: "Agent").
    pub manager_name: Option<String>,

    /// Manager last name for auto-start (default: "Manager").
    pub manager_last_name: Option<String>,

    /// Manager nationality for auto-start (default: team's country).
    pub manager_nationality: Option<String>,

    /// Allowed hosts for DNS rebinding protection.
    pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AutoStartConfig {
    pub world_path: String,
    /// Team ID to manage. Required for RosterBaseline worlds or when the
    /// exported world's user manager has no team. Optional for HistoricalSnapshot
    /// worlds where the manager already has a team assigned.
    pub team_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpMode {
    Sandbox,
    Competition,
}

impl McpMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "sandbox" => Some(Self::Sandbox),
            "competition" => Some(Self::Competition),
            _ => None,
        }
    }

    /// Tools that are disabled in this mode (not registered at all).
    pub fn disabled_tools(&self) -> &'static [&'static str] {
        match self {
            Self::Sandbox => &[],
            Self::Competition => &[
                "game_new",
                "game_select_team",
                "game_export_world",
                "game_exit",
                "game_load_save",
            ],
        }
    }
}

/// Parse MCP-related CLI arguments from the process arguments.
///
/// Returns `Ok(None)` if `--mcp-port` is not present (i.e. MCP server should not start).
/// Returns `Err(msg)` if validation fails (e.g. invalid --mcp-mode, competition mode
/// without --mcp-auto-start).
pub fn parse_mcp_config_from_args() -> Result<Option<McpConfig>, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    parse_mcp_config_from_iter(args)
}

/// Core parsing logic shared by production and test entry points.
///
/// Accepts an iterator of string tokens (CLI arguments) and returns
/// `Ok(Some(McpConfig))` on success, `Ok(None)` if `--mcp-port` is absent,
/// or `Err(msg)` on validation failure.
fn parse_mcp_config_from_iter<I, S>(args: I) -> Result<Option<McpConfig>, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut port: Option<u16> = None;
    let mut mode: Option<McpMode> = None;
    let mut disabled_tools = Vec::new();
    let mut auto_start: Option<AutoStartConfig> = None;
    let mut no_gui = false;
    let mut min_tick_delay_ms: u64 = 0;
    let mut auto_save_interval_days: u32 = 7;
    let mut manager_name: Option<String> = None;
    let mut manager_last_name: Option<String> = None;
    let mut manager_nationality: Option<String> = None;

    let args: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mcp-port" => {
                i += 1;
                if i < args.len() {
                    match args[i].parse() {
                        Ok(p) => port = Some(p),
                        Err(_) => log::warn!("[mcp-config] Invalid --mcp-port '{}', ignoring", args[i]),
                    }
                }
            }
            "--mcp-mode" => {
                i += 1;
                if i < args.len() {
                    mode = Some(McpMode::parse(&args[i]).ok_or_else(|| {
                        format!("Invalid --mcp-mode '{}' (expected 'sandbox' or 'competition')", args[i])
                    })?);
                }
            }
            "--mcp-disable-tools" => {
                i += 1;
                if i < args.len() {
                    disabled_tools = args[i].split(',').map(|s| s.trim().to_string()).collect();
                }
            }
            "--mcp-auto-start" => {
                i += 1;
                if i < args.len() {
                    let parts: Vec<&str> = args[i].splitn(2, ',').collect();
                    auto_start = Some(AutoStartConfig {
                        world_path: parts[0].to_string(),
                        team_id: parts.get(1).map(|s| s.to_string()),
                    });
                }
            }
            "--no-gui" => {
                no_gui = true;
            }
            "--min-tick-delay-ms" => {
                i += 1;
                if i < args.len() {
                    match args[i].parse() {
                        Ok(val) => min_tick_delay_ms = val,
                        Err(_) => log::warn!("[mcp-config] Invalid --min-tick-delay-ms '{}', using default 0", args[i]),
                    }
                }
            }
            "--auto-save-interval-days" => {
                i += 1;
                if i < args.len() {
                    match args[i].parse() {
                        Ok(val) => auto_save_interval_days = val,
                        Err(_) => log::warn!("[mcp-config] Invalid --auto-save-interval-days '{}', using default 7", args[i]),
                    }
                }
            }
            "--manager-name" => {
                i += 1;
                if i < args.len() {
                    manager_name = Some(args[i].clone());
                }
            }
            "--manager-last-name" => {
                i += 1;
                if i < args.len() {
                    manager_last_name = Some(args[i].clone());
                }
            }
            "--manager-nationality" => {
                i += 1;
                if i < args.len() {
                    manager_nationality = Some(args[i].clone());
                }
            }
            _ => {}
        }
        i += 1;
    }

    let Some(p) = port else {
        return Ok(None);
    };

    let mode = mode.unwrap_or(McpMode::Sandbox);

    // Validate: competition mode requires --mcp-auto-start
    if mode == McpMode::Competition && auto_start.is_none() {
        return Err("--mcp-mode competition requires --mcp-auto-start (world.json[,team_id])".to_string());
    }

    Ok(Some(McpConfig {
        port: p,
        mode,
        disabled_tools,
        auto_start,
        no_gui,
        min_tick_delay_ms,
        auto_save_interval_days,
        manager_name,
        manager_last_name,
        manager_nationality,
        allowed_hosts: vec![
            "localhost".into(),
            "127.0.0.1".into(),
            "::1".into(),
        ],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mcp_config_no_args() {
        // No --mcp-port means no MCP server
        assert!(parse_mcp_config_from_iter::<Vec<String>, String>(vec![]).unwrap().is_none());
    }

    #[test]
    fn parse_mcp_config_with_port() {
        let config =
            parse_mcp_config_from_iter(&["--mcp-port", "3001"]).unwrap().expect("config");
        assert_eq!(config.port, 3001);
        assert_eq!(config.mode, McpMode::Sandbox);
        assert!(config.disabled_tools.is_empty());
        assert!(!config.no_gui);
    }

    #[test]
    fn parse_mcp_config_competition_mode() {
        let config = parse_mcp_config_from_iter(&[
            "--mcp-port",
            "3001",
            "--mcp-mode",
            "competition",
            "--mcp-auto-start",
            "world.json",
        ])
        .unwrap()
        .expect("config");
        assert_eq!(config.mode, McpMode::Competition);
        assert!(config.disabled_tools.is_empty());
        assert!(config.auto_start.is_some());
    }

    #[test]
    fn parse_mcp_config_all_args() {
        let config = parse_mcp_config_from_iter(&[
            "--mcp-port",
            "3001",
            "--mcp-mode",
            "competition",
            "--mcp-disable-tools",
            "club_upgrade_facility",
            "--mcp-auto-start",
            "world.json,team_abc123",
            "--no-gui",
            "--min-tick-delay-ms",
            "100",
            "--auto-save-interval-days",
            "14",
            "--manager-name",
            "Agent 1",
            "--manager-nationality",
            "England",
        ])
        .unwrap()
        .expect("config");
        assert_eq!(config.port, 3001);
        assert_eq!(config.mode, McpMode::Competition);
        assert_eq!(config.disabled_tools, vec!["club_upgrade_facility"]);
        let auto_start = config.auto_start.expect("auto_start");
        assert_eq!(auto_start.world_path, "world.json");
        assert_eq!(auto_start.team_id, Some("team_abc123".to_string()));
        assert!(config.no_gui);
        assert_eq!(config.min_tick_delay_ms, 100);
        assert_eq!(config.auto_save_interval_days, 14);
        assert_eq!(config.manager_name.as_deref(), Some("Agent 1"));
        assert_eq!(config.manager_nationality.as_deref(), Some("England"));
    }

    #[test]
    fn competition_mode_disabled_tools() {
        assert!(McpMode::Competition.disabled_tools().contains(&"game_new"));
        assert!(!McpMode::Competition.disabled_tools().contains(&"info_game_state"));
        assert!(McpMode::Sandbox.disabled_tools().is_empty());
    }

    #[test]
    fn competition_mode_without_auto_start_returns_err() {
        let result = parse_mcp_config_from_iter(&[
            "--mcp-port",
            "3001",
            "--mcp-mode",
            "competition",
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--mcp-mode competition requires --mcp-auto-start"));
    }

    #[test]
    fn invalid_mcp_mode_returns_err() {
        let result = parse_mcp_config_from_iter(&[
            "--mcp-port",
            "3001",
            "--mcp-mode",
            "invalid_mode",
        ]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid --mcp-mode"));
        assert!(err.contains("invalid_mode"));
    }
}
