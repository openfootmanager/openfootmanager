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

    /// Manager name for auto-start (default: "Agent N").
    pub manager_name: Option<String>,

    /// Manager nationality for auto-start (default: team's country).
    pub manager_nationality: Option<String>,

    /// Allowed hosts for DNS rebinding protection.
    pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AutoStartConfig {
    pub world_path: String,
    pub team_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpMode {
    Sandbox,
    Competition,
}

impl McpMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::Competition => "competition",
        }
    }

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
                "info_game_state",
            ],
        }
    }
}

/// Parse MCP-related CLI arguments from the process arguments.
///
/// Returns `None` if `--mcp-port` is not present (i.e. MCP server should not start).
pub fn parse_mcp_config_from_args() -> Option<McpConfig> {
    let args: Vec<String> = std::env::args().collect();
    let mut port: Option<u16> = None;
    let mut mode = McpMode::Sandbox;
    let mut disabled_tools = Vec::new();
    let mut auto_start: Option<AutoStartConfig> = None;
    let mut no_gui = false;
    let mut min_tick_delay_ms: u64 = 0;
    let mut auto_save_interval_days: u32 = 7;
    let mut manager_name: Option<String> = None;
    let mut manager_nationality: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mcp-port" => {
                i += 1;
                if i < args.len() {
                    port = args[i].parse().ok();
                }
            }
            "--mcp-mode" => {
                i += 1;
                if i < args.len() {
                    if let Some(m) = McpMode::parse(&args[i]) {
                        mode = m;
                    }
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
                    if parts.len() == 2 {
                        auto_start = Some(AutoStartConfig {
                            world_path: parts[0].to_string(),
                            team_id: parts[1].to_string(),
                        });
                    }
                }
            }
            "--no-gui" => {
                no_gui = true;
            }
            "--min-tick-delay-ms" => {
                i += 1;
                if i < args.len() {
                    min_tick_delay_ms = args[i].parse().unwrap_or(0);
                }
            }
            "--auto-save-interval-days" => {
                i += 1;
                if i < args.len() {
                    auto_save_interval_days = args[i].parse().unwrap_or(7);
                }
            }
            "--manager-name" => {
                i += 1;
                if i < args.len() {
                    manager_name = Some(args[i].clone());
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

    port.map(|p| McpConfig {
        port: p,
        mode,
        disabled_tools,
        auto_start,
        no_gui,
        min_tick_delay_ms,
        auto_save_interval_days,
        manager_name,
        manager_nationality,
        allowed_hosts: vec![
            "localhost".into(),
            "127.0.0.1".into(),
            "::1".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mcp_config_no_args() {
        // No --mcp-port means no MCP server
        std::env::set_var("OFM_TEST_ARGS", "");
        assert!(parse_mcp_config_from_test_args(&[]).is_none());
    }

    #[test]
    fn parse_mcp_config_with_port() {
        let config =
            parse_mcp_config_from_test_args(&["--mcp-port", "3001"]).expect("config");
        assert_eq!(config.port, 3001);
        assert_eq!(config.mode, McpMode::Sandbox);
        assert!(config.disabled_tools.is_empty());
        assert!(!config.no_gui);
    }

    #[test]
    fn parse_mcp_config_competition_mode() {
        let config = parse_mcp_config_from_test_args(&[
            "--mcp-port",
            "3001",
            "--mcp-mode",
            "competition",
        ])
        .expect("config");
        assert_eq!(config.mode, McpMode::Competition);
        assert!(config.disabled_tools.is_empty());
    }

    #[test]
    fn parse_mcp_config_all_args() {
        let config = parse_mcp_config_from_test_args(&[
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
        .expect("config");
        assert_eq!(config.port, 3001);
        assert_eq!(config.mode, McpMode::Competition);
        assert_eq!(config.disabled_tools, vec!["club_upgrade_facility"]);
        let auto_start = config.auto_start.expect("auto_start");
        assert_eq!(auto_start.world_path, "world.json");
        assert_eq!(auto_start.team_id, "team_abc123");
        assert!(config.no_gui);
        assert_eq!(config.min_tick_delay_ms, 100);
        assert_eq!(config.auto_save_interval_days, 14);
        assert_eq!(config.manager_name.as_deref(), Some("Agent 1"));
        assert_eq!(config.manager_nationality.as_deref(), Some("England"));
    }

    #[test]
    fn competition_mode_disabled_tools() {
        assert!(McpMode::Competition.disabled_tools().contains(&"game_new"));
        assert!(McpMode::Competition.disabled_tools().contains(&"info_game_state"));
        assert!(McpMode::Sandbox.disabled_tools().is_empty());
    }

    /// Helper for tests: parse from an explicit arg list instead of std::env::args()
    fn parse_mcp_config_from_test_args(args: &[&str]) -> Option<McpConfig> {
        let mut port: Option<u16> = None;
        let mut mode = McpMode::Sandbox;
        let mut disabled_tools = Vec::new();
        let mut auto_start: Option<AutoStartConfig> = None;
        let mut no_gui = false;
        let mut min_tick_delay_ms: u64 = 0;
        let mut auto_save_interval_days: u32 = 7;
        let mut manager_name: Option<String> = None;
        let mut manager_nationality: Option<String> = None;

        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--mcp-port" => {
                    i += 1;
                    if i < args.len() {
                        port = args[i].parse().ok();
                    }
                }
                "--mcp-mode" => {
                    i += 1;
                    if i < args.len() {
                        if let Some(m) = McpMode::parse(args[i]) {
                            mode = m;
                        }
                    }
                }
                "--mcp-disable-tools" => {
                    i += 1;
                    if i < args.len() {
                        disabled_tools = args[i]
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect();
                    }
                }
                "--mcp-auto-start" => {
                    i += 1;
                    if i < args.len() {
                        let parts: Vec<&str> = args[i].splitn(2, ',').collect();
                        if parts.len() == 2 {
                            auto_start = Some(AutoStartConfig {
                                world_path: parts[0].to_string(),
                                team_id: parts[1].to_string(),
                            });
                        }
                    }
                }
                "--no-gui" => {
                    no_gui = true;
                }
                "--min-tick-delay-ms" => {
                    i += 1;
                    if i < args.len() {
                        min_tick_delay_ms = args[i].parse().unwrap_or(0);
                    }
                }
                "--auto-save-interval-days" => {
                    i += 1;
                    if i < args.len() {
                        auto_save_interval_days = args[i].parse().unwrap_or(7);
                    }
                }
                "--manager-name" => {
                    i += 1;
                    if i < args.len() {
                        manager_name = Some(args[i].to_string());
                    }
                }
                "--manager-nationality" => {
                    i += 1;
                    if i < args.len() {
                        manager_nationality = Some(args[i].to_string());
                    }
                }
                _ => {}
            }
            i += 1;
        }

        port.map(|p| McpConfig {
            port: p,
            mode,
            disabled_tools,
            auto_start,
            no_gui,
            min_tick_delay_ms,
            auto_save_interval_days,
            manager_name,
            manager_nationality,
            allowed_hosts: vec![
                "localhost".into(),
                "127.0.0.1".into(),
                "::1".into(),
            ],
        })
    }
}
