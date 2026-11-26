#![forbid(unsafe_code)]

/// Stable project identity for headless tooling and smoke checks.
pub const PROJECT_NAME: &str = "Kings of Dawn";

#[cfg(test)]
mod tests {
    use super::PROJECT_NAME;

    #[test]
    fn project_identity_is_available_headless() {
        assert_eq!(PROJECT_NAME, "Kings of Dawn");
    }
}
