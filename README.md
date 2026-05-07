# Elma CLI

Local-first autonomous CLI agent designed for high reliability on constrained local models.

## Installation

```bash
cargo install --path .
```

## Shell Completions

Elma can generate shell completions for Bash, Zsh, Fish, and PowerShell.

### Bash

Add this to your `~/.bashrc`:
```bash
eval "$(elma-cli completion bash)"
```

### Zsh

Add this to your `~/.zshrc`:
```zsh
eval "$(elma-cli completion zsh)"
```

### Fish

Add this to your `~/.config/fish/config.fish`:
```fish
elma-cli completion fish | source
```

### PowerShell

Add this to your profile:
```powershell
elma-cli completion powershell | Out-String | Invoke-Expression
```

## Usage

```bash
elma-cli [OPTIONS]
```

Use `elma-cli --help` to see all available options.

## Runtime Config

The auxiliary helper LLM is disabled by default so Elma can run on constrained
VRAM with only the primary local model:

```bash
elma-cli config set runtime.auxiliary.enabled false
```

Enable it only when a separate helper endpoint is available:

```bash
elma-cli config set runtime.auxiliary.enabled true
elma-cli config set runtime.auxiliary.base_url http://127.0.0.1:8084
elma-cli config set runtime.auxiliary.model helper-model-name
```
