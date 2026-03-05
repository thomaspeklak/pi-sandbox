# Changelog

All notable changes to this project will be documented in this file.

## [v0.1.0] — 2026-03-05

### Features

- Rust rewrite of the sandbox launcher CLI (`ags`) with rootless Podman execution.
- Multi-agent runtime support: `pi`, `claude`, `codex`, `gemini`, `opencode`, and `shell`.
- Config-driven mounts, tool wiring, secret resolution, SSH bootstrap, and browser sidecar support.
- New release automation via GitHub Actions on `v*` tags.

### Bug Fixes

- Added external git metadata mount handling for linked worktrees/submodules.
- Improved install/update flows and sandbox bootstrap behavior.

### Chores / Other

- Project rename from `pi-sandbox` to `agent-sandbox`.
- Expanded user and contributor documentation (`README`, `docs/*`, `CONTRIBUTING.md`).
- Added reusable release prompt under `.pi/prompts/release.md`.
