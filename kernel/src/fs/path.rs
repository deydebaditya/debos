//! Path Manipulation Utilities
//!
//! Handles path parsing, normalization, and resolution.

use alloc::string::String;
use alloc::vec::Vec;

/// Normalize a path by resolving `.`, `..`, and redundant slashes
pub fn normalize_path(path: &str) -> String {
    let is_absolute = path.starts_with('/');
    let mut components: Vec<&str> = Vec::new();
    
    for component in path.split('/') {
        match component {
            // Skip empty components (from multiple slashes) and current dir
            "" | "." => continue,
            // Go up one directory
            ".." => {
                if !components.is_empty() && components.last() != Some(&"..") {
                    components.pop();
                } else if !is_absolute {
                    // For relative paths, keep the ..
                    components.push("..");
                }
                // For absolute paths, ignore .. at root
            }
            // Regular component
            comp => components.push(comp),
        }
    }
    
    if is_absolute {
        if components.is_empty() {
            String::from("/")
        } else {
            let mut result = String::new();
            for comp in components {
                result.push('/');
                result.push_str(comp);
            }
            result
        }
    } else {
        if components.is_empty() {
            String::from(".")
        } else {
            components.join("/")
        }
    }
}

/// Join two paths
pub fn join_path(base: &str, path: &str) -> String {
    if path.starts_with('/') {
        // Absolute path, ignore base
        normalize_path(path)
    } else if base.is_empty() || base == "." {
        normalize_path(path)
    } else {
        let combined = if base.ends_with('/') {
            alloc::format!("{}{}", base, path)
        } else {
            alloc::format!("{}/{}", base, path)
        };
        normalize_path(&combined)
    }
}

/// Get the parent directory of a path
pub fn parent(path: &str) -> Option<String> {
    let normalized = normalize_path(path);
    
    if normalized == "/" {
        return None;
    }
    
    if let Some(pos) = normalized.rfind('/') {
        if pos == 0 {
            Some(String::from("/"))
        } else {
            Some(String::from(&normalized[..pos]))
        }
    } else {
        Some(String::from("."))
    }
}

/// Get the filename component of a path
pub fn filename(path: &str) -> Option<&str> {
    let normalized = path.trim_end_matches('/');
    
    if normalized.is_empty() || normalized == "/" {
        return None;
    }
    
    if let Some(pos) = normalized.rfind('/') {
        Some(&normalized[pos + 1..])
    } else {
        Some(normalized)
    }
}

/// Split path into parent and filename
pub fn split(path: &str) -> (String, String) {
    let normalized = normalize_path(path);
    
    if normalized == "/" {
        return (String::from("/"), String::new());
    }
    
    if let Some(pos) = normalized.rfind('/') {
        let parent = if pos == 0 {
            String::from("/")
        } else {
            String::from(&normalized[..pos])
        };
        let name = String::from(&normalized[pos + 1..]);
        (parent, name)
    } else {
        (String::from("."), normalized)
    }
}

/// Check if a path is absolute
pub fn is_absolute(path: &str) -> bool {
    path.starts_with('/')
}

/// Get path components as a vector
pub fn components(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normalize() {
        assert_eq!(normalize_path("/a/b/../c"), "/a/c");
        assert_eq!(normalize_path("/a/./b"), "/a/b");
        assert_eq!(normalize_path("//a///b//"), "/a/b");
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path("/.."), "/");
        assert_eq!(normalize_path("a/b/c"), "a/b/c");
        assert_eq!(normalize_path("../a"), "../a");
    }
    
    #[test]
    fn test_join() {
        assert_eq!(join_path("/home", "user"), "/home/user");
        assert_eq!(join_path("/home/", "user"), "/home/user");
        assert_eq!(join_path("/home", "/etc"), "/etc");
        assert_eq!(join_path(".", "file"), "file");
    }
    
    #[test]
    fn test_parent() {
        assert_eq!(parent("/home/user"), Some(String::from("/home")));
        assert_eq!(parent("/home"), Some(String::from("/")));
        assert_eq!(parent("/"), None);
    }
    
    #[test]
    fn test_filename() {
        assert_eq!(filename("/home/user"), Some("user"));
        assert_eq!(filename("/"), None);
        assert_eq!(filename("file.txt"), Some("file.txt"));
    }
}

