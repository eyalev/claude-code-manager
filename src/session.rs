use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

use crate::claude::ClaudeCodeManager;
use crate::tmux::TmuxManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,           // This will be the tmux session name
    pub name: String,         // Display name (same as id for simplicity)
    pub working_dir: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Idle,
    Failed,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Active => write!(f, "active"),
            SessionStatus::Idle => write!(f, "idle"),
            SessionStatus::Failed => write!(f, "failed"),
        }
    }
}

pub struct SessionManager {
    claude: ClaudeCodeManager,
    tmux: TmuxManager,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            claude: ClaudeCodeManager::new(),
            tmux: TmuxManager::new(),
        }
    }

    pub async fn start_session(
        &mut self,
        message: String,
        session_name: Option<String>,
        working_dir: Option<PathBuf>,
    ) -> Result<String> {
        // Generate session name
        let session_name = session_name.unwrap_or_else(|| {
            let timestamp = chrono::Utc::now().format("%m%d-%H%M%S");
            format!("claude-{}", timestamp)
        });

        info!("Starting new Claude Code session: {}", session_name);

        // Start the Claude Code session
        match self.claude.start_claude_session(
            &session_name,
            working_dir.as_ref(),
            &message,
        ) {
            Ok(_) => {
                info!("Successfully started Claude Code session: {}", session_name);
                Ok(session_name)
            }
            Err(e) => {
                error!("Failed to start Claude Code session: {}", e);
                Err(e)
            }
        }
    }

    pub async fn list_sessions(&mut self) -> Result<Vec<Session>> {
        debug!("Listing all Claude Code sessions");

        let claude_sessions = self.claude.list_claude_sessions()?;
        let mut sessions = Vec::new();

        for session_name in claude_sessions {
            // Get tmux session info if available
            let status = if self.tmux.session_exists(&session_name)? {
                SessionStatus::Active
            } else {
                SessionStatus::Failed
            };

            let session = Session {
                id: session_name.clone(),
                name: session_name,
                working_dir: None, // We don't track this for existing sessions
                created_at: Utc::now(), // We don't have the real creation time
                status,
            };

            sessions.push(session);
        }

        Ok(sessions)
    }

    pub async fn session_exists(&mut self, session_name: &str) -> Result<bool> {
        Ok(self.tmux.session_exists(session_name)?)
    }

    pub async fn send_message(&mut self, session_name: &str, message: &str) -> Result<()> {
        info!("Sending message to session {}: {}", session_name, message);

        // Check if session exists
        if !self.tmux.session_exists(session_name)? {
            return Err(anyhow!("Session not found: {}", session_name));
        }

        match self.claude.send_message_to_claude(session_name, message) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to send message to session {}: {}", session_name, e);
                Err(e)
            }
        }
    }

    pub async fn wait_for_completion(&mut self, session_name: &str, timeout: u64) -> Result<String> {
        info!(
            "Waiting for completion of session {} (timeout: {}s)",
            session_name, timeout
        );

        // Check if session exists
        if !self.tmux.session_exists(session_name)? {
            return Err(anyhow!("Session not found: {}", session_name));
        }

        match self.claude.wait_for_claude_completion(session_name, timeout) {
            Ok(output) => Ok(output),
            Err(e) => {
                error!("Session {} did not complete within timeout: {}", session_name, e);
                Err(e)
            }
        }
    }

    pub async fn get_session_status(&mut self, session_name: &str, lines: usize) -> Result<String> {
        debug!("Getting status for session: {}", session_name);

        // Check if session exists
        if !self.tmux.session_exists(session_name)? {
            return Err(anyhow!("Session not found: {}", session_name));
        }

        match self.claude.get_claude_output(session_name, Some(lines)) {
            Ok(output) => Ok(output),
            Err(e) => {
                error!("Failed to get status for session {}: {}", session_name, e);
                Err(e)
            }
        }
    }

    pub async fn attach_session(&mut self, session_name: &str) -> Result<()> {
        info!("Attaching to session: {}", session_name);

        // Check if session exists
        if !self.tmux.session_exists(session_name)? {
            return Err(anyhow!("Session not found: {}", session_name));
        }

        match self.claude.attach_to_session(session_name) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to attach to session {}: {}", session_name, e);
                Err(e)
            }
        }
    }

    pub async fn kill_session(&mut self, session_name: &str) -> Result<()> {
        info!("Killing session: {}", session_name);

        // Check if session exists
        if !self.tmux.session_exists(session_name)? {
            return Err(anyhow!("Session not found: {}", session_name));
        }

        match self.claude.kill_claude_session(session_name) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to kill session {}: {}", session_name, e);
                Err(e)
            }
        }
    }

    pub async fn kill_all_sessions(&mut self) -> Result<usize> {
        info!("Killing all Claude Code sessions");

        let claude_sessions = self.claude.list_claude_sessions()?;
        let mut killed_count = 0;

        for session_name in claude_sessions {
            if self.kill_session(&session_name).await.is_ok() {
                killed_count += 1;
            }
        }

        Ok(killed_count)
    }

    pub async fn get_session_history(&mut self, session_name: &str, lines: Option<usize>) -> Result<String> {
        debug!("Getting history for session: {}", session_name);

        // Check if session exists
        if !self.tmux.session_exists(session_name)? {
            return Err(anyhow!("Session not found: {}", session_name));
        }

        // Try to read from log file first, then fall back to current pane content
        match self.tmux.read_session_log(session_name, lines) {
            Ok(history) => Ok(history),
            Err(e) => {
                debug!("Failed to read log file, falling back to pane capture: {}", e);
                self.claude.get_claude_output(session_name, lines)
            }
        }
    }

    pub async fn follow_session_history(&mut self, session_name: &str) -> Result<()> {
        info!("Following history for session: {}", session_name);

        // Check if session exists
        if !self.tmux.session_exists(session_name)? {
            return Err(anyhow!("Session not found: {}", session_name));
        }

        let log_file = self.tmux.get_log_file_path(session_name);
        
        if std::path::Path::new(&log_file).exists() {
            // Use tail -f on the log file
            let mut cmd = std::process::Command::new("tail");
            cmd.args(["-f", &log_file]);
            
            let status = cmd.status()?;
            if !status.success() {
                return Err(anyhow!("Failed to follow log file: {}", log_file));
            }
        } else {
            println!("No log file found for session '{}'. Showing current content:", session_name);
            let content = self.claude.get_claude_output(session_name, None)?;
            println!("{}", content);
        }

        Ok(())
    }

    pub async fn export_session_history(&mut self, session_name: &str, output_path: &std::path::Path, clean: bool) -> Result<()> {
        info!("Exporting history for session {} to: {}", session_name, output_path.display());

        // Check if session exists
        if !self.tmux.session_exists(session_name)? {
            return Err(anyhow!("Session not found: {}", session_name));
        }

        // Get full session history
        let mut history = self.get_session_history(session_name, None).await?;
        
        // Strip ANSI codes if clean output requested
        if clean {
            history = self.strip_ansi_codes(&history);
        }
        
        // Create output directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Write to file
        std::fs::write(output_path, history)?;
        
        info!("Successfully exported session history to: {}", output_path.display());
        Ok(())
    }

    pub async fn enable_logging_for_existing_sessions(&mut self) -> Result<()> {
        info!("Enabling logging for existing sessions");
        
        let claude_sessions = self.claude.list_claude_sessions()?;
        
        for session_name in claude_sessions {
            if let Err(e) = self.tmux.enable_session_logging(&session_name) {
                warn!("Failed to enable logging for session {}: {}", session_name, e);
            }
        }
        
        Ok(())
    }

    fn strip_ansi_codes(&self, text: &str) -> String {
        // Remove ANSI escape sequences using regex-like pattern matching
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Found escape character, skip until we find 'm' (end of color code)
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['
                    while let Some(next_ch) = chars.next() {
                        if next_ch == 'm' {
                            break;
                        }
                    }
                }
            } else {
                result.push(ch);
            }
        }
        
        result
    }
}