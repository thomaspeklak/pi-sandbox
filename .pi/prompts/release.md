# Release

Prepare and publish a new `ags` release.

## Goals

- bump version in `crates/ags/Cargo.toml`
- ensure docs/tests/build are green
- commit + push
- let GitHub Actions publish release artifacts automatically

## Checklist

1. Update version in:
   - `crates/ags/Cargo.toml` (`[package].version`)
2. Run quality checks:
   - `cargo fmt`
   - `cargo clippy -p ags -- -D warnings`
   - `cargo test -p ags`
3. Verify docs mention any user-visible changes:
   - `README.md`
   - `docs/COMMANDS.md`
   - `docs/CONFIG.md`
   - `docs/TROUBLESHOOTING.md`
4. Commit:
   - `git add -A`
   - `git commit -m "release: vX.Y.Z"`
5. Push to `main`.

## Notes

- `.github/workflows/release.yml` watches `crates/ags/Cargo.toml` on `main`.
- If version changed, workflow will:
  - build `ags` in release mode
  - package `ags-<version>-linux-x86_64.tar.gz`
  - upload checksum file
  - create/update GitHub release tag `v<version>`

## Manual trigger

You can also run the workflow manually from GitHub Actions (`workflow_dispatch`).
