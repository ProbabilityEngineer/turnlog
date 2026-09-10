# Agent Instructions

## Project

- Build `turnlog`, a small Rust CLI provenance recorder for agent sessions and turns.
- Keep provenance local-only by default; do not assume records are committed or pushed.
- Canonical storage is append-only JSONL plus JSON snapshots and Markdown reports under `.turnlog/`.
- Do not mutate session JSONL or Git history from turnlog commands.

## Tasks

- Use `clu` as the authoritative task tracker. Run `clu ready` and use `clu claim --context` before substantial work; close tasks only after validation.

## Validation

- Run `cargo fmt`, `cargo test`, and `cargo build` for implementation changes.
- Keep `.turnlog/` out of GitHub by default unless explicitly enabled.

## Version control

- Use normal Git workflows. Inspect `git status` and the diff before committing or pushing.
