# Sara M1 smoke test checklist

Run after `cargo install --path .` from the repo root.

## Install & identity

- [ ] `sara --version` prints a version
- [ ] `sara --help` shows Sara branding (not tk)

## Migration (if you had tk installed)

- [ ] First `sara` run prints a one-time notice if tk data was imported
- [ ] `sara list` shows your existing tk tasks
- [ ] Old tk data at `~/Library/Application Support/tk/` is untouched

## Core task commands

- [ ] `sara add "smoke test task" --yes` creates a task
- [ ] `sara list` shows the new task
- [ ] `sara info 1` opens the detail view (or prints details)
- [ ] `sara start 1` / `sara stop 1` time tracking works
- [ ] `sara done 1` completes the task
- [ ] `sara undo` reverts the last command
- [ ] `sara dep`, `sara annotate`, `sara link` still work on a task

## Paths

- [ ] `sara paths` shows config under `.../sara/` (not `.../tk/`)

## Cleanup

- [ ] Optional: `cargo uninstall tk` if the migration notice appeared
