# Repository Guidelines

## Parent Context & Rough Docs
- Read the parent repository guidelines in `../AGENTS.md` and rough architectural/design docs in `../note/` for broader ecosystem context.

## Skills & Navigation
- **Codebase Manifest (`skill://codebase-manifest`)**: Read [`../MANIFEST.md`](../MANIFEST.md) FIRST before exploring or editing code (`../.agents/skills/codebase-manifest/SKILL.md`).
  - Locate exact crate, file, and symbol (`fn`, `struct`, `enum`, line range) in `../MANIFEST.md` before calling `read_file`.
  - Only update `../MANIFEST.md` when unit tests pass (`cargo test --manifest-path publication/Cargo.toml --lib`).
- **Codebase Navigation (`skill://axon-nav`)**: Use the `axon-nav` skill (`.agents/skills/axon-nav/SKILL.md`) for low-token codebase exploration and impact analysis.
  - Never read full files (>120 lines) to locate symbols or trace callers.
  - Start every exploration with `axon_query` or `axon_context`.
  - Run `axon_impact` before modifying any shared function, struct, or API.
  - Use `view_file` only with exact start/end line ranges returned by Axon.
- **Thinking & Talking Mode (`/skill:caveman full`)**: Always think and write in caveman `full` mode (`skill://caveman`, `.agents/skills/caveman/SKILL.md`).
  - Drop articles, filler, pleasantries, and narrative drift. Keep complete technical substance, exact symbols, code blocks, and error messages verbatim.
