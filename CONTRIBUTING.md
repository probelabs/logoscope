Contributing to Logoscope
=========================

Thanks for your interest in improving Logoscope! This guide explains how to propose changes and get them merged quickly and safely.

Guiding Principles
------------------

- Focus on DX/UX: clear output, sensible defaults, progressive discovery.
- Small, safe, test‑covered PRs: one logical change at a time.
- Consistency over cleverness: match existing style and patterns.
- Security & privacy first: masking and PII safety must not regress.

Getting Started
---------------

1. Fork the repo and clone your fork.
2. Rust toolchain: `rustup default stable` (or the toolchain in CI).
3. Install dev tooling (optional): `cargo fmt`, `cargo clippy`.
4. Build & test: `cargo build`, `cargo test` (manifest at `logoscope/Cargo.toml`).

Development Workflow
--------------------

- Add tests first (TDD) when feasible.
- Keep changes scoped; avoid unrelated refactors.
- Update docs (README/spec) when behavior changes.
- Ensure streaming/batch both remain functional.

Code Style
----------

- Run `cargo fmt --all` and `cargo clippy -- -D warnings` before pushing.
- Prefer explicit names; avoid one‑letter identifiers.
- Keep modules cohesive; avoid cross‑module tight coupling.

Commit Messages
---------------

- Use clear, imperative summaries: "Add streaming deltas output".
- Reference issues where applicable: "Fixes #123".
- One change per commit where possible; rebase before merging.

Testing
-------

- `cargo test --manifest-path logoscope/Cargo.toml` (unit + integration).
- Add tests for new maskers, parsers, anomaly logic, CLI flags.
- Provide golden samples when touching multi‑line or error recovery.

Documentation
-------------

- README: keep Quick Start + CLI reference aligned with new flags.
- Spec: append a brief progress log entry when major features land.

Security
--------

- Do not include secrets in code or CI.
- Keep masking robust; avoid regressions that expose PII.
- If you find a vulnerability, please open a Security Advisory or email the maintainers (see repo contacts) instead of filing a public issue.

Pull Requests
-------------

1. Ensure all CI checks pass (build, tests, clippy, fmt).
2. Keep PRs small and focused; describe the problem and the approach.
3. Link issues; add screenshots or sample outputs where helpful.

Thank you for helping make Logoscope better!

