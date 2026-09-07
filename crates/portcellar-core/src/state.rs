use std::env;
use std::path::{Path, PathBuf};

const STATE_ROOT_ENV: &str = "PORTCELLAR_STATE_ROOT";

pub fn state_root() -> PathBuf {
    resolve_state_root(
        env::var_os(STATE_ROOT_ENV).as_deref().map(Path::new),
        current_project_root().as_deref(),
        env::var_os("HOME").as_deref().map(Path::new),
        cfg!(target_os = "macos"),
    )
}

fn resolve_state_root(
    override_root: Option<&Path>,
    project_root: Option<&Path>,
    home: Option<&Path>,
    macos: bool,
) -> PathBuf {
    if let Some(path) = override_root {
        return path.to_path_buf();
    }
    if let Some(path) = project_root {
        return path.join(".portcellar");
    }
    if let Some(path) = home {
        return if macos {
            path.join("Library/Application Support/PortCellar")
        } else {
            path.join(".local/share/portcellar")
        };
    }
    PathBuf::from(".portcellar")
}

pub(crate) fn current_project_root() -> Option<PathBuf> {
    let mut path = env::current_dir().ok()?;
    loop {
        if path.join("Cargo.toml").is_file()
            && path.join("crates/portcellar-core/Cargo.toml").is_file()
        {
            return Some(path);
        }
        if !path.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_root_override_wins_without_touching_the_filesystem() {
        let override_root = Path::new("/tmp/Port Cellar/café");
        let project_root = Path::new("/tmp/project");
        assert_eq!(
            resolve_state_root(Some(override_root), Some(project_root), None, true),
            override_root
        );
    }

    #[test]
    fn state_root_uses_project_then_platform_home_defaults() {
        let project_root = Path::new("/tmp/Project With Spaces");
        let home = Path::new("/tmp/Home With Spaces/café");
        assert_eq!(
            resolve_state_root(None, Some(project_root), Some(home), true),
            project_root.join(".portcellar")
        );
        assert_eq!(
            resolve_state_root(None, None, Some(home), true),
            home.join("Library/Application Support/PortCellar")
        );
        assert_eq!(
            resolve_state_root(None, None, Some(home), false),
            home.join(".local/share/portcellar")
        );
    }
}
