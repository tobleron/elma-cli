# Master Plan

Last updated: 2026-05-03 (Prioritized Tasks 506-536, deferred others)

This is the execution index for current pending tasks. Use it to choose work in dependency order, not as a replacement for each task file. Each task file remains the implementation detail, verification commands, and done criteria.

## Phase: Iterative Tool Perfection (Priority)

These tasks are the current top priority, ordered from simple read-only tools to complex destructive/modifying tools.

### 506 exists
- File: `_tasks/pending/506_exists_Iterative_Tool_Perfection.md`

### 507 ls
- File: `_tasks/pending/507_ls_Iterative_Tool_Perfection.md`

### 508 stat
- File: `_tasks/pending/508_stat_Iterative_Tool_Perfection.md`

### 509 file_size
- File: `_tasks/pending/509_file_size_Iterative_Tool_Perfection.md`

### 510 read
- File: `_tasks/pending/510_read_Iterative_Tool_Perfection.md`

### 511 glob
- File: `_tasks/pending/511_glob_Iterative_Tool_Perfection.md`

### 512 search
- File: `_tasks/pending/512_search_Iterative_Tool_Perfection.md`

### 513 workspace_info
- File: `_tasks/pending/513_workspace_info_Iterative_Tool_Perfection.md`

### 514 repo_map
- File: `_tasks/pending/514_repo_map_Iterative_Tool_Perfection.md`

### 515 tool_search
- File: `_tasks/pending/515_tool_search_Iterative_Tool_Perfection.md`

### 516 summary
- File: `_tasks/pending/516_summary_Iterative_Tool_Perfection.md`

### 517 git_inspect
- File: `_tasks/pending/517_git_inspect_Iterative_Tool_Perfection.md`

### 518 fetch
- File: `_tasks/pending/518_fetch_Iterative_Tool_Perfection.md`

### 519 observe
- File: `_tasks/pending/519_observe_Iterative_Tool_Perfection.md`

### 520 job_status
- File: `_tasks/pending/520_job_status_Iterative_Tool_Perfection.md`

### 521 job_output
- File: `_tasks/pending/521_job_output_Iterative_Tool_Perfection.md`

### 522 touch
- File: `_tasks/pending/522_touch_Iterative_Tool_Perfection.md`

### 523 mkdir
- File: `_tasks/pending/523_mkdir_Iterative_Tool_Perfection.md`

### 524 write
- File: `_tasks/pending/524_write_Iterative_Tool_Perfection.md`

### 525 edit
- File: `_tasks/pending/525_edit_Iterative_Tool_Perfection.md`

### 526 patch
- File: `_tasks/pending/526_patch_Iterative_Tool_Perfection.md`

### 527 copy
- File: `_tasks/pending/527_copy_Iterative_Tool_Perfection.md`

### 528 move
- File: `_tasks/pending/528_move_Iterative_Tool_Perfection.md`

### 529 trash
- File: `_tasks/pending/529_trash_Iterative_Tool_Perfection.md`

### 530 update_todo_list
- File: `_tasks/pending/530_update_todo_list_Iterative_Tool_Perfection.md`

### 531 job_start
- File: `_tasks/pending/531_job_start_Iterative_Tool_Perfection.md`

### 532 job_stop
- File: `_tasks/pending/532_job_stop_Iterative_Tool_Perfection.md`

### 533 run_python
- File: `_tasks/pending/533_run_python_Iterative_Tool_Perfection.md`

### 534 run_node
- File: `_tasks/pending/534_run_node_Iterative_Tool_Perfection.md`

### 535 shell
- File: `_tasks/pending/535_shell_Iterative_Tool_Perfection.md`

### 536 respond
- File: `_tasks/pending/536_respond_Iterative_Tool_Perfection.md`

## Deferred Tasks

All tasks previously in Phase 1-5 that were not 506-536 have been moved to `_tasks/deferred/`.

### Previously Phase 1: Stabilization And Architecture Decisions (DEFERRED)
- Tasks 437, 438, 439, 440, 441, 442, 443, 451, 494, 452, 453

### Previously Phase 2: Core Local Execution And File Tools (DEFERRED)
- Tasks 454, 455, 456, 457, 458, 459, 460, 461, 462, 463, 464

### Previously Phase 3: Memory, Documents, Sessions, And Events (DEFERRED)
- Tasks 465, 466, 467, 468, 469, 470, 471, 472

### Previously Phase 4: Diagnostics, Release Gates, And Cleanup Safety (DEFERRED)
- Tasks 473, 474, 475, 476, 477, 478, 479, 480, 481, 482, 483, 484

### Previously Phase 5: Optional Network, Extension, And Workflow Expansion (DEFERRED)
- Tasks 485, 486, 487, 488, 489, 490, 491, 492, 493

### Additional Deferred Tasks (DEFERRED)
- Tasks 499, 500, 501, 502, 503, 504, 505

## Operating Rules

- Move a task from `_tasks/pending/` to `_tasks/active/` before implementation.
- Do not mark a task complete until its own verification section passes.
- Do not modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT` unless a task explicitly records user approval for that change.
- Prefer rust-native/offline tools over shell and network tools.
- Keep intel-unit JSON simple: one nested object level maximum, three required fields by default, five total fields absolute maximum.
- Surface routing, tool discovery, retries, compaction, stop reasons, and decomposition as transcript rows.
- Failed approaches do not continue down the objective hierarchy; retry with a new approach toward the same original objective.

## Current First Picks

Tasks 506-521 (Read-only tools) are the current first picks.
