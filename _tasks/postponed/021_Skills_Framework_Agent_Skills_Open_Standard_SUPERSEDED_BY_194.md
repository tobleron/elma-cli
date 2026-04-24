# Task 156: Skills Framework (Agent Skills Open Standard)

## Status
Superseded by Task 194. Historical reference only.

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Summary

Implement the Agent Skills open standard from Crush for extensible agent capabilities. This provides a structured way to define and load agent skills from SKILL.md files.

## Motivation

Elma currently loads skills from `.kilo/` but doesn't have a standardized skill format. Adopting the open standard would enable:
- Interoperability with other tools using the same standard
- Structured skill discovery and validation
- XML generation for system prompt injection

## Source

Crush's skills package at `_stress_testing/_crush/internal/skills/skills.go`

## Implementation

### Types And Interfaces

```go
// Skill represents a parsed SKILL.md file
type Skill struct {
    Name          string            `yaml:"name" json:"name"`
    Description  string            `yaml:"description" json:"description"`
    License     string           `yaml:"license,omitempty" json:"license,omitempty"`
    Compatibility string         `yaml:"compatibility,omitempty" json:"compatibility,omitempty"`
    Metadata    map[string]string `yaml:"metadata,omitempty" json:"metadata,omitempty"`
    Instructions string          `yaml:"-" json:"instructions"`
    Path        string           `yaml:"-" json:"path"`
    SkillFilePath string         `yaml:"-" json:"skill_file_path"`
    Builtin     bool             `yaml:"-" json:"builtin"`
}

// DiscoveryState represents the outcome of discovering a single skill file
type DiscoveryState int

const (
    StateNormal DiscoveryState = iota  // Parsed and validated successfully
    StateError                        // Scan/parse/validate error
)

// SkillState represents the latest discovery status
type SkillState struct {
    Name string
    Path string
    State DiscoveryState
    Err  error
}
```

### Functions

- `Parse(path string) (*Skill, error)` - Parse SKILL.md from disk
- `ParseContent(content []byte) (*Skill, error)` - Parse from raw bytes
- `Discover(paths []string) []*Skill` - Find all valid skills in paths
- `DiscoverWithStates(paths []string) ([]*Skill, []*SkillState)` - With parse status
- `Validate() error` - Validate skill meets spec requirements
- `ToPromptXML(skills []*Skill) string` - Generate XML for system prompt
- `Filter(skills []*Skill, disabled []string) []*Skill` - Filter disabled skills
- `Deduplicate(skills []*Skill) []*Skill` - Remove duplicates

### SKILL.md Format

Markdown files with YAML frontmatter:

```markdown
---
name: example-skill
description: A brief description of what this skill does
license: MIT
compatibility: elma v1.0+
---

# Skill instructions

Your skill implementation here...
```

### Constants

```go
const (
    SkillFileName          = "SKILL.md"
    MaxNameLength          = 64
    MaxDescriptionLength = 1024
    MaxCompatibilityLength = 500
)
```

### Validation Rules

- Name: required, alphanumeric with hyphens, max 64 chars
- Description: required, max 1024 chars
- Compatibility: max 500 chars
- Directory name should match skill name

## Verification

- All tasks in pending/ should build: `cargo build --manifest-path .kilo/agents/Manifest.toml`
- Skills can be discovered from test SkillFilePath
- Generated XML is valid for prompt injection

## Dependencies

- yaml (already in elma-cli: toml, serde_yaml)
- fastwalk or similar for concurrent directory walking
- PubSub for skill discovery events

## Notes

- This is different from the existing `.kilo/` command/agent format
- Consider whether to keep both or migrate to unified format
- Skills vs commands vs agents - clear separation of concerns