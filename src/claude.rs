use anyhow::{anyhow, Result};
use std::path::PathBuf;
use tracing::{debug, error, info};

use crate::tmux::TmuxManager;

pub struct ClaudeCodeManager {
    tmux: TmuxManager,
}

impl ClaudeCodeManager {
    pub fn new() -> Self {
        Self {
            tmux: TmuxManager::new(),
        }
    }

    pub fn start_claude_session(
        &self,
        session_name: &str,
        working_dir: Option<&PathBuf>,
        initial_message: &str,
    ) -> Result<()> {
        info!("Starting Claude Code session: {}", session_name);

        // Create tmux session with Claude Code
        let claude_command = if cfg!(debug_assertions) {
            // For development, you might want to use a different command
            "claude-code --dangerously-skip-permissions"
        } else {
            "claude-code --dangerously-skip-permissions"
        };

        self.tmux.create_session(session_name, working_dir, Some(claude_command))?;

        // Wait for Claude to initialize
        info!("Waiting for Claude Code to initialize...");
        std::thread::sleep(std::time::Duration::from_secs(5));

        // Send the initial message
        self.send_message_to_claude(session_name, initial_message)?;

        Ok(())
    }

    pub fn send_message_to_claude(&self, session_name: &str, message: &str) -> Result<()> {
        debug!("Sending message to Claude session {}: {}", session_name, message);

        // Send the message
        self.tmux.send_keys(session_name, message)?;
        
        // Wait a moment for the message to be processed
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        // Send Enter to execute
        self.tmux.send_enter(session_name)?;

        info!("Message sent to Claude Code session: {}", session_name);
        Ok(())
    }

    pub fn get_claude_output(&self, session_name: &str, lines: Option<usize>) -> Result<String> {
        debug!("Getting Claude output from session: {}", session_name);
        
        let output = self.tmux.capture_pane(session_name, lines)?;
        Ok(output)
    }

    pub fn is_claude_ready(&self, session_name: &str) -> Result<bool> {
        debug!("Checking if Claude is ready in session: {}", session_name);
        
        let output = self.get_claude_output(session_name, Some(10))?;
        
        // Look for Claude's prompt or ready indicators
        // This is a heuristic - you might need to adjust based on Claude Code's actual output
        let ready_indicators = [
            "claude-code>",
            "How can I help you",
            "What would you like me to help you with",
            "I'm ready to help",
        ];

        let is_ready = ready_indicators.iter().any(|indicator| {
            output.to_lowercase().contains(&indicator.to_lowercase())
        });

        debug!("Claude ready status for {}: {}", session_name, is_ready);
        Ok(is_ready)
    }

    pub fn wait_for_claude_completion(
        &self,
        session_name: &str,
        timeout_secs: u64,
    ) -> Result<String> {
        info!(
            "Waiting for Claude completion in session: {} (timeout: {}s)",
            session_name, timeout_secs
        );

        // Try hook-based completion detection first
        if let Ok(result) = self.wait_for_completion_hook(session_name, timeout_secs) {
            return Ok(result);
        }

        info!("Hook-based completion detection failed, falling back to heuristics");
        
        // Fallback to old method if hook-based detection fails
        self.wait_for_completion_heuristic(session_name, timeout_secs)
    }

    fn wait_for_completion_hook(&self, session_name: &str, timeout_secs: u64) -> Result<String> {
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let check_interval = std::time::Duration::from_millis(500); // Check more frequently
        
        let completion_file = format!("/tmp/claude-code-manager/{}.done", session_name);
        
        // Remove any existing completion file to start fresh
        let _ = std::fs::remove_file(&completion_file);
        
        info!("Monitoring completion file: {}", completion_file);
        
        loop {
            if start_time.elapsed() > timeout {
                return Err(anyhow!("Timeout waiting for Claude completion"));
            }
            
            // Check if completion file exists
            if std::path::Path::new(&completion_file).exists() {
                info!("Completion detected via hook file: {}", completion_file);
                
                // Give Claude a moment to finish writing output after the hook fires
                std::thread::sleep(std::time::Duration::from_millis(500));
                
                let final_output = self.get_claude_output(session_name, None)?;
                
                // Clean up the completion file
                let _ = std::fs::remove_file(&completion_file);
                
                return Ok(final_output);
            }
            
            std::thread::sleep(check_interval);
        }
    }

    fn wait_for_completion_heuristic(&self, session_name: &str, timeout_secs: u64) -> Result<String> {
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let check_interval = std::time::Duration::from_secs(3);

        let mut last_output = String::new();
        let mut stable_count = 0;
        let stability_threshold = 4; // Number of consecutive checks with same output

        loop {
            if start_time.elapsed() > timeout {
                error!("Timeout waiting for Claude completion");
                return Err(anyhow!("Timeout waiting for Claude completion"));
            }

            let current_output = self.get_claude_output(session_name, None)?;
            
            if current_output == last_output {
                stable_count += 1;
                if stable_count >= stability_threshold {
                    info!("Claude output appears stable, assuming completion");
                    return Ok(current_output);
                }
            } else {
                stable_count = 0;
                last_output = current_output;
            }

            // Additional heuristics for completion detection
            if self.looks_like_completion(&last_output) {
                info!("Claude completion detected based on output analysis");
                return Ok(last_output);
            }

            std::thread::sleep(check_interval);
        }
    }

    fn looks_like_completion(&self, output: &str) -> bool {
        // Check if Claude is still actively working
        let still_working_indicators = [
            "Wibbling…",
            "Synthesizing…", 
            "Writing…",
            "Thinking…",
            "Processing…",
            "⚒ 0 tokens", // Still starting
            "esc to interrupt", // Still working
        ];

        // If Claude is still working, definitely not complete
        let is_still_working = still_working_indicators.iter().any(|indicator| {
            output.contains(indicator)
        });

        if is_still_working {
            return false;
        }

        // Look for clear completion indicators
        let completion_indicators = [
            "Task completed",
            "Done!",
            "Finished",
            "✅",
            "✓",
        ];

        // Look for error indicators
        let error_indicators = [
            "Error:",
            "Failed:",
            "❌",
            "✗",
            "Exception:",
        ];

        // Check if it looks like Claude finished (either success or error)
        let has_completion = completion_indicators.iter().any(|indicator| {
            output.to_lowercase().contains(&indicator.to_lowercase())
        });

        let has_error = error_indicators.iter().any(|indicator| {
            output.to_lowercase().contains(&indicator.to_lowercase())
        });

        // Only rely on stability detection for most cases
        // Don't try to be too clever about detecting completion states
        // since Claude Code UI can be complex and variable
        
        // Only use very clear completion indicators
        has_completion || has_error
    }

    pub fn kill_claude_session(&self, session_name: &str) -> Result<()> {
        info!("Killing Claude Code session: {}", session_name);
        self.tmux.kill_session(session_name)
    }

    pub fn list_claude_sessions(&self) -> Result<Vec<String>> {
        debug!("Listing Claude Code sessions");
        
        let all_sessions = self.tmux.list_sessions()?;
        
        // Filter for sessions that are likely Claude Code sessions
        // This is a heuristic - you might want to adjust based on your naming convention
        let claude_sessions: Vec<String> = all_sessions
            .into_iter()
            .filter(|session| {
                session.starts_with("claude-") || 
                session.contains("claude") ||
                self.is_claude_session(session).unwrap_or(false)
            })
            .collect();

        Ok(claude_sessions)
    }

    fn is_claude_session(&self, session_name: &str) -> Result<bool> {
        // Try to get a small sample of the session output to determine if it's Claude
        match self.get_claude_output(session_name, Some(5)) {
            Ok(output) => {
                let claude_indicators = [
                    "claude-code",
                    "Claude",
                    "How can I help",
                    "I'm Claude",
                ];
                
                Ok(claude_indicators.iter().any(|indicator| {
                    output.to_lowercase().contains(&indicator.to_lowercase())
                }))
            }
            Err(_) => Ok(false),
        }
    }

    pub fn attach_to_session(&self, session_name: &str) -> Result<()> {
        info!("Attaching to Claude Code session: {}", session_name);
        self.tmux.attach_session(session_name)
    }
}