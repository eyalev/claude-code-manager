# Claude Code Manager

A CLI tool to manage Claude Code sessions through tmux, providing programmatic control over Claude Code in interactive mode.

## Problem

Currently, there's no built-in API or CLI to control Claude Code when running in interactive mode. This makes it difficult to:

- Automate tasks with Claude Code
- Monitor Claude Code execution programmatically  
- Manage multiple concurrent Claude Code sessions
- Integrate Claude Code into scripts and workflows

## Solution

Claude Code Manager uses tmux to spawn, monitor, and interact with Claude Code sessions. It provides:

- **Session Management**: Start, list, attach to, and kill Claude Code sessions
- **Task Execution**: Send messages/tasks to Claude Code and monitor completion
- **Process Monitoring**: Track session status and capture output
- **Multi-session Support**: Manage multiple concurrent Claude Code instances

## Installation

### From Source

```bash
git clone https://github.com/eyalev/claude-code-manager.git
cd claude-code-manager
cargo build --release
cp target/release/claude-code-manager ~/.local/bin/
```

### From GitHub Releases

Download the latest release for your platform:

```bash
# Linux x86_64
curl -L https://github.com/eyalev/claude-code-manager/releases/latest/download/claude-code-manager-linux-x86_64 -o ~/.local/bin/claude-code-manager
chmod +x ~/.local/bin/claude-code-manager

# macOS (Intel)
curl -L https://github.com/eyalev/claude-code-manager/releases/latest/download/claude-code-manager-macos-x86_64 -o ~/.local/bin/claude-code-manager
chmod +x ~/.local/bin/claude-code-manager

# macOS (Apple Silicon)
curl -L https://github.com/eyalev/claude-code-manager/releases/latest/download/claude-code-manager-macos-aarch64 -o ~/.local/bin/claude-code-manager
chmod +x ~/.local/bin/claude-code-manager
```

## Prerequisites

- **tmux**: Required for session management
- **claude-code**: Must be installed and accessible in PATH
- **Rust** (for building from source): Version 1.70 or later

## Usage

### Start a New Session

```bash
# Start a session with a task and wait for completion
claude-code-manager start --message "Create a hello world Python script" --wait

# Start a session in background
claude-code-manager start --message "Analyze the codebase and create a report" --session-name "analysis-task"

# Start with custom working directory
claude-code-manager start --message "Fix the failing tests" --working-dir /path/to/project --wait
```

### List Sessions

```bash
claude-code-manager list
```

Output:
```
Active Claude Code sessions:
  abc123def - analysis-task (active)
  xyz789ghi - claude-1f2a3b4c (completed)
```

### Send Messages to Existing Sessions

```bash
# Send a message and continue in background
claude-code-manager send abc123def "Now create unit tests for the script"

# Send a message and wait for completion
claude-code-manager send abc123def "Run the tests and fix any issues" --wait
```

### Check Session Status

```bash
# Get the last 50 lines of output
claude-code-manager status abc123def

# Get more output lines
claude-code-manager status abc123def --lines 100
```

### Attach to a Session

```bash
# Attach to session for interactive use
claude-code-manager attach analysis-task
```

### Kill Sessions

```bash
# Kill a specific session
claude-code-manager kill abc123def

# Kill all sessions
claude-code-manager kill-all
```

## Advanced Usage

### Automation Example

```bash
#!/bin/bash
# Automated code review script

SESSION_ID=$(claude-code-manager start \
  --message "Review all Python files in src/ directory for code quality issues" \
  --working-dir $(pwd) \
  --timeout 600)

echo "Started code review session: $SESSION_ID"

# Wait for initial analysis
claude-code-manager send $SESSION_ID \
  "Create a detailed report with findings and recommendations" --wait

# Get the results
echo "Code review completed:"
claude-code-manager status $SESSION_ID --lines 200

# Clean up
claude-code-manager kill $SESSION_ID
```

### Integration with Monitoring

```bash
# Use with cron or monitoring systems
claude-code-manager start \
  --message "Check system health and generate report" \
  --session-name "health-check-$(date +%Y%m%d-%H%M)" \
  --wait --timeout 300
```

## Configuration

Claude Code Manager uses the following defaults:

- **Timeout**: 300 seconds (5 minutes) for `--wait` operations
- **Output Lines**: 50 lines for status command
- **Session Naming**: Auto-generated names like `claude-abc123def`

## Architecture

The tool consists of several modules:

- **`tmux.rs`**: Low-level tmux session management
- **`claude.rs`**: Claude Code process spawning and communication
- **`session.rs`**: High-level session management and state tracking
- **`main.rs`**: CLI interface and command handling

## Limitations

- Requires tmux to be installed and functional
- Claude Code completion detection is heuristic-based
- Session state is maintained in memory (not persistent across restarts)
- Linux/macOS only (Windows support via WSL)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Changelog

### v0.1.0
- Initial release
- Basic session management (start, list, kill)
- Message sending and completion waiting
- Session attachment support
- Status monitoring