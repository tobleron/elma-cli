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
