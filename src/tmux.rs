use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, error, info, warn};

pub struct TmuxManager;

impl TmuxManager {
    pub fn new() -> Self {
        Self
    }

    pub fn session_exists(&self, session_name: &str) -> Result<bool> {
        debug!("Checking if tmux session exists: {}", session_name);
        
        let output = Command::new("tmux")
            .args(["has-session", "-t", session_name])
            .output()?;

        Ok(output.status.success())
    }

    pub fn list_sessions(&self) -> Result<Vec<String>> {
        debug!("Listing tmux sessions");
        
        let output = Command::new("tmux")
            .args(["list-sessions", "-F", "#{session_name}"])
            .output()?;

        if !output.status.success() {
            // No sessions exist
            return Ok(vec![]);
        }

        let sessions = String::from_utf8(output.stdout)?
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        Ok(sessions)
    }

    pub fn create_session(
        &self,
        session_name: &str,
        working_dir: Option<&PathBuf>,
        command: Option<&str>,
    ) -> Result<()> {
        self.create_session_with_logging(session_name, working_dir, command, true)
    }

    pub fn create_session_with_logging(
        &self,
        session_name: &str,
        working_dir: Option<&PathBuf>,
        command: Option<&str>,
        enable_logging: bool,
    ) -> Result<()> {
        info!("Creating tmux session: {}", session_name);

        // Kill existing session if it exists
        if self.session_exists(session_name)? {
            warn!("Session {} already exists, killing it first", session_name);
            self.kill_session(session_name)?;
        }

        let mut cmd = Command::new("tmux");
        cmd.args(["new-session", "-d", "-s", session_name]);

        if let Some(dir) = working_dir {
            cmd.args(["-c", &dir.to_string_lossy()]);
        }

        if let Some(command) = command {
            cmd.arg(command);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to create tmux session: {}", stderr);
            return Err(anyhow!("Failed to create tmux session: {}", stderr));
        }

        info!("Successfully created tmux session: {}", session_name);
        
        // Enable logging if requested
        if enable_logging {
            self.enable_session_logging(session_name)?;
        }
        
        Ok(())
    }

    pub fn kill_session(&self, session_name: &str) -> Result<()> {
        debug!("Killing tmux session: {}", session_name);

        let output = Command::new("tmux")
            .args(["kill-session", "-t", session_name])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't error if session doesn't exist
            if stderr.contains("no server running") || stderr.contains("session not found") {
                debug!("Session {} doesn't exist or no tmux server running", session_name);
                return Ok(());
            }
            error!("Failed to kill tmux session: {}", stderr);
            return Err(anyhow!("Failed to kill tmux session: {}", stderr));
        }

        info!("Successfully killed tmux session: {}", session_name);
        Ok(())
    }

    pub fn send_keys(&self, session_name: &str, keys: &str) -> Result<()> {
        debug!("Sending keys to tmux session {}: {}", session_name, keys);

        let output = Command::new("tmux")
            .args(["send-keys", "-t", session_name, keys])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to send keys to tmux session: {}", stderr);
            return Err(anyhow!("Failed to send keys to tmux session: {}", stderr));
        }

        Ok(())
    }

    pub fn send_enter(&self, session_name: &str) -> Result<()> {
        debug!("Sending Enter to tmux session: {}", session_name);

        let output = Command::new("tmux")
            .args(["send-keys", "-t", session_name, "C-m"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to send Enter to tmux session: {}", stderr);
            return Err(anyhow!("Failed to send Enter to tmux session: {}", stderr));
        }

        Ok(())
    }

    pub fn capture_pane(&self, session_name: &str, lines: Option<usize>) -> Result<String> {
        debug!("Capturing pane content from tmux session: {}", session_name);

        let mut cmd = Command::new("tmux");
        cmd.args(["capture-pane", "-t", session_name, "-p"]);

        if let Some(lines) = lines {
            cmd.args(["-S", &format!("-{}", lines)]);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to capture pane from tmux session: {}", stderr);
            return Err(anyhow!("Failed to capture pane from tmux session: {}", stderr));
        }

        let content = String::from_utf8(output.stdout)?;
        Ok(content)
    }

    pub fn attach_session(&self, session_name: &str) -> Result<()> {
        info!("Attaching to tmux session: {}", session_name);

        let output = Command::new("tmux")
            .args(["attach-session", "-t", session_name])
            .status()?;

        if !output.success() {
            error!("Failed to attach to tmux session: {}", session_name);
            return Err(anyhow!("Failed to attach to tmux session: {}", session_name));
        }

        Ok(())
    }

    pub fn get_session_info(&self, session_name: &str) -> Result<SessionInfo> {
        debug!("Getting session info for: {}", session_name);

        let output = Command::new("tmux")
            .args([
                "display-message",
                "-t",
                session_name,
                "-p",
                "#{session_name}:#{session_created}:#{session_windows}:#{session_attached}"
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to get session info: {}", stderr);
            return Err(anyhow!("Failed to get session info: {}", stderr));
        }

        let info_str = String::from_utf8(output.stdout)?;
        let parts: Vec<&str> = info_str.trim().split(':').collect();

        if parts.len() != 4 {
            return Err(anyhow!("Unexpected session info format: {}", info_str));
        }

        Ok(SessionInfo {
            name: parts[0].to_string(),
            created: parts[1].parse().unwrap_or(0),
            windows: parts[2].parse().unwrap_or(0),
            attached: parts[3] == "1",
        })
    }

    pub fn enable_session_logging(&self, session_name: &str) -> Result<()> {
        debug!("Enabling logging for tmux session: {}", session_name);
        
        let log_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let log_file = format!("{}/.claude-code-manager/logs/{}.log", log_dir, session_name);
        
        // Create log directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&log_file).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Enable tmux logging for the session
        let output = Command::new("tmux")
            .args([
                "pipe-pane", 
                "-t", session_name, 
                &format!("cat >> '{}'", log_file)
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to enable logging for tmux session: {}", stderr);
            return Err(anyhow!("Failed to enable logging for tmux session: {}", stderr));
        }

        info!("Enabled logging for session {} to: {}", session_name, log_file);
        Ok(())
    }

    pub fn get_log_file_path(&self, session_name: &str) -> String {
        let log_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/.claude-code-manager/logs/{}.log", log_dir, session_name)
    }

    pub fn read_session_log(&self, session_name: &str, lines: Option<usize>) -> Result<String> {
        let log_file = self.get_log_file_path(session_name);
        
        if !std::path::Path::new(&log_file).exists() {
            debug!("Log file does not exist for session: {}", session_name);
            // Fall back to capturing current pane content
            return self.capture_pane(session_name, lines);
        }

        debug!("Reading log file: {}", log_file);
        
        if let Some(lines) = lines {
            // Read only the last N lines
            let output = Command::new("tail")
                .args(["-n", &lines.to_string(), &log_file])
                .output()?;
                
            if !output.status.success() {
                return Err(anyhow!("Failed to read log file: {}", log_file));
            }
            
            Ok(String::from_utf8(output.stdout)?)
        } else {
            // Read entire file
            Ok(std::fs::read_to_string(&log_file)?)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub name: String,
    pub created: u64,
    pub windows: u32,
    pub attached: bool,
}