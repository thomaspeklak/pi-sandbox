use std::fmt;
use std::process::Command;

use crate::config::ValidatedConfig;

/// Options for the update command.
pub struct UpdateOptions {
    pub pull: bool,
}

impl Default for UpdateOptions {
    fn default() -> Self {
        Self { pull: true }
    }
}

#[derive(Debug)]
pub enum UpdateError {
    MissingContainerfile(String),
    BuildFailed(String),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingContainerfile(p) => write!(f, "missing Containerfile: {p}"),
            Self::BuildFailed(msg) => write!(f, "podman build failed: {msg}"),
        }
    }
}

impl std::error::Error for UpdateError {}

/// Rebuild the sandbox container image (deps only — agents live in volumes).
pub fn run(config: &ValidatedConfig, opts: &UpdateOptions) -> Result<(), UpdateError> {
    let image = &config.sandbox.image;
    let containerfile = &config.sandbox.containerfile;

    if !containerfile.exists() {
        return Err(UpdateError::MissingContainerfile(
            containerfile.display().to_string(),
        ));
    }

    let context_dir = containerfile
        .parent()
        .expect("containerfile must have a parent directory");

    let mut args: Vec<String> = vec![
        "build".into(),
        "-t".into(),
        image.clone(),
        "-f".into(),
        containerfile.display().to_string(),
    ];

    if opts.pull {
        args.push("--pull".into());
    }

    args.push(context_dir.display().to_string());

    println!("Rebuilding {image}");

    let status = Command::new("podman")
        .args(&args)
        .status()
        .map_err(|e| UpdateError::BuildFailed(e.to_string()))?;

    if !status.success() {
        return Err(UpdateError::BuildFailed(format!("exited with {status}")));
    }

    println!("\nDone. Image rebuilt (deps only).");
    println!("Run 'ags update-agents' to install/update agents in volumes.");
    Ok(())
}
