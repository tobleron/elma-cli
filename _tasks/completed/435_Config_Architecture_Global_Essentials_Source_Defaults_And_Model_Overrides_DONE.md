# Task 435: Config Architecture - Global Essentials, Source Defaults, And Model Overrides

**Status:** pending
**Priority:** CRITICAL
**Estimated effort:** 4-7 days
**Depends on:** pending Task 431
**References:** `src/dirs.rs`, `src/paths.rs`, `src/app_bootstrap_core.rs`, `src/app_bootstrap_profiles.rs`, `src/storage.rs`, `src/defaults_*.rs`, `src/types_core.rs`

## Problem

Elma config currently has an unclear ownership model:

- `src/dirs.rs` correctly defines an OS-native config directory through `directories::ProjectDirs`.
- `config_root_path("config")` resolves the default config root to that OS-native directory.
- `elma_config_path()` still points to repo-local `./elma.toml`.
- Bootstrap writes `./elma.toml` for CLI/env base URL and also writes `global.toml` under the config root.
- Model-specific folders can contain many full profile files, even when most values are just defaults.
- Some profile fallbacks reference `config/defaults`, while the intended default behavior should live in source code.

The result is confusing: users do not know which file is authoritative, and Elma can scatter essential runtime config, default intel-unit settings, and model-specific overrides across too many places.

## Architecture Decision

Use the OS-native config directory as the canonical home for user-level Elma configuration.

Example locations:

- macOS: `~/Library/Application Support/rs.elma.elma-cli/` or the exact path returned by `directories::ProjectDirs`
- Linux: `$XDG_CONFIG_HOME/elma/elma-cli/` or `~/.config/elma/elma-cli/`
- Windows: the platform config directory returned by `ProjectDirs`

The canonical global config file should be:

```text
{os_config_dir}/elma.toml
```

Repo-local `./elma.toml` should not be the global config. If supported, it should be an explicit project-local override with lower persistence priority and clearly documented precedence.

## Desired Config Layers

### Layer 1: Built-In Source Defaults

Elma ships best defaults in Rust source code.

These include:

- intel-unit system prompts
- temperatures
- top-p
- repeat penalty
- reasoning format defaults
- max tokens
- timeouts
- profile names and versions

Defaults should be unified as much as possible:

- Prefer a central source-default registry over many disconnected default files.
- Avoid writing all defaults to disk during normal startup.
- Do not require user config files for Elma to work.
- Do not make `config/defaults/*.toml` the canonical default source.

### Layer 2: Global Essential Config

The global config file contains only common runtime essentials needed to start and connect.

Suggested schema:

```toml
version = 1

[provider]
base_url = "http://localhost:8080"
model = ""
api_key_env = ""

[runtime]
sessions_root = ""
http_timeout_s = 120
request_timeout_s = 120
safe_mode = "ask"

[ui]
show_thinking = true
show_process = true
```

Rules:

- Keep this file small and understandable.
- Do not store every intel-unit profile here.
- Do not store tuned per-model prompts here.
- Do not store secrets directly unless explicitly supported later with warnings.
- Prefer environment-variable references for secrets.

### Layer 3: Project-Local Override

Optional project-local config may exist at:

```text
./elma.toml
```

It should only override project-specific essentials, such as:

- base URL for this repo
- model preference for this repo
- sessions root override

Project-local config must not become the hidden global source of truth.

### Layer 4: Per-Model Overrides

Per-model overrides live under the OS-native config root.

Suggested layout:

```text
{os_config_dir}/models/{sanitized_model_id}/
  model.toml
  profiles/
    orchestrator.toml
    turn_summary.toml
    final_answer_extractor.toml
  tune/
  formula_memory/
```

Rules:

- Per-model files are sparse overrides, not full copies of every default.
- Missing fields inherit from built-in source defaults.
- Missing profile files inherit the whole built-in profile.
- Tuning may write override files for a specific model.
- Users may manually edit model overrides when a model behaves differently.
- Elma must be able to show the effective merged profile for debugging.

### Layer 5: CLI And Environment Overrides

Precedence should be explicit:

1. CLI flags, such as `--base-url` and `--model`
2. Environment variables, such as `LLAMA_BASE_URL` and `LLAMA_MODEL`
3. Project-local `./elma.toml`
4. Global `{os_config_dir}/elma.toml`
5. Per-model overrides for profile behavior after model selection
6. Built-in source defaults

If the exact precedence differs during implementation, document it in the task file and tests before changing behavior.

## Objective

Implement a clear config architecture where:

- OS-native `elma.toml` is the canonical user-level global config.
- Global config contains only essential runtime settings.
- Built-in defaults remain in source code.
- Per-model config contains sparse overrides only.
- Effective profiles are merged from source defaults plus per-model overrides.
- Users can discover, view, and edit config paths without reading source code.

## Non-Goals

- Do not move all defaults into global config.
- Do not dump every profile file into every model folder on startup.
- Do not make JSON a user-facing config format.
- Do not edit `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT`.
- Do not remove tuning, formula memory, or model behavior profiles; re-home them under the clear per-model layout.
- Do not silently migrate or delete existing config files without backup.

## Implementation Plan

### Phase 1: Canonical Path Semantics

1. Change `elma_config_path()` to use `ElmaPaths::elma_toml()` for global config.
2. Add an explicit `project_elma_config_path()` for repo-local `./elma.toml`.
3. Stop treating repo-local `./elma.toml` as the global config.
4. Stop writing both `./elma.toml` and `global.toml` on startup.
5. Keep `global.toml` as a legacy read fallback for one migration cycle.

### Phase 2: Global Config Schema

1. Replace or evolve `ElmaProjectConfig` and `GlobalConfig` into one canonical `ElmaGlobalConfig`.
2. Keep the schema small and essential.
3. Validate base URL, model string, timeout ranges, safe mode values, and sessions root.
4. Add path-aware errors using existing error style.
5. Write a default global config only when needed, and only with essential fields.

### Phase 3: Source Default Registry

1. Create a central source-default registry for all intel-unit profiles.
2. Consolidate scattered `default_*_config()` functions behind one lookup API:
   - `default_profile(profile_id, base_url, model) -> Profile`
   - `all_default_profile_ids() -> Vec<ProfileId>`
3. Keep system prompts in source-controlled Rust strings or source-owned include files.
4. Ensure normal startup does not require `config/defaults`.
5. Add tests that every loaded profile has a source default.

### Phase 4: Sparse Per-Model Overrides

1. Define a partial profile override type where every field is optional except version/profile id.
2. Load effective profile as:
   - source default
   - plus per-model override
   - plus runtime base URL/model injection
3. Save only changed fields when writing model overrides.
4. Move model-specific data under `{os_config_dir}/models/{model}/`.
5. Keep compatibility reads from the current `{config_root}/{model}/profile.toml` layout.

### Phase 5: User-Facing Config Commands

Add commands or subcommands such as:

```bash
elma-cli config path
elma-cli config show
elma-cli config set provider.base_url http://localhost:8080
elma-cli config doctor
elma-cli config effective-profile orchestrator
```

Requirements:

- `config path` prints the OS-native global config path.
- `config show` prints only essential global config.
- `effective-profile` shows the merged source-default plus override result.
- `doctor` reports legacy files, invalid values, and precedence sources.

### Phase 6: Migration

Add a conservative migration path:

- Read old `global.toml` if canonical `elma.toml` is absent.
- Read old repo-local `./elma.toml` as project override only.
- Read old per-model full profile files as overrides.
- Never delete or rewrite old config without backup.
- Emit a concise notice that names the canonical config path.

## Files To Audit

| File | Reason |
|------|--------|
| `src/dirs.rs` | OS-native config/data/cache paths |
| `src/paths.rs` | Current global/project config confusion |
| `src/app_bootstrap_core.rs` | Startup config resolution and persistence |
| `src/app_bootstrap_profiles.rs` | Profile loading and fallback behavior |
| `src/storage.rs` | Config load/save helpers |
| `src/types_core.rs` | Config structs and CLI args |
| `src/llm_config.rs` | Runtime config currently stored as `runtime.toml` |
| `src/config_healthcheck.rs` | Validation and doctor logic |
| `src/defaults_core.rs` | Source defaults |
| `src/defaults_router.rs` | Source defaults |
| `src/defaults_evidence.rs` | Source defaults |
| `src/defaults_evidence_core.rs` | Source defaults |
| `src/app_bootstrap_modes.rs` | Restore/tune modes and profile paths |
| `src/tune.rs` | Tune artifacts and active manifests |
| `src/profile_sets.rs` | Formula memory and profile snapshots |

## Success Criteria

- [ ] `elma-cli config path` shows the OS-native global config file.
- [ ] Startup reads global essentials from OS-native `elma.toml`.
- [ ] Repo-local `./elma.toml` is treated only as an optional project override.
- [ ] `global.toml` is legacy fallback only.
- [ ] Built-in source defaults are enough to run Elma with no profile files on disk.
- [ ] Per-model folders contain sparse overrides, not full default dumps.
- [ ] Effective profile merging is deterministic and test-covered.
- [ ] Users can inspect the effective profile for any intel unit.
- [ ] Config validation reports precise file paths and field names.
- [ ] No secrets are written directly by default.

## Verification

```bash
cargo build
cargo test paths
cargo test config
cargo test app_bootstrap_profiles
cargo test storage
cargo test config_healthcheck
```

Manual smoke:

1. Delete or move local repo `./elma.toml`.
2. Run `elma-cli config path` and verify it points to the OS-native config dir.
3. Set `provider.base_url` through the config command.
4. Start Elma without `--base-url` and verify it uses the OS-native config.
5. Add a project-local `./elma.toml` and verify it overrides only for that repo.
6. Add a sparse per-model override for one profile and verify the effective profile merges correctly.

## Anti-Patterns To Avoid

- Do not scatter defaults across generated config files.
- Do not make users edit dozens of files for normal setup.
- Do not overwrite tuned model-specific settings during startup.
- Do not keep writing repo-local `./elma.toml` as if it were global.
- Do not let legacy `global.toml` compete indefinitely with canonical `elma.toml`.
