use crate::config::schema::SubstitutionRule;

/// Applies user-defined substitution rules to the text.
/// This runs LAST in the pipeline so LLM never overrides substitutions.
pub fn apply_substitutions(text: &str, rules: &[SubstitutionRule]) -> String {
    let mut result = text.to_string();

    for rule in rules {
        if rule.case_sensitive {
            result = result.replace(&rule.from, &rule.to);
        } else {
            result = case_insensitive_replace(&result, &rule.from, &rule.to);
        }
    }

    result
}

fn case_insensitive_replace(text: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return text.to_string();
    }

    let lower_from = from.to_lowercase();

    // Build lowered text and a byte-position map (lowered pos → original pos).
    // This is necessary because to_lowercase() can change byte lengths
    // (e.g., ẞ U+1E9E 3 bytes → ß U+00DF 2 bytes, İ U+0130 2 bytes → i̇ 3 bytes).
    let mut lower_text = String::with_capacity(text.len());
    let mut lower_to_orig: Vec<(usize, usize)> = Vec::new();

    for (orig_pos, ch) in text.char_indices() {
        lower_to_orig.push((lower_text.len(), orig_pos));
        for lc in ch.to_lowercase() {
            lower_text.push(lc);
        }
    }
    lower_to_orig.push((lower_text.len(), text.len()));

    let mut result = String::with_capacity(text.len());
    let mut last_orig_end: usize = 0;

    for (lower_start, _) in lower_text.match_indices(&lower_from) {
        let lower_end = lower_start + lower_from.len();

        // Map lowered byte positions back to original byte positions.
        // Skip matches whose endpoints don't align with character boundaries.
        let orig_start = lower_to_orig
            .binary_search_by_key(&lower_start, |&(lp, _)| lp)
            .ok()
            .map(|idx| lower_to_orig[idx].1);
        let orig_end = lower_to_orig
            .binary_search_by_key(&lower_end, |&(lp, _)| lp)
            .ok()
            .map(|idx| lower_to_orig[idx].1);

        if let (Some(os), Some(oe)) = (orig_start, orig_end) {
            if os >= last_orig_end {
                result.push_str(&text[last_orig_end..os]);
                result.push_str(to);
                last_orig_end = oe;
            }
        }
    }

    result.push_str(&text[last_orig_end..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_substitution() {
        let rules = vec![SubstitutionRule {
            from: "btw".to_string(),
            to: "by the way".to_string(),
            case_sensitive: false,
        }];
        assert_eq!(
            apply_substitutions("btw, I wanted to say", &rules),
            "by the way, I wanted to say"
        );
    }

    #[test]
    fn test_case_insensitive() {
        let rules = vec![SubstitutionRule {
            from: "btw".to_string(),
            to: "by the way".to_string(),
            case_sensitive: false,
        }];
        assert_eq!(
            apply_substitutions("BTW, I wanted to say", &rules),
            "by the way, I wanted to say"
        );
    }

    #[test]
    fn test_case_sensitive() {
        let rules = vec![SubstitutionRule {
            from: "btw".to_string(),
            to: "by the way".to_string(),
            case_sensitive: true,
        }];
        assert_eq!(
            apply_substitutions("BTW stays, btw changes", &rules),
            "BTW stays, by the way changes"
        );
    }

    #[test]
    fn test_multiple_rules() {
        let rules = vec![
            SubstitutionRule {
                from: "js".to_string(),
                to: "JavaScript".to_string(),
                case_sensitive: true,
            },
            SubstitutionRule {
                from: "ts".to_string(),
                to: "TypeScript".to_string(),
                case_sensitive: true,
            },
        ];
        assert_eq!(
            apply_substitutions("I use js and ts", &rules),
            "I use JavaScript and TypeScript"
        );
    }

    #[test]
    fn test_no_rules() {
        let text = "nothing changes";
        assert_eq!(apply_substitutions(text, &[]), text);
    }

    #[test]
    fn test_unicode_case_insensitive() {
        // ẞ (U+1E9E, 3 bytes) lowercases to ß (U+00DF, 2 bytes)
        let rules = vec![SubstitutionRule {
            from: "straße".to_string(),
            to: "street".to_string(),
            case_sensitive: false,
        }];
        assert_eq!(
            apply_substitutions("Die STRAẞE ist lang", &rules),
            "Die street ist lang"
        );
    }

    #[test]
    fn test_no_match() {
        let rules = vec![SubstitutionRule {
            from: "xyz".to_string(),
            to: "abc".to_string(),
            case_sensitive: false,
        }];
        assert_eq!(
            apply_substitutions("hello world", &rules),
            "hello world"
        );
    }
}
