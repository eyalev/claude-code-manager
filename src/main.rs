use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod session;
mod tmux;
mod claude;

use session::SessionManager;

#[derive(Parser)]
#[command(name = "claude-code-manager")]
#[command(about = "A CLI tool to manage Claude Code sessions through tmux")]
#[command(version = "0.1.0")]
struct Cli {
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
        
        /// Timeout in seconds (default: 300)
        #[arg(short, long, default_value = "300")]
        timeout: u64,
    },
    
    /// List all active Claude Code sessions
    List,
    
    /// Attach to an existing session
    Attach {
        /// Session name or ID
        session: String,
    },
    
    /// Send a message to an existing session
    Send {
        /// Session name or ID
        session: String,
        
        /// Message to send
        message: String,
        
        /// Wait for completion and return results
        #[arg(short, long)]
        wait: bool,
        
        /// Timeout in seconds (default: 300)
        #[arg(short, long, default_value = "300")]
        timeout: u64,
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    let mut session_manager = SessionManager::new();
    
    match cli.command {
        Commands::Start { 
            message, 
            session_name, 
            working_dir, 
            wait, 
            timeout 
        } => {
            let session_name = session_manager.start_session(
                message,
                session_name,
                working_dir,
            ).await?;
            
            println!("Started Claude Code session: {}", session_name);
            
            if wait {
                println!("Waiting for completion...");
                let result = session_manager.wait_for_completion(&session_name, timeout).await?;
                println!("Session completed:");
                println!("{}", result);
            } else {
                println!("Session started in background. Use 'claude-code-manager attach {}' to connect.", session_name);
            }
        },
        
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
        },
        
        Commands::Attach { session } => {
            session_manager.attach_session(&session).await?;
        },
        
        Commands::Send { session, message, wait, timeout } => {
            session_manager.send_message(&session, &message).await?;
            println!("Message sent to session: {}", session);
            
            if wait {
                println!("Waiting for completion...");
                let result = session_manager.wait_for_completion(&session, timeout).await?;
                println!("Response:");
                println!("{}", result);
            }
        },
        
        Commands::Status { session, lines } => {
            let status = session_manager.get_session_status(&session, lines).await?;
            println!("Session status for '{}':", session);
            println!("{}", status);
        },
        
        Commands::Kill { session } => {
            session_manager.kill_session(&session).await?;
            println!("Killed session: {}", session);
        },
        
        Commands::KillAll => {
            let count = session_manager.kill_all_sessions().await?;
            println!("Killed {} session(s)", count);
        },
        
        Commands::History { session, lines, follow } => {
            if follow {
                session_manager.follow_session_history(&session).await?;
            } else {
                let history = session_manager.get_session_history(&session, lines).await?;
                println!("Session history for '{}':", session);
                println!("{}", history);
            }
        },
        
        Commands::Export { session, output, clean } => {
            session_manager.export_session_history(&session, &output, clean).await?;
            println!("Exported session '{}' history to: {}", session, output.display());
        },
    }
    
    Ok(())
}