/// Capitalizes the first letter of each sentence.
/// A sentence starts after: beginning of text, '. ', '? ', '! ', '\n'
pub fn capitalize_sentences(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true;

    for ch in text.chars() {
        if capitalize_next && ch.is_alphabetic() {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
            if ch == '.' || ch == '?' || ch == '!' || ch == '\n' {
                capitalize_next = true;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize_first_letter() {
        assert_eq!(capitalize_sentences("hello world"), "Hello world");
    }

    #[test]
    fn test_capitalize_after_period() {
        assert_eq!(
            capitalize_sentences("hello world. this is a test."),
            "Hello world. This is a test."
        );
    }

    #[test]
    fn test_capitalize_after_question() {
        assert_eq!(
            capitalize_sentences("is this a test? yes it is."),
            "Is this a test? Yes it is."
        );
    }

    #[test]
    fn test_capitalize_after_exclamation() {
        assert_eq!(
            capitalize_sentences("wow! that is great."),
            "Wow! That is great."
        );
    }

    #[test]
    fn test_capitalize_after_newline() {
        assert_eq!(
            capitalize_sentences("line one.\nline two."),
            "Line one.\nLine two."
        );
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(capitalize_sentences(""), "");
    }

    #[test]
    fn test_already_capitalized() {
        assert_eq!(
            capitalize_sentences("Hello World. This Is Fine."),
            "Hello World. This Is Fine."
        );
    }
}
