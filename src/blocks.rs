//! Block model: every unit of content is a block with a stable ID, a type,
//! typed content, and an ordered position inside its parent (page or block).

/// Block types supported by the editor in the first release.
pub const TYPES: &[&str] = &[
    "paragraph", "heading_1", "heading_2", "heading_3",
    "bulleted_list", "numbered_list", "to_do", "toggle",
    "quote", "code", "divider", "callout", "image",
];

/// Validate that a block type string is known.
pub fn is_valid_type(t: &str) -> bool {
    TYPES.contains(&t)
}

/// Compute a fractional order key between two neighbors (reordering without
/// rewriting every sibling's ord).
pub fn order_between(prev: Option<f64>, next: Option<f64>) -> f64 {
    match (prev, next) {
        (Some(p), Some(n)) => (p + n) / 2.0,
        (Some(p), None) => p + 1.0,
        (None, Some(n)) => n / 2.0,
        (None, None) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_types() {
        assert!(is_valid_type("paragraph"));
        assert!(!is_valid_type("definitely_not_a_type"));
    }

    #[test]
    fn ordering() {
        assert_eq!(order_between(Some(1.0), Some(2.0)), 1.5);
        assert_eq!(order_between(Some(1.0), None), 2.0);
        assert_eq!(order_between(None, Some(2.0)), 1.0);
        assert_eq!(order_between(None, None), 0.0);
    }
}
