use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod claude;
mod session;
mod tmux;

use session::SessionManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    /// Skip Claude Code permissions checks (UNSAFE)
    #[serde(default)]
    pub skip_permissions: bool,

    /// Default timeout for operations in seconds
    #[serde(default = "default_timeout")]
    pub default_timeout: u64,

    /// Default session name
    #[serde(default = "default_session_name")]
    pub default_session_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            skip_permissions: false, // Safe by default
            default_timeout: 300,
            default_session_name: "claude-default".to_string(),
        }
    }
}

fn default_timeout() -> u64 {
    300
}

fn default_session_name() -> String {
    "claude-default".to_string()
}

fn load_config(config_path: Option<&PathBuf>) -> anyhow::Result<Config> {
    let config_file = if let Some(path) = config_path {
        path.clone()
    } else {
        // Default config location
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
            .join(".claude-code-manager")
            .join("config.json")
    };

    if config_file.exists() {
        let content = std::fs::read_to_string(&config_file)?;
        let config: Config = serde_json::from_str(&content)?;
        tracing::info!("Loaded config from: {}", config_file.display());
        Ok(config)
    } else {
        tracing::debug!(
            "No config file found at: {}, using defaults",
            config_file.display()
        );
        Ok(Config::default())
    }
}

fn create_default_config_file() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let config_dir = PathBuf::from(home).join(".claude-code-manager");
    let config_file = config_dir.join("config.json");

    std::fs::create_dir_all(&config_dir)?;

    let default_config = Config::default();
    let config_json = serde_json::to_string_pretty(&default_config)?;
    std::fs::write(&config_file, config_json)?;

    println!("Created default config file at: {}", config_file.display());
    Ok(config_file)
}

fn get_config_path(config_path: Option<&PathBuf>) -> PathBuf {
    if let Some(path) = config_path {
        path.clone()
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
            .join(".claude-code-manager")
            .join("config.json")
    }
}

async fn handle_config_command(
    config_command: &ConfigCommands,
    config_path: Option<&PathBuf>,
) -> anyhow::Result<()> {
    match config_command {
        ConfigCommands::Show => {
            let config = load_config(config_path)?;
            println!("Current configuration:");
            println!("{}", serde_json::to_string_pretty(&config)?);
        }

        ConfigCommands::Init => {
            create_default_config_file()?;
        }

        ConfigCommands::Get { key } => {
            let config = load_config(config_path)?;
            match key.as_str() {
                "skip-permissions" | "skip_permissions" => {
                    println!("{}", config.skip_permissions);
                }
                "default-timeout" | "default_timeout" => {
                    println!("{}", config.default_timeout);
                }
                "default-session-name" | "default_session_name" => {
                    println!("{}", config.default_session_name);
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Unknown config key: '{}'. Available keys: skip-permissions, default-timeout, default-session-name", 
                        key
                    ));
                }
            }
        }

        ConfigCommands::Set { key, value } => {
            let config_file = get_config_path(config_path);
            let mut config = if config_file.exists() {
                load_config(config_path)?
            } else {
                println!("Config file doesn't exist, creating new one...");
                Config::default()
            };

            match key.as_str() {
                "skip-permissions" | "skip_permissions" => {
                    let bool_value = match value.to_lowercase().as_str() {
                        "true" | "1" | "yes" | "on" => true,
                        "false" | "0" | "no" | "off" => false,
                        _ => {
                            return Err(anyhow::anyhow!(
                                "Invalid boolean value '{}'. Use: true/false, 1/0, yes/no, on/off",
                                value
                            ));
                        }
                    };
                    config.skip_permissions = bool_value;
                    println!("Set skip-permissions to: {}", config.skip_permissions);
                }
                "default-timeout" | "default_timeout" => {
                    let timeout_value: u64 = value.parse().map_err(|_| {
                        anyhow::anyhow!(
                            "Invalid timeout value '{}'. Must be a positive number",
                            value
                        )
                    })?;
                    config.default_timeout = timeout_value;
                    println!("Set default-timeout to: {}", config.default_timeout);
                }
                "default-session-name" | "default_session_name" => {
                    config.default_session_name = value.clone();
                    println!(
                        "Set default-session-name to: {}",
                        config.default_session_name
                    );
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Unknown config key: '{}'. Available keys: skip-permissions, default-timeout, default-session-name", 
                        key
                    ));
                }
            }

            // Create config directory if it doesn't exist
            if let Some(parent) = config_file.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Save the updated config
            let config_json = serde_json::to_string_pretty(&config)?;
            std::fs::write(&config_file, config_json)?;
            println!("Configuration saved to: {}", config_file.display());
        }
    }

    Ok(())
}

#[derive(Parser)]
#[command(name = "claude-code-manager")]
#[command(about = "A CLI tool to manage Claude Code sessions through tmux")]
#[command(version = "0.1.0")]
struct Cli {
    /// Skip Claude Code permissions checks (UNSAFE - use with caution)
    #[arg(long, global = true)]
    skip_permissions: bool,

    /// Path to config file (default: ~/.claude-code-manager/config.json)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new Claude Code session with a task
    Start {
        /// The message or task to send to Claude Code
        #[arg(short, long)]
        message: String,

        /// Custom session name (optional)
        #[arg(short, long)]
        session_name: Option<String>,

        /// Working directory for the Claude Code session
        #[arg(short, long)]
        working_dir: Option<PathBuf>,

        /// Wait for completion and return results
        #[arg(short, long)]
        wait: bool,

        /// Timeout in seconds (default: uses config)
        #[arg(short, long)]
        timeout: Option<u64>,
    },

    /// List all active Claude Code sessions
    List,

    /// Attach to an existing session
    Attach {
        /// Session name or ID
        session: String,
    },

    /// Send a message to a session (creates default session if none specified)
    Send {
        /// Message to send
        message: String,

        /// Session name or ID (default: creates/uses 'claude-default')
        #[arg(short, long)]
        session: Option<String>,

        /// Don't wait for completion (default: wait)
        #[arg(long)]
        no_wait: bool,

        /// Timeout in seconds (default: uses config)
        #[arg(short, long)]
        timeout: Option<u64>,
    },

    /// Get the status and output of a session
    Status {
        /// Session name or ID
        session: String,

        /// Number of lines to show from output (default: 50)
        #[arg(short, long, default_value = "50")]
        lines: usize,
    },

    /// Kill a Claude Code session
    Kill {
        /// Session name or ID
        session: String,
    },

    /// Kill all Claude Code sessions
    KillAll,

    /// View session history
    History {
        /// Session name or ID
        session: String,

        /// Number of lines to show (default: all)
        #[arg(short, long)]
        lines: Option<usize>,

        /// Follow the history (like tail -f)
        #[arg(short, long)]
        follow: bool,
    },

    /// Export session history to a file
    Export {
        /// Session name or ID
        session: String,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Remove ANSI color codes for clean text output
        #[arg(short, long)]
        clean: bool,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Initialize/create default configuration file
    Init,

    /// Set a configuration option
    Set {
        /// Configuration key to set
        key: String,
        /// Configuration value to set
        value: String,
    },

    /// Get a specific configuration value
    Get {
        /// Configuration key to get
        key: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Handle config command early
    if let Commands::Config { config_command } = &cli.command {
        handle_config_command(config_command, cli.config.as_ref()).await?;
        return Ok(());
    }

    // Load configuration
    let mut config = load_config(cli.config.as_ref())?;

    // Override config with CLI flags
    if cli.skip_permissions {
        config.skip_permissions = true;
    }

    let mut session_manager = SessionManager::new(config.clone());

    match cli.command {
        Commands::Start {
            message,
            session_name,
            working_dir,
            wait,
            timeout,
        } => {
            let session_name = session_manager
                .start_session(message, session_name, working_dir)
                .await?;

            println!("Started Claude Code session: {session_name}");

            if wait {
                println!("Waiting for completion...");
                let timeout = timeout.unwrap_or(config.default_timeout);
                let result = session_manager
                    .wait_for_completion(&session_name, timeout)
                    .await?;
                println!("Session completed:");
                println!("{result}");
            } else {
                println!("Session started in background. Use 'claude-code-manager attach {session_name}' to connect.");
            }
        }

        Commands::List => {
            let sessions = session_manager.list_sessions().await?;
            if sessions.is_empty() {
                println!("No active Claude Code sessions.");
            } else {
                println!("Active Claude Code sessions:");
                for session in sessions {
                    println!("  {} ({})", session.name, session.status);
                }
            }
        }

        Commands::Attach { session } => {
            session_manager.attach_session(&session).await?;
        }

        Commands::Send {
            message,
            session,
            no_wait,
            timeout,
        } => {
            let session_name = session.unwrap_or_else(|| config.default_session_name.clone());

            // Ensure the default session exists
            if !session_manager.session_exists(&session_name).await? {
                println!("Creating default Claude Code session...");
                session_manager
                    .start_session(
                        "Ready for commands".to_string(),
                        Some(session_name.clone()),
                        None,
                    )
                    .await?;
                println!("Default session '{session_name}' created.");
            }

            session_manager
                .send_message(&session_name, &message)
                .await?;

            if no_wait {
                println!("Message sent to session: {session_name}");
            } else {
                println!("Waiting for completion...");
                let timeout = timeout.unwrap_or(config.default_timeout);
                let result = session_manager
                    .wait_for_completion(&session_name, timeout)
                    .await?;
                println!("{result}");
            }
        }

        Commands::Status { session, lines } => {
            let status = session_manager.get_session_status(&session, lines).await?;
            println!("Session status for '{session}':");
            println!("{status}");
        }

        Commands::Kill { session } => {
            session_manager.kill_session(&session).await?;
            println!("Killed session: {session}");
        }

        Commands::KillAll => {
            let count = session_manager.kill_all_sessions().await?;
            println!("Killed {count} session(s)");
        }

        Commands::History {
            session,
            lines,
            follow,
        } => {
            if follow {
                session_manager.follow_session_history(&session).await?;
            } else {
                let history = session_manager.get_session_history(&session, lines).await?;
                println!("Session history for '{}':", session);
                println!("{}", history);
            }
        }

        Commands::Export {
            session,
            output,
            clean,
        } => {
            session_manager
                .export_session_history(&session, &output, clean)
                .await?;
            println!(
                "Exported session '{}' history to: {}",
                session,
                output.display()
            );
        }

        Commands::Config { .. } => {
            // This should never be reached because Config is handled early
            unreachable!("Config command should be handled before this match")
        }
    }

    Ok(())
}
