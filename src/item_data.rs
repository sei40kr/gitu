use std::{ops::Range, path::PathBuf, rc::Rc};

use crate::git::diff::Diff;

/// Graph visualization data for a commit
#[derive(Clone, Debug)]
pub(crate) struct GraphData {
    /// Columns that have lines passing through this row (the commit row)
    pub columns: Vec<GraphColumn>,
    /// Optional: columns for a post-commit connector row (e.g., merge lines)
    /// This row appears immediately after the commit, showing how branches connect
    pub post_commit_line: Option<Vec<GraphColumn>>,
    /// Maximum graph width across all commits (for alignment)
    pub max_width: usize,
}

/// Represents a column in the graph
#[derive(Clone, Debug)]
pub(crate) struct GraphColumn {
    /// Color index (used to cycle through graph_colors)
    pub color_index: usize,
    /// Type of character to draw in this column
    pub char_type: GraphCharType,
    /// Number of horizontal line segments to draw after this column (for connectors)
    pub horizontal_padding: usize,
}

/// Type of character to draw in a graph column
/// Based on Serie's EdgeType for consistent graph rendering
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum GraphCharType {
    /// Commit node
    Node,
    /// Vertical line (│)
    Vertical,
    /// Horizontal line (─)
    Horizontal,
    /// Right-Top corner (╮)
    RightTop,
    /// Vertical dashed line (┊) - used in merge commit connectors
    VerticalDashed,
    /// Left-Vertical junction (├)
    LeftVertical,
    /// Empty space
    Empty,
}

#[derive(Clone, Debug)]
pub(crate) enum ItemData {
    Raw(String),
    AllUnstaged(usize),
    AllStaged(usize),
    AllUntracked(Vec<PathBuf>),
    Reference {
        prefix: &'static str,
        kind: RefKind,
    },
    Commit {
        oid: String,
        short_id: String,
        associated_references: Vec<RefKind>,
        summary: String,
        parent_ids: Vec<String>,
        graph_data: Option<GraphData>,
    },
    /// A graph-only line showing branch connections (no commit data)
    /// Used to display merge/branch lines between commits
    GraphLine {
        columns: Vec<GraphColumn>,
    },
    Untracked(PathBuf),
    Delta {
        diff: Rc<Diff>,
        file_i: usize,
    },
    Hunk {
        diff: Rc<Diff>,
        file_i: usize,
        hunk_i: usize,
    },
    HunkLine {
        diff: Rc<Diff>,
        file_i: usize,
        hunk_i: usize,
        line_i: usize,
        line_range: Range<usize>,
    },
    Stash {
        message: String,
        stash_ref: String,
        id: usize,
    },
    Header(SectionHeader),
    BranchStatus(String, u32, u32),
    Error(String),
}

impl ItemData {
    pub(crate) fn is_section(&self) -> bool {
        matches!(
            self,
            ItemData::AllUnstaged(_)
                | ItemData::AllStaged(_)
                | ItemData::AllUntracked(_)
                | ItemData::Untracked(_)
                | ItemData::Delta { .. }
                | ItemData::Hunk { .. }
                | ItemData::Header(_)
                | ItemData::BranchStatus(_, _, _)
        )
    }
}

impl Default for ItemData {
    fn default() -> Self {
        ItemData::Raw(String::new())
    }
}

#[derive(Clone, Debug)]
pub(crate) enum RefKind {
    Tag(String),
    Branch(String),
    Remote(String),
}

#[derive(Clone, Debug)]
pub(crate) enum SectionHeader {
    Remote(String),
    Tags,
    Branches,
    NoBranch,
    OnBranch(String),
    Rebase(String, String),
    Merge(String),
    Revert(String),
    Stashes,
    RecentCommits,
    Commit(String),
}
