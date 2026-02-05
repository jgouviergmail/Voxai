/// Normalizes spacing in text:
/// - Collapses multiple spaces to one
/// - Removes spaces before punctuation (. , ; : ? !)
/// - Ensures space after punctuation
/// - Trims leading/trailing whitespace
pub fn normalize_spacing(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Step 1: Collapse multiple spaces to one
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        if ch == ' ' {
            // Skip extra spaces
            if !result.is_empty() && !result.ends_with(' ') {
                result.push(' ');
            }
        } else if is_punctuation(ch) {
            // Remove trailing space before punctuation
            if result.ends_with(' ') {
                result.pop();
            }
            result.push(ch);
            // Ensure space after punctuation (if next char is not space, newline, or end)
            if i + 1 < len && chars[i + 1] != ' ' && chars[i + 1] != '\n' && !is_punctuation(chars[i + 1]) {
                result.push(' ');
            }
        } else {
            result.push(ch);
        }

        i += 1;
    }

    result.trim().to_string()
}

fn is_punctuation(ch: char) -> bool {
    matches!(ch, '.' | ',' | ';' | ':' | '?' | '!')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collapse_multiple_spaces() {
        assert_eq!(normalize_spacing("hello   world"), "hello world");
    }

    #[test]
    fn test_remove_space_before_punctuation() {
        assert_eq!(normalize_spacing("hello , world"), "hello, world");
        assert_eq!(normalize_spacing("hello ."), "hello.");
    }

    #[test]
    fn test_add_space_after_punctuation() {
        assert_eq!(normalize_spacing("hello.world"), "hello. world");
        assert_eq!(normalize_spacing("hello,world"), "hello, world");
    }

    #[test]
    fn test_trim() {
        assert_eq!(normalize_spacing("  hello world  "), "hello world");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(normalize_spacing(""), "");
    }

    #[test]
    fn test_already_correct() {
        assert_eq!(normalize_spacing("Hello, world."), "Hello, world.");
    }

    #[test]
    fn test_multiple_punctuation() {
        assert_eq!(
            normalize_spacing("hello ..."),
            "hello..."
        );
    }
}
