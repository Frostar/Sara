# tk — folder-aware, LLM-assisted task manager

A Taskwarrior-inspired CLI written in Rust. It auto-detects the Git project you're working in, enriches new tasks with an LLM (priority, due date, tags, dependencies, relevant files), and presents everything in an interactive **ratatui** review form. All data lives in a single SQLite file — nothing is written into your repos.

---

## Installation

### 1 — Prerequisites

**Rust** (if not already installed):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Restart your shell, or:
source "$HOME/.cargo/env"
```

**Ollama** (optional — for local LLM enrichment):

```bash
# macOS
brew install ollama
ollama serve &         # start the server
ollama pull qwen2.5   # pull the default model (best structured-output quality)
```

`tk` works fine without any LLM — just pass `--no-llm` or leave Ollama stopped.

### 2 — Build & install

```bash
git clone <this repo>
cd tk
cargo install --path .
```

This compiles the binary and places it at `~/.cargo/bin/tk`.

Make sure `~/.cargo/bin` is on your `PATH` (the Rust installer usually adds it; if not):

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc   # or ~/.bashrc
source ~/.zshrc
```

Verify:

```bash
tk --version
```

---

## First run

```bash
# Go into a Git repo
cd ~/my-project

# Initialize the project profile (detects stack, stores goal in SQLite)
tk init

# Add your first task — opens an LLM-enriched ratatui review form
tk add "implement user authentication"

# Or add quickly without the form
tk add "fix login bug" pri:H --yes

# List tasks for the current project
tk list

# List ALL tasks across all projects
tk list -a
# or
tk list --all

# List tasks for a specific project from anywhere
tk list --project my-project

# Mark a task done
tk done 1

# Edit a task (opens the review form)
tk modify 2

# Delete a task (prompts for confirmation)
tk delete 3

# Add a dependency (task 4 blocks until task 5 is done)
tk dep 4 on 5

# Remove a dependency
tk dep 4 off 5

# List a task's dependencies
tk dep 4 list
```

---

## LLM setup

### Ollama (default — local, private, no API key)

```toml
# ~/.config/tk/config.toml  (created automatically on first run with defaults)
[llm]
provider = "ollama"
model    = "qwen2.5"        # or llama3.1, mistral-nemo, etc.
# base_url = "http://localhost:11434"  # default
```

### OpenAI

```toml
[llm]
provider = "openai"
model    = "gpt-4o"
api_key  = "sk-..."
```

### Anthropic

```toml
[llm]
provider = "anthropic"
model    = "claude-opus-4-5"
api_key  = "sk-ant-..."
```

### Without LLM

```bash
tk add "my task" --no-llm       # skip enrichment, form opens with empty suggestions
tk add "my task" --yes --no-llm # skip both LLM and the form entirely
```

---

## Config file

Location (created automatically with defaults on first run):

| OS    | Path                                                      |
|-------|-----------------------------------------------------------|
| macOS | `~/Library/Application Support/tk/config.toml`           |
| Linux | `~/.config/tk/config.toml`                               |

Full example:

```toml
default_project = "inbox"   # project name when not inside a Git repo
date_dialect    = "uk"      # "uk" or "us" — affects "next friday" parsing

[llm]
provider     = "ollama"
model        = "qwen2.5"
timeout_secs = 60
# base_url = "http://localhost:11434"
# api_key  = ""

[urgency]                   # Taskwarrior-style coefficients (all optional)
# priority_h = 6.0
# priority_m = 3.9
# priority_l = 1.8
# due        = 12.0
# blocking   = 8.0
# blocked    = -5.0
```

Print the exact config and database paths:

```bash
tk paths
```

---

## Database location

| OS    | Path                                                            |
|-------|-----------------------------------------------------------------|
| macOS | `~/Library/Application Support/tk/tasks.db`                   |
| Linux | `~/.local/share/tk/tasks.db`                                   |

---

## Shell completions (optional)

```bash
# Zsh
tk completions zsh > ~/.zsh/completions/_tk
# add to ~/.zshrc if not already:  fpath=(~/.zsh/completions $fpath) && autoload -U compinit && compinit

# Bash
tk completions bash >> ~/.bashrc

# Fish
tk completions fish > ~/.config/fish/completions/tk.fish
```

---

## Inline Taskwarrior-style tokens

Leading and trailing tokens are parsed as attributes:

```bash
tk add "fix login bug" project:backend +auth pri:H
tk add project:api "redesign rate limiting" +backend
```

Mid-description tokens stay as literal text. Explicit flags are always unambiguous:

```bash
tk add "fix the project:foo reference in docs" --project backend
```

---

## Due dates

Natural-language dates are supported in the `Due` field of the review form and via `--` flags:

| Input         | Meaning              |
|---------------|----------------------|
| `2026-06-20`  | ISO date             |
| `friday`      | This coming Friday   |
| `next friday` | Friday next week     |
| `+3d`         | 3 days from now      |
| `+2w`         | 2 weeks from now     |
| `tomorrow`    | Tomorrow             |

---

## Uninstall

```bash
cargo uninstall tk
```

Remove data and config:

```bash
# macOS
rm -rf ~/Library/Application\ Support/tk/

# Linux
rm -rf ~/.config/tk/ ~/.local/share/tk/
```

---

## Backlog / future features

- `tk next` — LLM-powered "what should I work on next?"
- `tk summary` — project standup (done / in-progress / blockers)
- `tk breakdown <id>` — split a task into subtasks
- Milestones / goals above tasks
- Git linkage (branch from task, auto-close from commit messages)
- `tk capture` — import `TODO`/`FIXME` code comments as tasks
- Time tracking (`tk start` / `tk stop`)
- Cross-machine sync / export
