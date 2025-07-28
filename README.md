# Claude Code Manager

A powerful Rust CLI tool for managing [Claude Code](https://claude.ai/code) sessions through tmux. Solve the problem of no built-in API/CLI for Claude Code interactive control by providing comprehensive session management, message sending, and completion detection.

## Features

### 🚀 Session Management
- **Start**: Create new Claude Code sessions with custom names and working directories
- **List**: View all active sessions with status information
- **Attach**: Connect to existing sessions interactively
- **Kill**: Terminate individual sessions or all sessions at once

### 💬 Message Handling
- **Smart Sending**: Send messages to sessions with automatic completion detection
- **Hybrid Detection**: Uses Claude Code stop hooks for reliable completion detection with heuristic fallback
- **Default Sessions**: Automatically creates and manages default sessions for quick usage

### 📋 History & Logging
- **Automatic Logging**: All session activity logged via tmux pipe-pane
- **History Viewing**: Review session history with configurable line limits
- **Export**: Save session logs to files with optional ANSI code stripping
- **Live Following**: Follow session output in real-time like `tail -f`

### ⚙️ Configuration Management
- **Safe Defaults**: Secure by default (no dangerous permissions)
- **CLI Config**: Easy configuration management with `config` subcommand
- **Flexible Settings**: Configure permissions, timeouts, and session names
- **Multiple Formats**: Boolean values accept `true/false`, `1/0`, `yes/no`, `on/off`

### 🔒 Security
- **Safe by Default**: Does not use `--dangerously-skip-permissions` by default
- **Configurable**: Optional unsafe mode via CLI flag or config file
- **Clear Warnings**: Shows warnings when running in unsafe mode

## Installation

### From Source
```bash
git clone https://github.com/YOUR_USERNAME/claude-code-manager.git
cd claude-code-manager
cargo build --release
```

The binary will be available at `target/release/claude-code-manager`.

### Using Cargo
```bash
cargo install --path .
```

## Quick Start

1. **Initialize configuration**:
   ```bash
   claude-code-manager config init
   ```

2. **Send a message to the default session**:
   ```bash
   claude-code-manager send "Hello, Claude!"
   ```

3. **List active sessions**:
   ```bash
   claude-code-manager list
   ```

4. **View session history**:
   ```bash
   claude-code-manager history claude-default
   ```

## Usage

### Session Commands

#### Start a New Session
```bash
# Basic usage
claude-code-manager start -m "Create a Python script"

# With custom session name and working directory
claude-code-manager start -m "Debug the app" -s debug-session -w /path/to/project

# Wait for completion and show results
claude-code-manager start -m "Fix the bug" --wait
```

#### Send Messages
```bash
# Send to default session (creates if doesn't exist)
claude-code-manager send "Write a test for this function"

# Send to specific session
claude-code-manager send "Review this code" -s my-session

# Send without waiting for completion
claude-code-manager send "Start the server" --no-wait
```

#### List and Manage Sessions
```bash
# List all active sessions
claude-code-manager list

# Attach to a session (interactive)
claude-code-manager attach my-session

# Kill a specific session
claude-code-manager kill my-session

# Kill all sessions
claude-code-manager kill-all
```

#### History and Status
```bash
# Get current session status
claude-code-manager status my-session

# View session history
claude-code-manager history my-session

# View last 100 lines
claude-code-manager history my-session -l 100

# Follow history in real-time
claude-code-manager history my-session --follow

# Export history to file
claude-code-manager export my-session -o session.log

# Export with clean text (no ANSI codes)
claude-code-manager export my-session -o clean.txt --clean
```

### Configuration Management

#### View Configuration
```bash
# Show current configuration
claude-code-manager config show

# Get specific setting
claude-code-manager config get skip-permissions
```

#### Modify Settings
```bash
# Enable unsafe mode (use with caution!)
claude-code-manager config set skip-permissions true

# Set custom timeout (in seconds)
claude-code-manager config set default-timeout 600

# Change default session name
claude-code-manager config set default-session-name my-claude

# Disable unsafe mode (recommended)
claude-code-manager config set skip-permissions false
```

#### Available Configuration Keys
- `skip-permissions`: Enable/disable `--dangerously-skip-permissions` (boolean)
- `default-timeout`: Default timeout for operations in seconds (number)
- `default-session-name`: Default name for auto-created sessions (string)

### Global Options

```bash
# Use custom config file
claude-code-manager --config /path/to/config.json <command>

# Enable unsafe mode for single command (use with caution!)
claude-code-manager --skip-permissions send "test message"
```

## Configuration File

Configuration is stored in `~/.claude-code-manager/config.json`:

```json
{
  "skip_permissions": false,
  "default_timeout": 300,
  "default_session_name": "claude-default"
}
```

## How It Works

### Completion Detection
The tool uses a hybrid approach for detecting when Claude Code completes a task:

1. **Primary (Hook-based)**: Uses Claude Code's stop hooks to create completion marker files
2. **Fallback (Heuristic)**: Monitors output stability and looks for completion indicators

### Claude Code Stop Hook
Add this to your `~/.claude/settings.json` to enable hook-based completion detection:

```json
{
  "hooks": {
    "stop": [
      {
        "type": "command",
        "command": "mkdir -p /tmp/claude-code-manager && echo \"$(date -Iseconds)\" > \"/tmp/claude-code-manager/$(tmux display-message -p '#{session_name}' 2>/dev/null || echo 'unknown').done\""
      }
    ]
  }
}
```

### Session Management
- Sessions are managed through tmux with automatic logging enabled
- Each session gets a unique log file in `~/.claude-code-manager/logs/`
- Session persistence survives tool restarts and system reboots

## Examples

### Development Workflow
```bash
# Start a coding session
claude-code-manager start -m "Help me build a REST API" -s api-dev -w ~/projects/api

# Send follow-up messages
claude-code-manager send "Add authentication middleware" -s api-dev
claude-code-manager send "Write unit tests" -s api-dev

# Review what happened
claude-code-manager history api-dev -l 50

# Export the session for documentation
claude-code-manager export api-dev -o api-development-log.txt --clean
```

### Quick Tasks
```bash
# Quick one-off tasks using default session
claude-code-manager send "Explain this error message: ImportError: No module named 'requests'"
claude-code-manager send "Write a Python function to parse CSV files"
claude-code-manager send "How do I configure nginx for reverse proxy?"
```

### Batch Operations
```bash
# Kill all sessions and start fresh
claude-code-manager kill-all
claude-code-manager send "Ready for new tasks"

# Enable unsafe mode temporarily for operations requiring file access
claude-code-manager config set skip-permissions true
claude-code-manager send "Create a project structure in ~/new-project"
claude-code-manager config set skip-permissions false
```

## Security Considerations

⚠️ **Important**: This tool runs Claude Code with normal permissions by default. Only enable `skip-permissions` when you need Claude Code to perform actions that require elevated privileges.

- **Safe by Default**: The tool defaults to `skip_permissions: false`
- **Explicit Consent**: Unsafe mode must be explicitly enabled via config or CLI flag
- **Clear Warnings**: Shows warnings when running in unsafe mode
- **Easy Toggle**: Can quickly enable/disable unsafe mode through config commands

## Troubleshooting

### Sessions Not Starting
- Ensure `claude-code` is in your PATH
- Check if tmux is installed and accessible
- Verify your Claude Code authentication

### Completion Detection Issues
- Add the stop hook to `~/.claude/settings.json` for better detection
- Increase timeout if operations take longer than expected
- Check `/tmp/claude-code-manager/` for completion marker files

### Configuration Issues
- Use `claude-code-manager config show` to verify current settings
- Reset with `claude-code-manager config init` if configuration is corrupted
- Check file permissions on `~/.claude-code-manager/`

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Related

- [Claude Code Documentation](https://docs.anthropic.com/en/docs/claude-code)
- [tmux Documentation](https://github.com/tmux/tmux/wiki)