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
                if (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || (c >= b'0' && c <= b'9') || c == b'_' {
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
