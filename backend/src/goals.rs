//! Goal definitions: validation shared by the REST API and the MCP tools.

use crate::models::GoalInput;

/// Longest custom event name the browser tracker sends.
pub const MAX_EVENT_NAME: usize = 120;
const MAX_GOAL_NAME: usize = 120;
const MAX_PATH_PATTERN: usize = 512;

/// A goal definition after trimming and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalDefinition {
    pub name: String,
    pub event_name: String,
    pub path_pattern: Option<String>,
}

/// Validates a goal so it can match events the collectors accept.
///
/// The event name is matched exactly, so surrounding whitespace is rejected rather than
/// trimmed: a goal for `" signup"` could never match the tracker's `signup` event.
/// `pathPattern` is a SQL `LIKE` pattern against the event's path (`%` matches any
/// characters, `_` one character); an empty pattern means any path.
pub fn validate_goal(input: GoalInput) -> Result<GoalDefinition, &'static str> {
    let name = input.name.trim().to_owned();
    if name.is_empty() || name.chars().count() > MAX_GOAL_NAME {
        return Err("name must be 1-120 characters");
    }
    let event_name = input.event_name;
    if event_name.is_empty()
        || event_name.chars().count() > MAX_EVENT_NAME
        || event_name.trim() != event_name
        || event_name.chars().any(char::is_control)
    {
        return Err("eventName must be 1-120 characters without surrounding whitespace");
    }
    let path_pattern = input
        .path_pattern
        .map(|pattern| pattern.trim().to_owned())
        .filter(|pattern| !pattern.is_empty());
    if let Some(pattern) = &path_pattern {
        if pattern.len() > MAX_PATH_PATTERN
            || !(pattern.starts_with('/') || pattern.starts_with('%'))
            || pattern.chars().any(char::is_control)
            // A backslash escapes LIKE wildcards, and a trailing one is a database error that
            // would fail ingestion of every matching event. Paths do not contain backslashes.
            || pattern.contains('\\')
        {
            return Err(
                "pathPattern must start with / or %, have no backslashes, and be at most 512 characters",
            );
        }
    }
    Ok(GoalDefinition {
        name,
        event_name,
        path_pattern,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, event: &str, path: Option<&str>) -> GoalInput {
        GoalInput {
            name: name.into(),
            event_name: event.into(),
            path_pattern: path.map(str::to_owned),
        }
    }

    #[test]
    fn accepts_event_and_page_goals() {
        assert_eq!(
            validate_goal(input(" Signup ", "signup", None)).unwrap(),
            GoalDefinition {
                name: "Signup".into(),
                event_name: "signup".into(),
                path_pattern: None
            }
        );
        let page = validate_goal(input("Welcome", "pageview", Some(" /welcome% "))).unwrap();
        assert_eq!(page.path_pattern.as_deref(), Some("/welcome%"));
        assert_eq!(
            validate_goal(input("Any", "signup", Some("  ")))
                .unwrap()
                .path_pattern,
            None
        );
        assert!(validate_goal(input("Spaces", "Sign up", None)).is_ok());
    }

    #[test]
    fn rejects_goals_that_could_never_match() {
        assert!(validate_goal(input("", "signup", None)).is_err());
        assert!(validate_goal(input("Signup", "", None)).is_err());
        assert!(validate_goal(input("Signup", " signup", None)).is_err());
        assert!(validate_goal(input("Signup", "sign\nup", None)).is_err());
        assert!(validate_goal(input("Signup", &"x".repeat(121), None)).is_err());
        assert!(validate_goal(input("Signup", "signup", Some("welcome"))).is_err());
        assert!(validate_goal(input("Signup", "signup", Some("/signup\\"))).is_err());
        assert!(validate_goal(input("Signup", "signup", Some("/a\\%b"))).is_err());
        assert!(validate_goal(input(
            "Signup",
            "signup",
            Some(&format!("/{}", "a".repeat(512)))
        ))
        .is_err());
    }
}
