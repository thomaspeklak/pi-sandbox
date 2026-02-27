# Agent Sandbox Migration Plan (pi-sandbox → ags)

Status: Draft v1  
Owner: @tom  
Last updated: 2026-02-27

---

## 1) Goal

Build a **single sandbox launcher** for multiple coding-agent harnesses with one command pattern:

```bash
ags --agent <pi|claude|codex|gemini|opencode> -- [agent args...]
```

Users can define their own aliases/wrappers externally.

Primary objective: **feature parity with current `pis` implementation first**, then add new harnesses safely.

---

## 2) Scope

### In scope
- Replace shell-heavy launcher internals with a **compiled binary** (Rust preferred, Go fallback).
- Keep Podman-based sandbox runtime and security posture.
- Preserve all current `pis` behavior for `--agent pi`.
- Add adapter architecture for: `pi`, `claude`, `codex`, `gemini`, `opencode`.
- Keep install/uninstall/setup/doctor/update workflows.

### Out of scope (for initial migration)
- Changing container runtime (Podman remains).
- Reworking secret backend model beyond parity.
- Major UX redesign beyond `ags --agent ...`.

---

## 3) High-level design

## 3.1 Runtime model
- **Core launcher** computes a launch plan (mounts/env/secrets/command).
- **Agent adapter** contributes agent-specific command + config mounts + preflight checks.
- **Podman executor** receives argv list only (no shell interpolation for user input).

### 3.2 Language decision
- [ ] Decision checkpoint: Rust confirmed (fallback: Go).
- [ ] Document rationale and constraints in `docs/adr/0001-launcher-language.md`.

### 3.3 Command UX
- [ ] Implement `ags --agent <name> -- ...`.
- [ ] Keep `pis`, `pisb`, `pis-doctor`, `pis-setup`, `pis-update` as compatibility wrappers initially.
- [ ] Add clear deprecation path/timeline for legacy wrappers.

### 3.4 Config strategy (scalable)
- Keep current config structure and add agent blocks gradually.
- Proposed direction:
  - global `[sandbox]`, `[[mount]]`, `[[secret]]`, `[[tool]]`, `[browser]`, `[update]`
  - new `[agent.<name>]` overlays for command/env/mount extras
- [ ] Define final schema in `docs/config-schema-v2.md`.

---

## 4) Feature parity checklist (must be complete before adding new agents)

Source of truth: current scripts in `bin/` + `scripts/resolve-config.py`.

### 4.1 Core launcher parity (`bin/pis-run`)
- [ ] Validate required host binaries (`python3`, `jq`, `podman`) equivalent behavior or replacement.
- [ ] Resolve config and strict validation parity.
- [ ] Build image if missing (`ensure_image`).
- [ ] Generate git signing config if missing (`ensure_gitconfig`).
- [ ] Dedicated SSH agent lifecycle parity + key loading.
- [ ] Secret resolution order parity (first successful source wins).
- [ ] Passthrough env behavior parity.
- [ ] Browser sidecar startup parity.
- [ ] Clipboard/Wayland mount behavior parity.
- [ ] Podman security flags parity (`no-new-privileges`, `cap-drop=all`, etc.).
- [ ] Mount handling parity: optional/create/file-vs-dir/when=always|browser.
- [ ] Guard roots env parity (`PI_SBOX_GUARD_READ_ROOTS_JSON`, `WRITE_ROOTS_JSON`).
- [ ] Boot directories creation parity (`container_boot_dirs`).
- [ ] Browser networking/port forward parity (`socat` behavior).
- [ ] Git worktree metadata auto-mount parity (external `.git`/`git-common-dir` support).

### 4.2 Config resolver parity (`scripts/resolve-config.py`)
- [ ] Type validation parity for all fields.
- [ ] Path expansion semantics parity (`~`, env vars, absolute resolve).
- [ ] Normalization parity for mounts/tools/secrets.
- [ ] Legacy secret provider compatibility parity.
- [ ] Browser/update defaults parity.

### 4.3 Setup/doctor/update/install parity
- [ ] `setup`: key generation behavior parity.
- [ ] `setup`: optional secret-store population parity.
- [ ] `doctor`: all checks parity and exit-code behavior parity.
- [ ] `update`: image rebuild args parity.
- [ ] `install/uninstall`: symlink migration + bootstrap behavior parity.

---

## 5) Implementation plan with tasks

## Phase 0 — Foundation
- [ ] Create repo structure for compiled launcher (`cmd/ags` or Rust crate root).
- [ ] Add CI job for build + lint + tests.
- [ ] Add golden test fixtures from current config examples.
- [ ] Add parity test harness (old shell vs new binary output plan comparison where feasible).

### Phase 1 — Config + launch plan engine
- [ ] Implement config parser/validator equivalent to `resolve-config.py`.
- [ ] Implement internal `LaunchPlan` model (env, mounts, workdir, command, warnings).
- [ ] Implement path utilities (normalize/containment checks).
- [ ] Implement secret source resolvers (`env`, `secret-tool`).
- [ ] Implement mount expansion from `[[tool]]` entries.

### Phase 2 — Podman executor + pi parity
- [ ] Implement image existence/build logic.
- [ ] Implement gitconfig bootstrap logic.
- [ ] Implement dedicated ssh-agent management.
- [ ] Implement browser startup/probe logic.
- [ ] Implement Wayland clipboard mount logic.
- [ ] Implement external git metadata discovery and mounts.
- [ ] Implement final podman argv assembly and exec.
- [ ] Add `ags --agent pi` adapter with full parity.

### Phase 3 — Companion commands
- [ ] Implement `ags doctor` parity checks.
- [ ] Implement `ags setup` parity.
- [ ] Implement `ags update` parity.
- [ ] Implement `ags install` / `ags uninstall` parity.
- [ ] Add compatibility wrapper scripts for current `pis*` commands.

### Phase 4 — Additional adapters
- [ ] Add `claude` adapter (command/env/config mounts + preflight).
- [ ] Add `codex` adapter.
- [ ] Add `gemini` adapter.
- [ ] Add `opencode` adapter.
- [ ] Add adapter capability matrix doc.

### Phase 5 — Hardening + rollout
- [ ] Security review of all mount paths and defaults.
- [ ] Backward compatibility test with existing user configs.
- [ ] Dogfood period using `--agent pi` only.
- [ ] Controlled enablement of non-pi adapters.
- [ ] Mark legacy shell implementation deprecated.

---

## 6) Testing strategy (parity guarantee)

### 6.1 Automated
- [ ] Unit tests: config validation and normalization.
- [ ] Unit tests: mount planner and optional/create modes.
- [ ] Unit tests: git metadata detection (normal repo + worktree).
- [ ] Unit tests: secret resolution precedence.
- [ ] Integration tests: generated podman argv snapshots.
- [ ] Integration tests: `doctor/setup/update` command behavior.

### 6.2 Golden parity tests
- [ ] Capture baseline outputs from current scripts for representative configs.
- [ ] Compare normalized plan objects from new launcher to baseline.
- [ ] Track intentional differences in an allowlist.

### 6.3 Manual acceptance matrix
- [ ] Standard repo launch.
- [ ] Git worktree with `.git` file pointing to external root.
- [ ] Browser mode enabled/disabled.
- [ ] Missing optional tools/mounts.
- [ ] Missing required mounts failure path.
- [ ] Secret from env and secret-tool fallback.
- [ ] SSH signing + push path.

---

## 7) Non-functional requirements

- [ ] Single static binary release per platform.
- [ ] No Python/jq dependency for runtime launcher path.
- [ ] Deterministic/structured logs (`--verbose`, `--json` optional).
- [ ] Clear, actionable error messages.
- [ ] Startup latency not worse than current scripts.

---

## 8) Open questions

- [ ] Final config schema shape: overlays vs full per-agent blocks?
- [ ] Should `--agent` be mandatory, or support auto-detect fallback?
- [ ] Should each agent have optional per-agent image override?
- [ ] How strict should we be on adapter capability mismatch (warn vs fail)?
- [ ] Keep browser feature global or adapter-specific?

---

## 9) Milestone exit criteria

### M1: `ags --agent pi` parity ready
- [ ] All Phase 1–2 tasks complete.
- [ ] Feature parity checklist for core launcher fully checked.
- [ ] Manual acceptance matrix green for pi.

### M2: Command suite parity ready
- [ ] Phase 3 complete.
- [ ] Existing `pis*` users can migrate with no behavior regressions.

### M3: Multi-agent beta ready
- [ ] At least two non-pi adapters complete and tested.
- [ ] Capability matrix published.
- [ ] Rollout notes and known limitations documented.

---

## 10) Suggested first execution slice (next 1–2 days)

- [ ] Confirm language: Rust.
- [ ] Scaffold binary + CLI parser with `ags --agent ... -- ...`.
- [ ] Implement config parsing for existing schema (read-only parse + print resolved JSON).
- [ ] Implement git worktree detection module + tests (including external `.git` metadata).
- [ ] Implement minimal pi adapter that launches podman with fixed command (no setup/doctor yet).

---

## 11) Tracking

- [ ] Create epic: `agent-sandbox`.
- [ ] Create sub-issues per phase/task group.
- [ ] Link each checkbox to issue IDs as they are created.
