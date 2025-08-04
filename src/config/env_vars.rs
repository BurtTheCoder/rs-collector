/// Parse Windows-style environment variables (%VAR%)
pub fn parse_windows_env_vars(path: &str) -> String {
    let mut result = path.to_string();

    // Handle %USERPROFILE% and other environment variables
    if result.contains('%') {
        // Process each environment variable
        let mut i = 0;
        while let Some(start) = result[i..].find('%') {
            let real_start = i + start;
            if let Some(end) = result[real_start + 1..].find('%') {
                let real_end = real_start + 1 + end;
                let var_name = &result[real_start + 1..real_end];

                if let Ok(var_value) = std::env::var(var_name) {
                    let to_replace = format!("%{}%", var_name);
                    result = result.replacen(&to_replace, &var_value, 1);
                    // Reset position after replacement
                    i = 0;
                } else {
                    // Skip this var if it doesn't exist
                    i = real_end + 1;
                }
            } else {
                // No closing %, move past this one
                i = real_start + 1;
            }
        }
    }

    result
}

/// Parse Unix-style environment variables ($VAR and ${VAR})
pub fn parse_unix_env_vars(path: &str) -> String {
    let mut result = path.to_string();

    // First handle ${VAR} style variables
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start + 2..].find('}') {
            let var_name = &result[start + 2..start + 2 + end];
            let to_replace = format!("${{{}}}", var_name);

            if let Ok(var_value) = std::env::var(var_name) {
                result = result.replacen(&to_replace, &var_value, 1);
            } else {
                // Skip this var if it doesn't exist
                result = result.replacen(&to_replace, "", 1);
            }
        } else {
            // No closing }, break to avoid infinite loop
            break;
        }
    }

    // Then handle $VAR style variables
    // This is more complex because we need to determine where the variable name ends
    let mut i = 0;
    while i < result.len() {
        if let Some(pos) = result[i..].find('$') {
            let var_start = i + pos;

            // Skip if this is the end of the string or if it's a ${VAR} style variable
            if var_start + 1 >= result.len() || result.as_bytes()[var_start + 1] == b'{' {
                i = var_start + 1;
                continue;
            }

            // Find the end of the variable name (first non-alphanumeric, non-underscore character)
            let mut var_end = var_start + 1;
            while var_end < result.len() {
                let c = result.as_bytes()[var_end];
                if (c >= b'a' && c <= b'z')
                    || (c >= b'A' && c <= b'Z')
                    || (c >= b'0' && c <= b'9')
                    || c == b'_'
                {
                    var_end += 1;
                } else {
                    break;
                }
            }

            if var_end > var_start + 1 {
                let var_name = &result[var_start + 1..var_end];
                let to_replace = format!("${}", var_name);

                if let Ok(var_value) = std::env::var(var_name) {
                    result = result.replacen(&to_replace, &var_value, 1);
                    // Reset position after replacement
                    i = 0;
                } else {
                    // Skip this var if it doesn't exist
                    i = var_end;
                }
            } else {
                i = var_start + 1;
            }
        } else {
            break;
        }
    }

    result
}

/// Normalize path separators for the current OS
pub fn normalize_path_for_os(path: &str) -> String {
    if cfg!(windows) {
        // Ensure Windows paths use backslashes
        path.replace('/', "\\")
    } else {
        // Ensure Unix paths use forward slashes
        path.replace('\\', "/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_parse_windows_env_vars_basic() {
        // Set test environment variables
        env::set_var("TESTVAR", "testvalue");
        env::set_var("USERPROFILE", "/home/user");

        // Test single variable
        assert_eq!(parse_windows_env_vars("%TESTVAR%"), "testvalue");

        // Test variable in path
        assert_eq!(
            parse_windows_env_vars("%USERPROFILE%\\Documents"),
            "/home/user\\Documents"
        );

        // Clean up
        env::remove_var("TESTVAR");
        env::remove_var("USERPROFILE");
    }

    #[test]
    fn test_parse_windows_env_vars_multiple() {
        env::set_var("VAR1", "value1");
        env::set_var("VAR2", "value2");

        // Test multiple variables
        assert_eq!(
            parse_windows_env_vars("%VAR1%\\%VAR2%\\file.txt"),
            "value1\\value2\\file.txt"
        );

        // Clean up
        env::remove_var("VAR1");
        env::remove_var("VAR2");
    }

    #[test]
    fn test_parse_windows_env_vars_nonexistent() {
        // Test non-existent variable (should remain unchanged)
        assert_eq!(
            parse_windows_env_vars("%DOESNOTEXIST%\\file.txt"),
            "%DOESNOTEXIST%\\file.txt"
        );
    }

    #[test]
    fn test_parse_windows_env_vars_malformed() {
        // Test malformed variables
        assert_eq!(parse_windows_env_vars("%INCOMPLETE"), "%INCOMPLETE");
        assert_eq!(parse_windows_env_vars("%%"), "%%");
        assert_eq!(parse_windows_env_vars("%"), "%");
    }

    #[test]
    fn test_parse_unix_env_vars_dollar_style() {
        env::set_var("HOME", "/home/user");
        env::set_var("USER", "testuser");

        // Test $VAR style
        assert_eq!(parse_unix_env_vars("$HOME"), "/home/user");
        assert_eq!(
            parse_unix_env_vars("$HOME/Documents"),
            "/home/user/Documents"
        );
        assert_eq!(parse_unix_env_vars("$USER-file.txt"), "testuser-file.txt");

        // Clean up
        env::remove_var("HOME");
        env::remove_var("USER");
    }

    #[test]
    fn test_parse_unix_env_vars_brace_style() {
        env::set_var("MYVAR", "myvalue");
        env::set_var("PATH_VAR", "/usr/local");

        // Test ${VAR} style
        assert_eq!(parse_unix_env_vars("${MYVAR}"), "myvalue");
        assert_eq!(parse_unix_env_vars("${PATH_VAR}/bin"), "/usr/local/bin");
        assert_eq!(
            parse_unix_env_vars("prefix-${MYVAR}-suffix"),
            "prefix-myvalue-suffix"
        );

        // Clean up
        env::remove_var("MYVAR");
        env::remove_var("PATH_VAR");
    }

    #[test]
    fn test_parse_unix_env_vars_mixed_styles() {
        env::set_var("VAR1", "value1");
        env::set_var("VAR2", "value2");

        // Test mixed styles
        assert_eq!(
            parse_unix_env_vars("$VAR1/${VAR2}/file"),
            "value1/value2/file"
        );

        // Clean up
        env::remove_var("VAR1");
        env::remove_var("VAR2");
    }

    #[test]
    fn test_parse_unix_env_vars_nonexistent() {
        // Test non-existent variables
        assert_eq!(parse_unix_env_vars("$DOESNOTEXIST"), "$DOESNOTEXIST");
        assert_eq!(parse_unix_env_vars("${DOESNOTEXIST}"), "");
        assert_eq!(
            parse_unix_env_vars("$DOESNOTEXIST/path"),
            "$DOESNOTEXIST/path"
        );
    }

    #[test]
    fn test_parse_unix_env_vars_edge_cases() {
        // Test edge cases
        assert_eq!(parse_unix_env_vars("$"), "$");
        assert_eq!(parse_unix_env_vars("${"), "${");
        assert_eq!(parse_unix_env_vars("${incomplete"), "${incomplete");
        assert_eq!(parse_unix_env_vars("$$"), "$$");

        // Test variable name boundaries
        env::set_var("VAR", "value");
        assert_eq!(parse_unix_env_vars("$VAR123"), "$VAR123"); // 123 is part of var name
        assert_eq!(parse_unix_env_vars("$VAR-123"), "value-123"); // - ends var name
        assert_eq!(parse_unix_env_vars("$VAR_TEST"), "$VAR_TEST"); // _ is part of var name
        env::remove_var("VAR");
    }

    #[test]
    fn test_normalize_path_for_os() {
        // Test Windows normalization
        if cfg!(windows) {
            assert_eq!(normalize_path_for_os("C:/Users/test"), "C:\\Users\\test");
            assert_eq!(normalize_path_for_os("path/to/file"), "path\\to\\file");
            assert_eq!(
                normalize_path_for_os("already\\correct"),
                "already\\correct"
            );
            assert_eq!(
                normalize_path_for_os("mixed/path\\style"),
                "mixed\\path\\style"
            );
        } else {
            // Test Unix normalization
            assert_eq!(normalize_path_for_os("C:\\Users\\test"), "C:/Users/test");
            assert_eq!(normalize_path_for_os("path\\to\\file"), "path/to/file");
            assert_eq!(normalize_path_for_os("already/correct"), "already/correct");
            assert_eq!(
                normalize_path_for_os("mixed\\path/style"),
                "mixed/path/style"
            );
        }
    }

    #[test]
    fn test_normalize_path_empty_and_special() {
        // Test empty and special cases
        assert_eq!(normalize_path_for_os(""), "");
        assert_eq!(normalize_path_for_os("/"), "/");
        assert_eq!(
            normalize_path_for_os("\\"),
            if cfg!(windows) { "\\" } else { "/" }
        );

        // Test multiple consecutive separators
        if cfg!(windows) {
            assert_eq!(
                normalize_path_for_os("path//to///file"),
                "path\\\\to\\\\\\file"
            );
        } else {
            assert_eq!(
                normalize_path_for_os("path\\\\to\\\\\\file"),
                "path//to///file"
            );
        }
    }

    #[test]
    fn test_complex_path_with_env_vars() {
        env::set_var("APPDATA", "/home/user/.config");
        env::set_var("USERNAME", "testuser");

        // Test complex Windows-style path
        let windows_path = "%APPDATA%\\MyApp\\%USERNAME%\\config.ini";
        let parsed = parse_windows_env_vars(windows_path);
        assert_eq!(parsed, "/home/user/.config\\MyApp\\testuser\\config.ini");

        // Test complex Unix-style path
        let unix_path = "$HOME/.config/${USERNAME}/settings";
        env::set_var("HOME", "/home/user");
        let parsed = parse_unix_env_vars(unix_path);
        assert_eq!(parsed, "/home/user/.config/testuser/settings");

        // Clean up
        env::remove_var("APPDATA");
        env::remove_var("USERNAME");
        env::remove_var("HOME");
    }

    #[test]
    fn test_special_characters_in_env_values() {
        // Test environment variables containing special characters
        env::set_var("SPECIAL", "value with spaces & symbols!");

        assert_eq!(
            parse_windows_env_vars("%SPECIAL%\\file.txt"),
            "value with spaces & symbols!\\file.txt"
        );

        assert_eq!(
            parse_unix_env_vars("${SPECIAL}/file.txt"),
            "value with spaces & symbols!/file.txt"
        );

        env::remove_var("SPECIAL");
    }
}
