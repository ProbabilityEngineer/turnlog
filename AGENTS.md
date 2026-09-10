# Agent Instructions

## Project

- Build `turnlog`, a small Rust CLI provenance recorder for agent sessions and turns.
- Keep provenance local-only by default; do not assume records are committed or pushed.
- Canonical storage is append-only JSONL plus JSON snapshots and Markdown reports under `.turnlog/`.
- Do not mutate session JSONL or Git history from turnlog commands.

## Work tracking

- Use `clu` as the authoritative source of project tasks and work state.
- At the start of substantial work, run `clu ready`, then use `clu claim --context` or claim the specifically requested task; read inherited context before editing.
- Put newly discovered work, notes, and dependencies in `clu`, not Markdown todo lists.
- Close completed work in `clu` after validation; leave incomplete or blocked work represented there.

## Validation

- Run `cargo fmt`, `cargo test`, and `cargo build` for implementation changes.
- Keep `.turnlog/` out of GitHub by default unless explicitly enabled.

## Version control

- Use normal Git workflows. Inspect `git status` and the diff before committing or pushing.
