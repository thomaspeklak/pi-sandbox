# Feature Parity Checklist (Current `pis` → `ags --agent pi`)

## Core launcher parity
- [ ] Config resolution + strict validation
- [ ] Ensure image build if missing
- [ ] Ensure sandbox git signing config
- [ ] Dedicated SSH agent management
- [ ] Secret precedence parity (first successful source wins)
- [ ] Passthrough env parity
- [ ] Browser startup parity
- [ ] Wayland clipboard mount parity
- [ ] Mount logic parity (`optional`, `create`, `kind`, `when`)
- [ ] Podman security flags parity
- [ ] Guard read/write roots env parity
- [ ] Container boot dirs parity
- [ ] Browser network/port-forward parity
- [ ] External git metadata mount parity (worktrees)

## Companion command parity
- [ ] `setup` key generation + optional secret-store fill
- [ ] `doctor` checks + summary + exit codes
- [ ] `update` args/behavior parity
- [ ] `install/uninstall` symlink/bootstrap parity

## Acceptance scenarios
- [ ] Normal git repo
- [ ] Worktree with `.git` file to external root
- [ ] Browser mode on/off
- [ ] Optional mounts/tools missing
- [ ] Required mount missing failure path
- [ ] Secret from env + secret-tool fallback
- [ ] Signed commit + push path
