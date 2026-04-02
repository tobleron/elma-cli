<!-- @efficiency-role: infra-config -->

# Config Profiles

Each profile is a separate TOML file in this folder.

The CLI loads a profile by name via `--profile <name>` which maps to `config/<name>.toml`.

For runtime-managed Elma intel units, `system_prompt` is canonically enforced by Rust code during startup sync.
Treat these TOML prompts as seed copies, not the final source of truth.
