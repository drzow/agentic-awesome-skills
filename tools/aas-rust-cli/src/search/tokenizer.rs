use std::collections::HashSet;

/// Tokenize text into lowercase words, removing non-alphanumeric characters.
pub fn tokenize(text: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    for token in split_tokens(text) {
        if token.len() >= 2 {
            tokens.insert(token);
        }
    }
    tokens
}

/// Split text into tokens, handling hyphens, camelCase, and underscores.
fn split_tokens(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::with_capacity(32);

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '\'' {
            current.push(ch);
        } else {
            if !current.is_empty() {
                result.extend(split_token_word(&current));
                current.clear();
            }
            if ch == '-' || ch == '_' {
                if !current.is_empty() {
                    result.extend(split_token_word(&current));
                    current.clear();
                }
            }
        }
    }

    if !current.is_empty() {
        result.extend(split_token_word(&current));
    }

    result
}

/// Split a single word that might contain camelCase.
fn split_token_word(word: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();

    if len == 0 {
        return parts;
    }

    let mut current = String::with_capacity(32);
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_uppercase() && chars[i - 1].is_lowercase() {
            parts.push(current.to_lowercase());
            current.clear();
        }
        current.push(ch.to_ascii_lowercase());
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

/// Tokenize a search query into individual terms.
pub fn tokenize_query(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for word in query.split_whitespace() {
        for t in split_tokens(word) {
            if !tokens.contains(&t) {
                tokens.push(t);
            }
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("hello world");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
    }

    #[test]
    fn test_tokenize_camelcase() {
        let tokens = tokenize("myAwesomeSkill");
        assert!(tokens.contains(&"my".to_string()));
        assert!(tokens.contains(&"awesome".to_string()));
        assert!(tokens.contains(&"skill".to_string()));
    }

    #[test]
    fn test_tokenize_query() {
        let tokens = tokenize_query("security scanning CI pipeline");
        assert!(tokens.contains(&"security".to_string()));
        assert!(tokens.contains(&"scanning".to_string()));
        assert!(tokens.contains(&"ci".to_string()));
        assert!(tokens.contains(&"pipeline".to_string()));
    }

    #[test]
    fn test_tokenize_single_char_ignored() {
        let tokens = tokenize("a b c");
        for t in &tokens {
            assert!(t.len() >= 2, "token '{}' is too short", t);
        }
    }
}
