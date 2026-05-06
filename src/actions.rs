//! Action metadata used by help/status UI and consistency tests.
//!
//! This module intentionally does not drive command execution yet. Key dispatch
//! remains in `keys.rs` until a fakeable jj backend seam exists.

/// Stable identifier for a user-visible action.
pub type ActionId = &'static str;

/// View or state scope where an action is meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewScope {
    Global,
    Log,
    Detail,
    Diff,
}

/// Event priority layer for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityLayer {
    Input,
    Modal,
    Help,
    ActiveView,
}

/// Static metadata for help/status display and tests.
#[derive(Debug, Clone, Copy)]
pub struct ActionSpec {
    pub id: ActionId,
    pub view_scope: ViewScope,
    pub priority_layer: PriorityLayer,
    pub key_label: &'static str,
    pub help: &'static str,
    pub requires_input: bool,
    pub requires_confirmation: bool,
    pub is_jj_mutating: bool,
}

impl ActionSpec {
    /// Validate metadata shape without implying execution behavior.
    pub fn has_valid_metadata(&self) -> bool {
        matches!(
            self.view_scope,
            ViewScope::Global | ViewScope::Log | ViewScope::Detail | ViewScope::Diff
        ) && matches!(
            self.priority_layer,
            PriorityLayer::Input
                | PriorityLayer::Modal
                | PriorityLayer::Help
                | PriorityLayer::ActiveView
        ) && (!self.requires_input || !self.requires_confirmation)
            && (!self.requires_confirmation || self.is_jj_mutating)
    }
}

pub const NAVIGATION_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        id: "log.move_down",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "j / ↓",
        help: "Move down",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "log.move_up",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "k / ↑",
        help: "Move up",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "log.go_top",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "g / Home",
        help: "Go to top",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "log.go_bottom",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "G / End",
        help: "Go to bottom",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "view.page_down",
        view_scope: ViewScope::Global,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "Ctrl+d / PgDn",
        help: "Page down",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "view.page_up",
        view_scope: ViewScope::Global,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "Ctrl+u / PgUp",
        help: "Page up",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
];

pub const JJ_COMMAND_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        id: "jj.new",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "n",
        help: "jj new <rev>",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: true,
    },
    ActionSpec {
        id: "jj.new_message",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::Input,
        key_label: "N",
        help: "jj new <rev> -m <msg>",
        requires_input: true,
        requires_confirmation: false,
        is_jj_mutating: true,
    },
    ActionSpec {
        id: "jj.edit",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "e",
        help: "jj edit <rev>",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: true,
    },
    ActionSpec {
        id: "jj.describe",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::Input,
        key_label: "d",
        help: "jj describe <rev> -m <msg>",
        requires_input: true,
        requires_confirmation: false,
        is_jj_mutating: true,
    },
    ActionSpec {
        id: "jj.bookmark_set",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::Input,
        key_label: "b",
        help: "jj bookmark set <name> -r <rev>",
        requires_input: true,
        requires_confirmation: false,
        is_jj_mutating: true,
    },
    ActionSpec {
        id: "jj.abandon",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::Modal,
        key_label: "a",
        help: "jj abandon <rev>",
        requires_input: false,
        requires_confirmation: true,
        is_jj_mutating: true,
    },
    ActionSpec {
        id: "jj.squash",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::Modal,
        key_label: "s",
        help: "jj squash -r <rev>",
        requires_input: false,
        requires_confirmation: true,
        is_jj_mutating: true,
    },
    ActionSpec {
        id: "jj.git_fetch",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "f",
        help: "jj git fetch",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "jj.git_push",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::Modal,
        key_label: "p",
        help: "jj git push",
        requires_input: false,
        requires_confirmation: true,
        is_jj_mutating: true,
    },
    ActionSpec {
        id: "jj.undo",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::Modal,
        key_label: "u",
        help: "jj undo",
        requires_input: false,
        requires_confirmation: true,
        is_jj_mutating: true,
    },
    ActionSpec {
        id: "jj.rebase",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::Input,
        key_label: "r",
        help: "jj rebase -r <rev> -d <dest>",
        requires_input: true,
        requires_confirmation: false,
        is_jj_mutating: true,
    },
];

pub const DETAIL_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        id: "detail.open_diff",
        view_scope: ViewScope::Detail,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "d",
        help: "View file diffs",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "detail.back",
        view_scope: ViewScope::Detail,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "q / Esc",
        help: "Back to log",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
];

pub const DIFF_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        id: "diff.next_file",
        view_scope: ViewScope::Diff,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "j / ↓",
        help: "Select next file",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "diff.previous_file",
        view_scope: ViewScope::Diff,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "k / ↑",
        help: "Select previous file",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "diff.scroll_vertical",
        view_scope: ViewScope::Diff,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "Ctrl+d/u / PgDn/PgUp",
        help: "Scroll diff vertically",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "diff.scroll_horizontal",
        view_scope: ViewScope::Diff,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "← / →",
        help: "Scroll diff horizontally",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "diff.back",
        view_scope: ViewScope::Diff,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "q / Esc",
        help: "Back to detail",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
];

pub const GENERAL_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        id: "log.open_detail",
        view_scope: ViewScope::Log,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "Enter",
        help: "Open selected revision details",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "view.quit_or_close",
        view_scope: ViewScope::Global,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "q",
        help: "Quit / close view (not help)",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "view.escape",
        view_scope: ViewScope::Global,
        priority_layer: PriorityLayer::ActiveView,
        key_label: "Esc",
        help: "Close view / close help",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
    ActionSpec {
        id: "help.toggle",
        view_scope: ViewScope::Global,
        priority_layer: PriorityLayer::Help,
        key_label: "?",
        help: "Open help / close help",
        requires_input: false,
        requires_confirmation: false,
        is_jj_mutating: false,
    },
];

pub fn action_by_id(id: ActionId) -> Option<&'static ActionSpec> {
    [
        NAVIGATION_ACTIONS,
        JJ_COMMAND_ACTIONS,
        DETAIL_ACTIONS,
        DIFF_ACTIONS,
        GENERAL_ACTIONS,
    ]
    .into_iter()
    .flatten()
    .find(|action| action.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_jj_action_metadata() {
        let describe = action_by_id("jj.describe").unwrap();
        assert_eq!(describe.view_scope, ViewScope::Log);
        assert_eq!(describe.priority_layer, PriorityLayer::Input);
        assert_eq!(describe.key_label, "d");
        assert!(describe.requires_input);
        assert!(!describe.requires_confirmation);
        assert!(describe.is_jj_mutating);

        let abandon = action_by_id("jj.abandon").unwrap();
        assert_eq!(abandon.priority_layer, PriorityLayer::Modal);
        assert_eq!(abandon.key_label, "a");
        assert!(abandon.requires_confirmation);
        assert!(abandon.is_jj_mutating);

        let fetch = action_by_id("jj.git_fetch").unwrap();
        assert_eq!(fetch.key_label, "f");
        assert!(!fetch.requires_input);
        assert!(!fetch.requires_confirmation);
        assert!(!fetch.is_jj_mutating);
    }

    #[test]
    fn test_same_key_can_have_view_scoped_meanings() {
        let describe = action_by_id("jj.describe").unwrap();
        let open_diff = action_by_id("detail.open_diff").unwrap();

        assert_eq!(describe.key_label, "d");
        assert_eq!(describe.view_scope, ViewScope::Log);
        assert_eq!(open_diff.key_label, "d");
        assert_eq!(open_diff.view_scope, ViewScope::Detail);
    }

    #[test]
    fn test_help_metadata_is_in_help_priority_layer() {
        let help = action_by_id("help.toggle").unwrap();
        assert_eq!(help.key_label, "?");
        assert_eq!(help.priority_layer, PriorityLayer::Help);
    }
}
