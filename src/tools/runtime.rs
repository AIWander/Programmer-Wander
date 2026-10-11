//! Portable runtime paths shared by stateful Programmer tools.

use std::path::{Path, PathBuf};

pub fn state_dir() -> PathBuf {
    let override_dir = std::env::var_os("PROGRAMMER_STATE_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let current_dir = std::env::current_dir().ok();

    resolve_state_dir(override_dir, executable_dir, current_dir)
}

pub fn state_path(component: &str) -> PathBuf {
    state_dir().join(component)
}

pub fn default_working_dir() -> PathBuf {
    if let Ok(current) = std::env::current_dir() {
        if current.is_dir() {
            return current;
        }
    }

    #[cfg(windows)]
    {
        let drive = std::env::var_os("SystemDrive")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "C:".into());
        let root = PathBuf::from(format!("{}\\", drive.to_string_lossy()));
        if root.is_dir() {
            return root;
        }
    }

    PathBuf::from(std::path::MAIN_SEPARATOR.to_string())
}

fn resolve_state_dir(
    override_dir: Option<PathBuf>,
    executable_dir: Option<PathBuf>,
    current_dir: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = override_dir {
        if path.is_absolute() {
            return path;
        }
        if let Some(cwd) = current_dir.as_ref() {
            return cwd.join(path);
        }
        return path;
    }

    executable_dir
        .or(current_dir)
        .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR.to_string()))
        .join(".programmer")
}

#[cfg(test)]
mod tests {
    use super::resolve_state_dir;
    use std::path::PathBuf;

    #[test]
    fn explicit_state_override_wins() {
        let resolved = resolve_state_dir(
            Some(PathBuf::from(r"D:\ProgrammerState")),
            Some(PathBuf::from(r"C:\Apps")),
            Some(PathBuf::from(r"C:\Work")),
        );
        assert_eq!(resolved, PathBuf::from(r"D:\ProgrammerState"));
    }

    #[test]
    fn relative_override_is_resolved_from_current_directory() {
        let resolved = resolve_state_dir(
            Some(PathBuf::from("state")),
            Some(PathBuf::from(r"C:\Apps")),
            Some(PathBuf::from(r"D:\Work")),
        );
        assert_eq!(resolved, PathBuf::from(r"D:\Work").join("state"));
    }

    #[test]
    fn state_defaults_beside_executable_then_current_directory() {
        let beside_exe = resolve_state_dir(
            None,
            Some(PathBuf::from(r"C:\Apps\Programmer")),
            Some(PathBuf::from(r"D:\Work")),
        );
        assert_eq!(beside_exe, PathBuf::from(r"C:\Apps\Programmer\.programmer"));

        let beside_cwd = resolve_state_dir(None, None, Some(PathBuf::from(r"D:\Work")));
        assert_eq!(beside_cwd, PathBuf::from(r"D:\Work\.programmer"));
    }
}
