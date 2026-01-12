//! Conventional Commits parsing and emoji formatting.
//!
//! Parses commit messages following the Conventional Commits specification
//! and converts them to emoji-prefixed display format.
//!
//! See: <https://www.conventionalcommits.org/en/v1.0.0/>

/// Parsed conventional commit message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConventionalCommit<'a> {
    /// The commit type (e.g., "feat", "fix").
    pub commit_type: &'a str,
    /// Optional scope (e.g., "api" in "feat(api):").
    pub scope: Option<&'a str>,
    /// Whether this is a breaking change (has `!` suffix).
    pub breaking: bool,
    /// The description after the type prefix.
    pub description: &'a str,
}

impl<'a> ConventionalCommit<'a> {
    /// Parse a commit message into a ConventionalCommit.
    ///
    /// Returns `None` if the message doesn't follow the conventional format.
    ///
    /// # Format
    /// ```text
    /// <type>[optional scope][!]: <description>
    /// ```
    ///
    /// # Examples
    /// ```
    /// use xorcist::conventional::ConventionalCommit;
    ///
    /// let cc = ConventionalCommit::parse("feat: add new feature").unwrap();
    /// assert_eq!(cc.commit_type, "feat");
    /// assert_eq!(cc.description, "add new feature");
    ///
    /// let cc = ConventionalCommit::parse("fix(api)!: breaking fix").unwrap();
    /// assert_eq!(cc.commit_type, "fix");
    /// assert_eq!(cc.scope, Some("api"));
    /// assert!(cc.breaking);
    /// ```
    pub fn parse(message: &'a str) -> Option<Self> {
        // Quick reject: must contain ": "
        let colon_pos = message.find(": ")?;
        let prefix = &message[..colon_pos];
        let description = &message[colon_pos + 2..];

        // Parse prefix: type[(scope)][!]
        let (type_and_scope, breaking) = if let Some(stripped) = prefix.strip_suffix('!') {
            (stripped, true)
        } else {
            (prefix, false)
        };

        // Check for scope: type(scope)
        let (commit_type, scope) = if let Some(paren_start) = type_and_scope.find('(') {
            // Must end with ')'
            if !type_and_scope.ends_with(')') {
                return None;
            }
            let scope_content = &type_and_scope[paren_start + 1..type_and_scope.len() - 1];
            let commit_type = &type_and_scope[..paren_start];
            (commit_type, Some(scope_content))
        } else {
            (type_and_scope, None)
        };

        // Validate commit_type: must be lowercase alphanumeric
        if commit_type.is_empty() || !commit_type.chars().all(|c| c.is_ascii_lowercase()) {
            return None;
        }

        Some(ConventionalCommit {
            commit_type,
            scope,
            breaking,
            description,
        })
    }

    /// Get the emoji for this commit type.
    pub fn emoji(&self) -> &'static str {
        type_to_emoji(self.commit_type)
    }

    /// Format the commit as emoji display string.
    ///
    /// # Format
    /// - `feat: blah` → `✨ blah`
    /// - `fix!: hoge` → `🩹💥 hoge`
    /// - `fix(hoge): blah` → `🩹(hoge) blah`
    /// - `feat(api)!: xyz` → `✨(api)💥 xyz`
    pub fn to_display(&self) -> String {
        let emoji = self.emoji();
        let breaking_emoji = if self.breaking { "💥" } else { "" };

        match self.scope {
            Some(scope) => {
                format!("{emoji}({scope}){breaking_emoji} {}", self.description)
            }
            None => {
                format!("{emoji}{breaking_emoji} {}", self.description)
            }
        }
    }
}

/// Convert a conventional commit type to its corresponding emoji.
fn type_to_emoji(commit_type: &str) -> &'static str {
    match commit_type {
        "feat" => "✨",
        "fix" => "🩹",
        "docs" => "📝",
        "style" => "💄",
        "refactor" => "🏗️",
        "perf" => "⚡",
        "test" => "🧪",
        "build" => "📦",
        "ci" => "👷",
        "chore" => "🔧",
        "revert" => "⏪",
        // Additional common types
        "wip" => "🚧",
        "hotfix" => "🚑",
        "security" => "🔒",
        "deps" => "⬆️",
        "release" => "🔖",
        "init" => "🎉",
        // Fallback for unknown types
        _ => "📌",
    }
}

/// Format a commit message, converting conventional commits to emoji format.
///
/// If the message follows conventional commits format, it's converted.
/// Otherwise, the original message is returned unchanged.
pub fn format_commit_message(message: &str) -> String {
    ConventionalCommit::parse(message)
        .map(|cc| cc.to_display())
        .unwrap_or_else(|| message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let cc = ConventionalCommit::parse("feat: add new feature").unwrap();
        assert_eq!(cc.commit_type, "feat");
        assert_eq!(cc.scope, None);
        assert!(!cc.breaking);
        assert_eq!(cc.description, "add new feature");
    }

    #[test]
    fn test_parse_with_scope() {
        let cc = ConventionalCommit::parse("fix(api): handle null").unwrap();
        assert_eq!(cc.commit_type, "fix");
        assert_eq!(cc.scope, Some("api"));
        assert!(!cc.breaking);
        assert_eq!(cc.description, "handle null");
    }

    #[test]
    fn test_parse_breaking() {
        let cc = ConventionalCommit::parse("feat!: breaking change").unwrap();
        assert_eq!(cc.commit_type, "feat");
        assert!(cc.breaking);
        assert_eq!(cc.description, "breaking change");
    }

    #[test]
    fn test_parse_scope_and_breaking() {
        let cc = ConventionalCommit::parse("refactor(core)!: rewrite engine").unwrap();
        assert_eq!(cc.commit_type, "refactor");
        assert_eq!(cc.scope, Some("core"));
        assert!(cc.breaking);
        assert_eq!(cc.description, "rewrite engine");
    }

    #[test]
    fn test_parse_invalid() {
        // No colon
        assert!(ConventionalCommit::parse("just a message").is_none());
        // No space after colon
        assert!(ConventionalCommit::parse("feat:no space").is_none());
        // Empty type
        assert!(ConventionalCommit::parse(": no type").is_none());
        // Uppercase type
        assert!(ConventionalCommit::parse("FEAT: uppercase").is_none());
        // Unclosed scope
        assert!(ConventionalCommit::parse("feat(api: unclosed").is_none());
    }

    #[test]
    fn test_parse_no_description() {
        // This is technically valid but unusual
        assert!(ConventionalCommit::parse("(no description)").is_none());
    }

    #[test]
    fn test_to_display_simple() {
        let cc = ConventionalCommit::parse("feat: blah").unwrap();
        assert_eq!(cc.to_display(), "✨ blah");
    }

    #[test]
    fn test_to_display_breaking() {
        let cc = ConventionalCommit::parse("fix!: hoge").unwrap();
        assert_eq!(cc.to_display(), "🩹💥 hoge");
    }

    #[test]
    fn test_to_display_with_scope() {
        let cc = ConventionalCommit::parse("fix(hoge): blah").unwrap();
        assert_eq!(cc.to_display(), "🩹(hoge) blah");
    }

    #[test]
    fn test_to_display_scope_and_breaking() {
        let cc = ConventionalCommit::parse("feat(api)!: xyz").unwrap();
        assert_eq!(cc.to_display(), "✨(api)💥 xyz");
    }

    #[test]
    fn test_format_commit_message_conventional() {
        assert_eq!(format_commit_message("feat: new feature"), "✨ new feature");
        assert_eq!(format_commit_message("fix!: breaking"), "🩹💥 breaking");
        assert_eq!(
            format_commit_message("docs(readme): update"),
            "📝(readme) update"
        );
    }

    #[test]
    fn test_format_commit_message_non_conventional() {
        // Non-conventional messages pass through unchanged
        assert_eq!(
            format_commit_message("just a regular message"),
            "just a regular message"
        );
        assert_eq!(
            format_commit_message("(no description)"),
            "(no description)"
        );
        assert_eq!(format_commit_message("WIP stuff"), "WIP stuff");
    }

    #[test]
    fn test_emoji_mapping() {
        assert_eq!(type_to_emoji("feat"), "✨");
        assert_eq!(type_to_emoji("fix"), "🩹");
        assert_eq!(type_to_emoji("docs"), "📝");
        assert_eq!(type_to_emoji("style"), "💄");
        assert_eq!(type_to_emoji("refactor"), "🏗️");
        assert_eq!(type_to_emoji("perf"), "⚡");
        assert_eq!(type_to_emoji("test"), "🧪");
        assert_eq!(type_to_emoji("build"), "📦");
        assert_eq!(type_to_emoji("ci"), "👷");
        assert_eq!(type_to_emoji("chore"), "🔧");
        assert_eq!(type_to_emoji("revert"), "⏪");
        assert_eq!(type_to_emoji("unknown"), "📌"); // fallback
    }

    #[test]
    fn test_edge_cases() {
        // Japanese description
        let cc = ConventionalCommit::parse("feat: 日本語の説明").unwrap();
        assert_eq!(cc.to_display(), "✨ 日本語の説明");

        // Empty description (valid but unusual)
        let cc = ConventionalCommit::parse("fix: ").unwrap();
        assert_eq!(cc.to_display(), "🩹 ");

        // Multiple colons in description
        let cc = ConventionalCommit::parse("feat: time: 12:00").unwrap();
        assert_eq!(cc.to_display(), "✨ time: 12:00");

        // Scope with hyphen
        let cc = ConventionalCommit::parse("fix(my-module): issue").unwrap();
        assert_eq!(cc.scope, Some("my-module"));
    }
}
