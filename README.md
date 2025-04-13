# GitHub Issues Auto-Assignment Bot

A Rust application that monitors GitHub repositories and automatically requests assignment to open issues based on configurable filters. The bot is designed to appear human-like, respecting rate limits, and only handling one issue at a time to avoid spamming repositories.

## Features

- Monitor multiple GitHub repositories for new issues
- Filter issues by labels (e.g., "good first issue", "help wanted")
- Optional filtering by title patterns using regex
- Natural, randomized comment templates to appear human-like
- Rate limiting and jitter to avoid triggering bot detection
- Processes only one issue at a time with configurable timeout
- Persists state between runs

## Prerequisites

- Rust 1.56+ (2021 edition)
- GitHub Personal Access Token with `repo` scope

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/gh-issues-bot.git
cd gh-issues-bot

# Build the release version
cargo build --release

# The binary will be available at target/release/gh-issues-bot
```

## Configuration

The bot can be configured using a TOML file. See [config.example.toml](config.example.toml) for a sample configuration.

### Configuration Options

- `auth_token`: Your GitHub Personal Access Token
- `user_login`: Your GitHub username
- `poll_interval_secs`: How often to check for new issues (in seconds)
- `max_retries`: Number of attempts to make for API calls
- `cooldown_hours`: How long to wait for an issue assignment before trying another
- `comment_templates`: Array of message templates to use when requesting assignment
- `repositories`: Array of repository configurations
  - `owner`: Repository owner (username or organization)
  - `repo`: Repository name
  - `labels`: Array of labels to filter issues by
  - `title_regex` (optional): Regex pattern to filter issue titles
  - `exclude_labels` (optional): Array of labels to exclude

### Creating a Configuration File

1. Copy the sample configuration file:
   ```bash
   cp config.example.toml config.toml
   ```

2. Edit the configuration file with your GitHub token and repositories you want to monitor:
   ```bash
   # Replace with your actual GitHub token and username
   auth_token = "ghp_YOUR_TOKEN_HERE"
   user_login = "yourusername"
   
   # Add repositories you want to monitor
   [[repositories]]
   owner = "rust-lang"
   repo = "rust"
   labels = ["good first issue", "E-easy"]
   ```

### Environment Variables

Instead of using a config file, you can use environment variables:

```bash
export GITHUB_TOKEN=your_github_token
export GITHUB_USERNAME=your_github_username
```

You can also create a `.env` file in the project root:
```
GITHUB_TOKEN=your_github_token
GITHUB_USERNAME=your_github_username
```

## Usage

```bash
# Run with a config file
./gh-issues-bot run --config config.toml

# Run with environment variables (no config file)
export GITHUB_TOKEN=your_github_token
export GITHUB_USERNAME=your_github_username
./gh-issues-bot run

# Specify a custom data directory (default is .gh-issues-bot)
./gh-issues-bot run --config config.toml --data-dir /path/to/data
```

## How It Works

1. The bot periodically checks configured repositories for new open issues
2. It filters issues based on your configured labels and other criteria
3. When it finds an eligible issue:
   - It posts a comment requesting assignment using one of your templates
   - It waits for the configured cooldown period before checking for another issue
4. The bot tracks which issues it has already processed to avoid duplicates

## State Management

The bot maintains state between runs in the `.gh-issues-bot` directory (or in your custom data directory). This includes:

- `active_issue.json`: Information about the current issue awaiting assignment
- `processed_issues.json`: List of issues the bot has already processed

## Limitations

- The bot only processes one issue at a time to avoid spamming
- It respects GitHub API rate limits
- It does not automatically check if it has been assigned after commenting

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 