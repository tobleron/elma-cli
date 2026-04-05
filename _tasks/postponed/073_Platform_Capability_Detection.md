# Task 073: Platform Capability Detection

## Priority
**P3 - ADVANCED/LATER (Tier C — Postponed)**
**Postponed until:** Tier A + Tier B phases complete

## Objective
Implement platform capability detection that runs on startup and provides context to the orchestrator.

## Implementation Steps

1. **Create platform detection module** `src/platform.rs`:
   ```rust
   pub struct PlatformCapabilities {
       pub os: OperatingSystem,
       pub arch: String,
       pub shell: String,
       pub sudo_available: bool,
       pub resource_limits: ResourceLimits,
       pub network: NetworkCapabilities,
       pub platform_specific: Vec<PlatformFeature>,
   }

   pub struct ResourceLimits {
       pub available_memory_gb: Option<u64>,
       pub available_disk_gb: Option<u64>,
       pub max_processes: Option<u64>,
   }

   pub async fn detect_platform_capabilities() -> PlatformCapabilities;
   ```

2. **Detect operating system**:
   ```rust
   pub enum OperatingSystem {
       MacOS { version: String, arch: String },
       Linux { distro: String, version: String },
       Windows { version: String, arch: String },
       Unknown,
   }
   ```

3. **Detect shell environment**:
   - Current shell ($SHELL)
   - Available shells (bash, zsh, fish, etc.)
   - Shell features (associative arrays, process substitution, etc.)

4. **Detect permission levels**:
   - Sudo availability (`sudo -n true`)
   - Write permissions in common directories
   - Docker/podman availability

5. **Detect resource constraints**:
   - Available memory (from sysinfo or /proc/meminfo)
   - Available disk space
   - CPU core count

6. **Detect network capabilities**:
   - Internet connectivity
   - Proxy configuration
   - Available network tools (curl, wget, nc)

7. **Integrate with workspace context**:
   ```rust
   // In workspace.rs or app_bootstrap_core.rs
   let platform = detect_platform_capabilities().await;
   let ws_brief = format!(
       "{}\nPlatform: {} {} | Shell: {} | Sudo: {}",
       ws_brief,
       platform.os,
       platform.arch,
       platform.shell,
       if platform.sudo_available { "yes" } else { "no" }
   );
   ```

8. **Add platform-aware command suggestions**:
   ```rust
   // In tool discovery or orchestration
   pub fn suggest_platform_command(base_cmd: &str, platform: &PlatformCapabilities) -> String {
       match (base_cmd, &platform.os) {
           ("brew", OperatingSystem::MacOS {..}) => "brew",
           ("brew", OperatingSystem::Linux {..}) => "linuxbrew",
           ("Get-Content", OperatingSystem::Windows {..}) => "Get-Content",
           ("Get-Content", _) => "cat",
           // ... more mappings
       }
   }
   ```

9. **Detect command dialect / flag capabilities**:
   - Record not only whether `rg`, `sed`, `find`, etc. exist, but which important flags/forms are supported.
   - Provide helpers for safe portable variants:
     - `rg --color never` vs unsupported `--no-color`
     - `sed -i ''` on macOS vs `sed -i` on Linux
     - shell / grep / find syntax differences that affect repair reliability.

## Acceptance Criteria
- [ ] Platform detection runs on Elma startup
- [ ] OS, shell, and sudo availability are detected
- [ ] Resource limits are estimated (memory, disk)
- [ ] Network connectivity is checked
- [ ] Platform info is included in workspace context
- [ ] Orchestrator receives platform-aware prompts
- [ ] Command suggestions adapt to platform
- [ ] Command repair and shell generation can query supported flag/dialect capabilities instead of assuming cross-platform equivalence

## Additional Session Evidence
- Session `s_1775235404_589084000` showed a concrete dialect gap:
  - `rg` existed and was discovered correctly
  - but both the orchestrator and command-repair path kept using unsupported `--no-color`
  - this is not just “tool availability”; it is missing command-dialect capability detection

## Files to Create
- `src/platform.rs` - Platform detection module

## Files to Modify
- `src/workspace.rs` - Include platform in workspace brief
- `src/app_bootstrap_core.rs` - Run detection on startup
- `src/orchestration_planning.rs` - Use platform in prompts
- `src/main.rs` - Add platform module

## Priority
MEDIUM - Enhances reliability and platform awareness

## Dependencies
- Task 015 (Autonomous Tool Discovery) - complementary

## Philosophy Alignment
- **"Dynamically leverages available knowledge, tools, environment context, and platform capabilities"**
- **"Maximize intelligence per token"** - platform-aware commands are more efficient
