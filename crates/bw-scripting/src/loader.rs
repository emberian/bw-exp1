//! Script loader utilities

use std::path::Path;

/// Validate a script path is safe (no path traversal).
pub fn validate_script_path(base: &str, path: &str) -> bool {
    // Normalize and check for path traversal
    if path.contains("..") {
        return false;
    }

    let full_path = Path::new(base).join(path);

    // Check it's actually under base
    match (full_path.canonicalize(), Path::new(base).canonicalize()) {
        (Ok(full), Ok(base)) => full.starts_with(base),
        _ => false,
    }
}

/// Get all .rhai files in a directory recursively.
pub fn find_scripts(dir: &Path) -> Vec<String> {
    let mut scripts = Vec::new();
    find_scripts_recursive(dir, dir, &mut scripts);
    scripts
}

fn find_scripts_recursive(base: &Path, dir: &Path, scripts: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                find_scripts_recursive(base, &path, scripts);
            } else if path.extension().is_some_and(|ext| ext == "rhai") {
                if let Ok(relative) = path.strip_prefix(base) {
                    scripts.push(relative.to_string_lossy().to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_traversal_prevention() {
        assert!(!validate_script_path("/scripts", "../etc/passwd"));
        assert!(!validate_script_path("/scripts", "missions/../../../etc/passwd"));
    }
}
