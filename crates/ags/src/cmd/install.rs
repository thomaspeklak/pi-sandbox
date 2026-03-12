use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::assets;
use crate::cli::InstallOptions;

#[derive(Debug)]
pub enum InstallError {
    Io(io::Error),
    HomeDir,
    InvalidMountDir(String),
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "install I/O error: {e}"),
            Self::HomeDir => f.write_str("could not determine home directory"),
            Self::InvalidMountDir(msg) => write!(f, "invalid directory mount: {msg}"),
        }
    }
}

impl std::error::Error for InstallError {}

impl From<io::Error> for InstallError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Install ags: write embedded assets and ensure config layout.
pub fn run(opts: &InstallOptions) -> Result<(), InstallError> {
    let home = dirs::home_dir().ok_or(InstallError::HomeDir)?;
    let bin_dir = home.join(".local/bin");
    let config_dir = home.join(".config/ags");
    let agent_dir =
        std::env::var("AGS_AGENT_DIR").map_or_else(|_| config_dir.join("pi"), PathBuf::from);

    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(agent_dir.join("extensions"))?;

    // Write embedded Containerfile + tmux config
    let containerfile = config_dir.join("Containerfile");
    assets::ensure_containerfile(&containerfile)?;
    println!("Wrote Containerfile: {}", containerfile.display());

    let tmux_conf = config_dir.join("tmux.conf");
    assets::ensure_tmux_conf(&tmux_conf)?;
    println!("Wrote tmux config: {}", tmux_conf.display());

    // Write Pi guard extension
    assets::ensure_guard_extension(&agent_dir)?;
    println!(
        "Wrote guard extension: {}",
        agent_dir.join("extensions/guard.ts").display()
    );

    // Write Claude guard hook
    let hooks_dir = config_dir.join("ags-hooks");
    assets::ensure_claude_guard_hook(&hooks_dir)?;
    println!(
        "Wrote Claude guard hook: {}",
        hooks_dir.join("guard.sh").display()
    );
    assets::ensure_claude_guard_skill(&hooks_dir)?;
    println!(
        "Wrote Claude guard skill: {}",
        hooks_dir.join("skills/guard/SKILL.md").display()
    );

    // Write settings template (only if missing)
    let settings = agent_dir.join("settings.json");
    if !settings.exists() {
        assets::ensure_settings_template(&agent_dir)?;
        println!("Wrote settings template: {}", settings.display());
    } else {
        println!("Using existing settings: {}", settings.display());
    }

    // Remove legacy config-dir symlink if it points elsewhere
    remove_legacy_symlink(&config_dir);

    let config_path = config_dir.join("config.toml");
    if opts.add_agent_mounts {
        ensure_agent_mounts_block(&config_path)?;
    }
    if !opts.add_dir_mounts.is_empty() {
        ensure_dir_mounts_block(&config_path, &opts.add_dir_mounts)?;
    }

    if opts.link_self {
        fs::create_dir_all(&bin_dir)?;
        let link_path = bin_dir.join("ags");
        link_self_executable(&link_path, opts.force)?;
    } else if opts.force {
        eprintln!("warning: --force has no effect without --link-self");
    }

    // Legacy binary/alias cleanup intentionally omitted.
    // Binary aliases are treated as userland responsibility.

    println!("\nInstall complete.");
    println!("Run: ags doctor");
    if !opts.link_self {
        println!("Tip: run `ags install --link-self` to link ags into ~/.local/bin");
    }
    Ok(())
}

/// Uninstall currently performs no binary alias cleanup.
pub fn uninstall() -> Result<(), InstallError> {
    let _ = dirs::home_dir().ok_or(InstallError::HomeDir)?;
    println!("Uninstall complete.");
    Ok(())
}

fn ensure_agent_mounts_block(config_path: &Path) -> Result<(), InstallError> {
    if !config_path.exists() {
        eprintln!(
            "warning: {} does not exist; skipped --add-agent-mounts",
            config_path.display()
        );
        return Ok(());
    }

    let content = fs::read_to_string(config_path)?;
    if content.contains("container = \"/home/dev/.pi\"")
        && content.contains("container = \"/home/dev/.claude\"")
        && content.contains("container = \"/home/dev/.claude.json\"")
        && content.contains("container = \"/home/dev/.codex\"")
        && content.contains("container = \"/home/dev/.gemini\"")
        && content.contains("container = \"/home/dev/.config/opencode\"")
    {
        println!("Agent mounts already present in {}", config_path.display());
        return Ok(());
    }

    let block = r#"
# Added by `ags install --add-agent-mounts`
[[agent_mount]]
host = "~/.claude.json"
container = "/home/dev/.claude.json"
kind = "file"

[[agent_mount]]
host = "~/.claude"
container = "/home/dev/.claude"

[[agent_mount]]
host = "~/.codex"
container = "/home/dev/.codex"

[[agent_mount]]
host = "~/.pi"
container = "/home/dev/.pi"

[[agent_mount]]
host = "~/.config/opencode"
container = "/home/dev/.config/opencode"

[[agent_mount]]
host = "~/.gemini"
container = "/home/dev/.gemini"
"#;

    let mut updated = content;
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(block);

    fs::write(config_path, updated)?;
    println!("Appended default agent mounts to {}", config_path.display());

    Ok(())
}

fn ensure_dir_mounts_block(config_path: &Path, dirs: &[String]) -> Result<(), InstallError> {
    if !config_path.exists() {
        eprintln!(
            "warning: {} does not exist; skipped --add-dir-mount",
            config_path.display()
        );
        return Ok(());
    }

    let mut content = fs::read_to_string(config_path)?;
    let mut appended = 0u32;

    for raw_dir in dirs {
        let host_dir = canonicalize_mount_dir(raw_dir)?;
        let host = host_dir.display().to_string();
        let needle = format!("host = \"{host}\"\ncontainer = \"{host}\"");
        if content.contains(&needle) {
            continue;
        }

        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!(
            "\n# Added by `ags install --add-dir-mount`\n[[mount]]\nhost = \"{host}\"\ncontainer = \"{host}\"\nmode = \"rw\"\nkind = \"dir\"\n"
        ));
        appended += 1;
    }

    if appended == 0 {
        println!(
            "Directory mounts already present in {}",
            config_path.display()
        );
        return Ok(());
    }

    fs::write(config_path, content)?;
    println!(
        "Appended {appended} additional directory mount(s) to {}",
        config_path.display()
    );
    Ok(())
}

fn canonicalize_mount_dir(raw_dir: &str) -> Result<PathBuf, InstallError> {
    let path = if let Some(rest) = raw_dir.strip_prefix("~/") {
        dirs::home_dir().ok_or(InstallError::HomeDir)?.join(rest)
    } else if raw_dir == "~" {
        dirs::home_dir().ok_or(InstallError::HomeDir)?
    } else {
        PathBuf::from(raw_dir)
    };

    let canonical = fs::canonicalize(&path)
        .map_err(|e| InstallError::InvalidMountDir(format!("{raw_dir} ({})", e)))?;
    if !canonical.is_dir() {
        return Err(InstallError::InvalidMountDir(format!(
            "{raw_dir} ({}) is not a directory",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn link_self_executable(link_path: &Path, force: bool) -> Result<(), InstallError> {
    let mut current = std::env::current_exe()?;
    if let Ok(canon) = fs::canonicalize(&current) {
        current = canon;
    }

    if !is_stable_executable_path(&current) && !force {
        eprintln!(
            "warning: current executable looks build-local ({}); skipping link (use --force to override)",
            current.display()
        );
        return Ok(());
    }

    if let Ok(meta) = fs::symlink_metadata(link_path) {
        if meta.file_type().is_symlink() {
            if let Ok(existing_target) = fs::read_link(link_path) {
                let existing_abs = if existing_target.is_absolute() {
                    existing_target
                } else {
                    link_path
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(existing_target)
                };
                if let Ok(existing_canon) = fs::canonicalize(existing_abs)
                    && existing_canon == current
                {
                    println!("Self-link already up to date: {}", link_path.display());
                    return Ok(());
                }
            }

            if !force {
                eprintln!(
                    "warning: existing symlink at {} (use --force to replace)",
                    link_path.display()
                );
                return Ok(());
            }
            fs::remove_file(link_path)?;
        } else {
            if !force {
                eprintln!(
                    "warning: existing non-symlink at {} (use --force to replace)",
                    link_path.display()
                );
                return Ok(());
            }
            if meta.is_dir() {
                eprintln!(
                    "warning: {} is a directory; refusing to replace",
                    link_path.display()
                );
                return Ok(());
            }
            fs::remove_file(link_path)?;
        }
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&current, link_path)?;
        println!(
            "Linked self binary: {} -> {}",
            link_path.display(),
            current.display()
        );
    }
    #[cfg(not(unix))]
    {
        let _ = (current, link_path);
        eprintln!("warning: self-link install is only implemented on unix platforms");
    }

    Ok(())
}

fn is_stable_executable_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    !s.contains("/target/debug/")
        && !s.contains("/target/release/")
        && !s.contains("\\target\\debug\\")
        && !s.contains("\\target\\release\\")
}

/// If `path` is a symlink, remove it so we can use it as a real directory.
fn remove_legacy_symlink(path: &Path) {
    if let Ok(meta) = fs::symlink_metadata(path)
        && meta.file_type().is_symlink()
    {
        let _ = fs::remove_file(path);
        println!("Removed legacy config symlink: {}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::{canonicalize_mount_dir, ensure_dir_mounts_block};
    use std::fs;

    #[test]
    fn dir_mount_block_appends_same_path_mount() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.toml");
        fs::write(&config_path, "[sandbox]\nimage = \"x\"\n").unwrap();

        let extra_dir = tmp.path().join("extra");
        fs::create_dir_all(&extra_dir).unwrap();

        ensure_dir_mounts_block(&config_path, &[extra_dir.display().to_string()]).unwrap();

        let updated = fs::read_to_string(&config_path).unwrap();
        let dir = extra_dir.display().to_string();
        assert!(updated.contains("[[mount]]"));
        assert!(updated.contains(&format!("host = \"{dir}\"")));
        assert!(updated.contains(&format!("container = \"{dir}\"")));
        assert!(updated.contains("mode = \"rw\""));
        assert!(updated.contains("kind = \"dir\""));
    }

    #[test]
    fn dir_mount_block_skips_existing_mount() {
        let tmp = tempfile::tempdir().unwrap();
        let extra_dir = tmp.path().join("extra");
        fs::create_dir_all(&extra_dir).unwrap();
        let dir = extra_dir.display().to_string();
        let config_path = tmp.path().join("config.toml");
        fs::write(
            &config_path,
            format!(
                "[[mount]]\nhost = \"{dir}\"\ncontainer = \"{dir}\"\nmode = \"rw\"\nkind = \"dir\"\n"
            ),
        )
        .unwrap();

        ensure_dir_mounts_block(&config_path, std::slice::from_ref(&dir)).unwrap();

        let updated = fs::read_to_string(&config_path).unwrap();
        assert_eq!(updated.matches("[[mount]]").count(), 1);
    }

    #[test]
    fn canonicalize_mount_dir_rejects_missing_path() {
        let err = canonicalize_mount_dir("/definitely/missing/ags-test-dir").unwrap_err();
        assert!(err.to_string().contains("invalid directory mount"));
    }
}
