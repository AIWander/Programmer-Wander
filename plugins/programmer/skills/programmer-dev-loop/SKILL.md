---
name: programmer-dev-loop
description: The edit-build-test-commit loop with Programmer-Wander tools, including Rust/cargo specifics on Windows, locked-binary swaps, and long builds in persistent sessions. Surface when editing source code, building a project, running tests, committing, or releasing using programmer tools.
---

# Programmer Dev Loop

## The core loop

```
read_file (grep first if large)
  -> edit_block            (surgical, atomic; one logical change per call)
  -> bash "cargo build"    (or the project's build command)
  -> bash "cargo test"
  -> git_diff              (review what actually changed)
  -> git_commit
```

## Rust on Windows - the rules that bite

- **cargo goes through `bash`, never `powershell`.** PowerShell pipes corrupt
  cargo output and mangle ANSI on multi-line commits.
- **Locked binary:** a running .exe cannot be overwritten. Build with an
  alternate target dir and swap by rename:
  ```
  bash "CARGO_TARGET_DIR=/c/tmp/alt-target cargo build --release"
  move_file  old exe -> old.exe.bak
  copy_file  new exe -> destination
  ```
  Never kill the running process to free the lock if it is an MCP server the
  host is using - stage the swap and let the user restart at their boundary.
- **Cross-arch:** on ARM64 hosts build x64 with
  `cargo build --release --target x86_64-pc-windows-msvc`.

## Long builds

Blocking `run`/`bash` calls time out on big builds. Use a persistent session:

```
psession_create
psession_run "cargo build --release 2>&1 | tail -20"
psession_read          (poll until done)
psession_history       (audit what ran)
psession_destroy
```

## Review before commit

`git_diff_summary` for the shape, `git_diff` for content, then `git_commit`
with a message that says why, not just what. Push is a separate decision -
`git_push` publishes; confirm intent before pushing to shared branches.

## Clone-modify-push-back

```
git_clone -> edit_block (x N) -> git_status -> git_diff -> git_commit -> git_push
```

## Scaffolding

`transform_scaffold` for new project skeletons; `transform_sync_dir` to mirror
a template directory into place.
