use serde::{Deserialize, Serialize};

/// GlobPattern is a minimal prefix/star matcher for namespaces.
///
/// Supported syntax:
/// * `foo/*`        → matches `foo/anything`
/// * `foo/**`       → matches `foo/anything/deep`
/// * `foo/bar`      → exact literal
/// * plain value    → treated as prefix, with `/` as separator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobPattern(String);

impl GlobPattern {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn matches(&self, value: &str) -> bool {
        match_pattern(&self.0, value)
    }
}

fn match_pattern(pattern: &str, value: &str) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }
    if pattern == "*" || pattern == "**" {
        return true;
    }

    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let value_parts: Vec<&str> = value.split('/').collect();

    match_parts(&pattern_parts, &value_parts)
}

fn match_parts(pattern: &[&str], value: &[&str]) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }

    let head = pattern[0];
    let rest = &pattern[1..];

    if head == "**" {
        if rest.is_empty() {
            return true;
        }
        for i in 0..=value.len() {
            if match_parts(rest, &value[i..]) {
                return true;
            }
        }
        return false;
    }

    if value.is_empty() {
        return false;
    }

    if head == "*" || head == value[0] {
        return match_parts(rest, &value[1..]);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::GlobPattern;

    #[test]
    fn star_matches_single_segment() {
        let pattern = GlobPattern::new("user/*");
        assert!(pattern.matches("user/profile"));
        assert!(!pattern.matches("user/profile/deep"));
        assert!(!pattern.matches("agents/coder"));
    }

    #[test]
    fn double_star_matches_any_depth() {
        let pattern = GlobPattern::new("agents/**");
        assert!(pattern.matches("agents/coder"));
        assert!(pattern.matches("agents/coder/skills"));
    }

    #[test]
    fn literal_matches_exact() {
        let pattern = GlobPattern::new("interaction/protocol");
        assert!(pattern.matches("interaction/protocol"));
        assert!(!pattern.matches("interaction/protocol/v2"));
    }
}
