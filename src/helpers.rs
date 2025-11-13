/// Minimal PascalCase/camelCase to snake_case.
pub fn to_snake_case(name: &str) -> String {
    // Handles transitions from lower->upper and acronym boundaries reasonably.
    let mut out = String::with_capacity(name.len() * 2);
    let mut prev_is_lower_or_digit = false;
    let mut prev_is_upper = false;
    for ch in name.chars() {
        if ch.is_ascii_uppercase() {
            if prev_is_lower_or_digit || (prev_is_upper && !out.ends_with('_')) {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_is_lower_or_digit = false;
            prev_is_upper = true;
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_is_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
            prev_is_upper = false;
        } else {
            // Replace any non-alphanumeric with underscore boundary.
            if !out.ends_with('_') {
                out.push('_');
            }
            prev_is_lower_or_digit = false;
            prev_is_upper = false;
        }
    }
    // Trim possible leading/trailing underscores.
    while out.ends_with('_') {
        out.pop();
    }
    while out.starts_with('_') {
        out.remove(0);
    }
    out
}

/// Minimal snake_case to PascalCase.
pub fn to_pascal_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut new_word = true;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if new_word {
                out.push(ch.to_ascii_uppercase());
                new_word = false;
            } else {
                out.push(ch);
            }
        } else {
            new_word = true;
        }
    }
    out
}
