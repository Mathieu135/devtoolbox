use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

#[derive(Debug)]
pub struct Project {
    root: PathBuf,
    compose_project_name: String,
}

impl Project {
    pub fn discover(start: &Path) -> Result<Self> {
        let canonical = start
            .canonicalize()
            .with_context(|| format!("le chemin {} n'existe pas", start.display()))?;
        let mut cursor = if canonical.is_file() {
            canonical
                .parent()
                .context("impossible de déterminer le dossier du projet")?
                .to_path_buf()
        } else {
            canonical
        };

        loop {
            if cursor.join("Cargo.toml").is_file() {
                let compose_project_name = compose_name(&cursor);
                return Ok(Self {
                    root: cursor,
                    compose_project_name,
                });
            }

            if !cursor.pop() {
                break;
            }
        }

        bail!(
            "aucun Cargo.toml trouvé depuis {} ou ses parents",
            start.display()
        )
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn compose_project_name(&self) -> &str {
        &self.compose_project_name
    }
}

fn compose_name(root: &Path) -> String {
    let raw = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("project");
    let mut sanitized = String::new();
    let mut previous_was_separator = false;

    for character in raw.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character);
            previous_was_separator = false;
        } else if !previous_was_separator && !sanitized.is_empty() {
            sanitized.push('-');
            previous_was_separator = true;
        }
    }

    let sanitized = sanitized.trim_matches('-');
    let sanitized = if sanitized.is_empty() {
        "project"
    } else {
        sanitized
    };
    let shortened = sanitized.chars().take(40).collect::<String>();
    let mut path_hash = 0x811c_9dc5_u32;
    for byte in root.to_string_lossy().bytes() {
        path_hash ^= u32::from(byte);
        path_hash = path_hash.wrapping_mul(0x0100_0193);
    }
    format!("toolbox-{shortened}-{path_hash:08x}")
}

#[cfg(test)]
mod tests {
    use super::compose_name;
    use std::path::Path;

    #[test]
    fn compose_name_is_portable() {
        let name = compose_name(Path::new("/tmp/My Rust_app!"));
        assert!(name.starts_with("toolbox-my-rust-app-"));
        assert!(name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        }));
    }

    #[test]
    fn same_directory_name_in_different_paths_does_not_collide() {
        assert_ne!(
            compose_name(Path::new("/workspace/alpha/api")),
            compose_name(Path::new("/workspace/beta/api"))
        );
    }
}
