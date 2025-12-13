use std::collections::HashMap;

use crate::item_data::{GraphCharType, GraphColumn, GraphData, ItemData};
use crate::items::Item;

/// Compute graph layout for a list of commit items
pub(crate) fn compute_graph_layout(items: &mut [Item]) {
    let mut layout = GraphLayout::new();

    // First pass: compute graph data
    for item in items.iter_mut() {
        if let ItemData::Commit {
            oid,
            parent_ids,
            graph_data,
            ..
        } = &mut item.data
        {
            let graph = layout.process_commit(oid, parent_ids);
            *graph_data = Some(graph);
        }
    }

    // Second pass: calculate max width and update all graph data
    let max_width = calculate_max_graph_width(items);
    for item in items.iter_mut() {
        if let ItemData::Commit { graph_data, .. } = &mut item.data
            && let Some(graph) = graph_data
        {
            graph.max_width = max_width;
        }
    }
}

/// Calculate the maximum graph width (in characters) across all commit items
/// This simulates the actual rendering to get accurate width
fn calculate_max_graph_width(items: &[Item]) -> usize {
    items
        .iter()
        .filter_map(|item| {
            match &item.data {
                ItemData::Commit { graph_data, .. } => {
                    graph_data.as_ref().map(|g| {
                        // Simulate the actual rendering:
                        // For each column: 1 char + horizontal_padding chars + (0 or 1 space)
                        g.columns
                            .iter()
                            .map(|col| {
                                // Character itself (1 char)
                                let mut width = 1;

                                // Add horizontal padding width
                                if col.horizontal_padding > 0 {
                                    width += col.horizontal_padding;
                                    // No space after horizontal padding
                                } else {
                                    // Add space after character (except for Horizontal, VerticalDashed, and Empty)
                                    if !matches!(
                                        col.char_type,
                                        GraphCharType::Horizontal
                                            | GraphCharType::VerticalDashed
                                            | GraphCharType::Empty
                                    ) {
                                        width += 1;
                                    }
                                }

                                width
                            })
                            .sum::<usize>()
                    })
                }
                _ => None, // Only count Commit items, not GraphLine
            }
        })
        .max()
        .unwrap_or(0)
}

/// Insert GraphLine items after commits that have post_commit_line data
pub(crate) fn insert_graph_lines(items: Vec<Item>) -> Vec<Item> {
    let mut result = Vec::new();

    for item in items {
        let has_post_line = if let ItemData::Commit { graph_data, .. } = &item.data {
            graph_data
                .as_ref()
                .and_then(|g| g.post_commit_line.as_ref())
                .is_some()
        } else {
            false
        };

        result.push(item.clone());

        if has_post_line
            && let ItemData::Commit { graph_data, .. } = &item.data
            && let Some(post_line) = graph_data
                .as_ref()
                .and_then(|g| g.post_commit_line.as_ref())
        {
            result.push(Item {
                id: item.id + 1, // Simple ID generation
                depth: item.depth,
                data: ItemData::GraphLine {
                    columns: post_line.clone(),
                },
                unselectable: true,
                default_collapsed: false,
            });
        }
    }

    result
}

/// Tracks which commit OID is expected in each column
#[derive(Debug, Clone)]
struct ColumnState {
    /// The OID expected to appear in this column
    oid: String,
    /// Color index for this flow
    color: usize,
}

struct GraphLayout {
    /// Current column states (active flows)
    columns: Vec<Option<ColumnState>>,
    /// Next color to assign
    next_color: usize,
    /// Map from OID to assigned column (for commits we've already placed)
    oid_to_column: HashMap<String, usize>,
}

impl GraphLayout {
    fn new() -> Self {
        Self {
            columns: Vec::new(),
            next_color: 0,
            oid_to_column: HashMap::new(),
        }
    }

    fn process_commit(&mut self, oid: &str, parent_ids: &[String]) -> GraphData {
        // Find which column this commit should be in
        let commit_col = self.find_commit_column(oid);
        let commit_color = self.columns[commit_col].as_ref().unwrap().color;

        // Build the graph columns for this commit row
        let mut graph_columns = Vec::new();

        for (col_idx, slot) in self.columns.iter().enumerate() {
            if col_idx == commit_col {
                // This is the commit node
                graph_columns.push(GraphColumn {
                    color_index: commit_color,
                    char_type: GraphCharType::Node,
                    horizontal_padding: 0,
                });
            } else if let Some(state) = slot {
                // Active column - draw vertical line
                graph_columns.push(GraphColumn {
                    color_index: state.color,
                    char_type: GraphCharType::Vertical,
                    horizontal_padding: 0,
                });
            }
            // Skip empty columns - they don't render anything
        }

        // Update columns for the next row FIRST (to know where parents will be)
        let parent_columns = self.preview_parent_columns(commit_col, parent_ids);

        // For merge commits (multiple parents), generate a post-commit line
        let post_commit_line = if parent_ids.len() > 1 {
            Some(self.generate_post_commit_line_with_parents(commit_col, &parent_columns))
        } else {
            None
        };

        // Now actually update columns
        self.update_columns_with_parents(commit_col, parent_ids);

        GraphData {
            columns: graph_columns,
            post_commit_line,
            max_width: 0, // Will be set later in second pass
        }
    }

    /// Preview where parents will be placed (without actually updating columns)
    fn preview_parent_columns(&self, commit_col: usize, parent_ids: &[String]) -> Vec<usize> {
        let mut parent_cols = Vec::new();

        if parent_ids.is_empty() {
            return parent_cols;
        }

        // First parent goes in commit_col
        parent_cols.push(commit_col);

        // Additional parents need new columns
        for parent_id in &parent_ids[1..] {
            // Check if already in a column
            let existing = self
                .columns
                .iter()
                .enumerate()
                .find(|(_, slot)| slot.as_ref().is_some_and(|s| &s.oid == parent_id))
                .map(|(idx, _)| idx);

            if let Some(col) = existing {
                parent_cols.push(col);
            } else {
                // Find first empty column
                let empty_col = self
                    .columns
                    .iter()
                    .enumerate()
                    .find(|(_, slot)| slot.is_none())
                    .map(|(idx, _)| idx)
                    .unwrap_or(self.columns.len());
                parent_cols.push(empty_col);
            }
        }

        parent_cols
    }

    /// Generate a post-commit line knowing where parents will be
    /// Creates a line like: `├─╮` or `├─┊─╮` to show merge connections
    fn generate_post_commit_line_with_parents(
        &self,
        commit_col: usize,
        parent_cols: &[usize],
    ) -> Vec<GraphColumn> {
        let mut line_columns = Vec::new();
        let commit_color = self.columns[commit_col].as_ref().unwrap().color;

        // Find the rightmost parent column (for merge visualization)
        let rightmost_parent = parent_cols
            .iter()
            .filter(|&&c| c != commit_col)
            .max()
            .copied();

        // Determine the maximum column we need to render
        // Must include all parent columns and existing columns
        let max_col = parent_cols
            .iter()
            .max()
            .copied()
            .unwrap_or(commit_col)
            .max(self.columns.len().saturating_sub(1));

        for col_idx in 0..=max_col {
            let char_type = if col_idx == commit_col {
                // This is where the merge commit was
                if parent_cols.len() > 1 && rightmost_parent.is_some() {
                    // Merge: use ├ to connect down and to right
                    GraphCharType::LeftVertical // ├
                } else {
                    GraphCharType::Vertical // │ (single parent, just continue)
                }
            } else if col_idx > commit_col
                && rightmost_parent.is_some()
                && col_idx < rightmost_parent.unwrap()
            {
                // Between commit and rightmost parent - draw horizontal line or dashed vertical
                if self.columns.get(col_idx).and_then(|s| s.as_ref()).is_some() {
                    // Active column in the way - use dashed vertical ┊
                    GraphCharType::VerticalDashed
                } else {
                    // Empty space - use horizontal line ─
                    GraphCharType::Horizontal
                }
            } else if Some(col_idx) == rightmost_parent {
                // Rightmost parent - use ╮ to turn down
                GraphCharType::RightTop // ╮
            } else if col_idx < self.columns.len() && self.columns[col_idx].is_some() {
                // Active column, just pass through
                GraphCharType::Vertical
            } else {
                GraphCharType::Empty
            };

            let color_index = if col_idx < self.columns.len() {
                self.columns[col_idx]
                    .as_ref()
                    .map(|s| s.color)
                    .unwrap_or(commit_color)
            } else {
                commit_color
            };

            // Calculate horizontal padding: how many horizontal lines to draw after this column
            // For connector lines, we need horizontal lines between each column from ├ to ╮
            let horizontal_padding = if let Some(rp) = rightmost_parent {
                if col_idx >= commit_col && col_idx < rp {
                    // Draw one horizontal line after each column between ├ and ╮
                    1
                } else {
                    0
                }
            } else {
                0
            };

            line_columns.push(GraphColumn {
                color_index,
                char_type,
                horizontal_padding,
            });
        }

        line_columns
    }

    /// Find or assign a column for the given commit
    fn find_commit_column(&mut self, oid: &str) -> usize {
        // Check if this commit is already expected in a column
        for (col_idx, slot) in self.columns.iter().enumerate() {
            if let Some(state) = slot
                && state.oid == oid
            {
                return col_idx;
            }
        }

        // This commit wasn't expected - find an empty column or add one
        for (col_idx, slot) in self.columns.iter_mut().enumerate() {
            if slot.is_none() {
                let color = self.next_color;
                self.next_color += 1;
                *slot = Some(ColumnState {
                    oid: oid.to_string(),
                    color,
                });
                self.oid_to_column.insert(oid.to_string(), col_idx);
                return col_idx;
            }
        }

        // No empty column - add a new one
        let col_idx = self.columns.len();
        let color = self.next_color;
        self.next_color += 1;
        self.columns.push(Some(ColumnState {
            oid: oid.to_string(),
            color,
        }));
        self.oid_to_column.insert(oid.to_string(), col_idx);
        col_idx
    }

    /// Update columns: remove the commit from commit_col and add its parents
    fn update_columns_with_parents(&mut self, commit_col: usize, parent_ids: &[String]) {
        if parent_ids.is_empty() {
            // No parents - clear this column
            self.columns[commit_col] = None;
            return;
        }

        let commit_color = self.columns[commit_col].as_ref().unwrap().color;

        // First parent continues in the same column (same color/flow)
        self.columns[commit_col] = Some(ColumnState {
            oid: parent_ids[0].clone(),
            color: commit_color,
        });
        self.oid_to_column.insert(parent_ids[0].clone(), commit_col);

        // Additional parents (for merge commits) need new columns
        for parent_id in &parent_ids[1..] {
            // Skip if this parent is already in a column
            if self.oid_to_column.contains_key(parent_id) {
                continue;
            }

            // Try to find an empty column
            let mut placed = false;
            for (col_idx, slot) in self.columns.iter_mut().enumerate() {
                if slot.is_none() {
                    let color = self.next_color;
                    self.next_color += 1;
                    *slot = Some(ColumnState {
                        oid: parent_id.clone(),
                        color,
                    });
                    self.oid_to_column.insert(parent_id.clone(), col_idx);
                    placed = true;
                    break;
                }
            }

            if !placed {
                // Add a new column
                let color = self.next_color;
                self.next_color += 1;
                let col_idx = self.columns.len();
                self.columns.push(Some(ColumnState {
                    oid: parent_id.clone(),
                    color,
                }));
                self.oid_to_column.insert(parent_id.clone(), col_idx);
            }
        }

        // Compact: remove trailing empty columns
        while self.columns.last().is_some_and(|s| s.is_none()) {
            self.columns.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_history() {
        let mut items = vec![
            create_commit("c3", vec!["c2"]),
            create_commit("c2", vec!["c1"]),
            create_commit("c1", vec![]),
        ];

        compute_graph_layout(&mut items);

        // All commits should have graph data
        for item in &items {
            if let ItemData::Commit { graph_data, .. } = &item.data {
                assert!(graph_data.is_some());
            }
        }
    }

    #[test]
    fn test_merge_commit() {
        let mut items = vec![
            create_commit("m1", vec!["c2", "c3"]), // Merge commit
            create_commit("c2", vec!["c1"]),
            create_commit("c3", vec!["c1"]),
            create_commit("c1", vec![]),
        ];

        compute_graph_layout(&mut items);

        // Verify merge commit has graph data with post_commit_line
        if let ItemData::Commit {
            graph_data,
            parent_ids,
            ..
        } = &items[0].data
        {
            assert!(graph_data.is_some());
            let graph = graph_data.as_ref().unwrap();
            // Merge commits should have post_commit_line
            assert_eq!(parent_ids.len(), 2);
            assert!(graph.post_commit_line.is_some());
        }
    }

    fn create_commit(oid: &str, parents: Vec<&str>) -> Item {
        Item {
            data: ItemData::Commit {
                oid: oid.to_string(),
                short_id: oid[..2.min(oid.len())].to_string(),
                associated_references: vec![],
                summary: format!("Commit {}", oid),
                parent_ids: parents.iter().map(|s| s.to_string()).collect(),
                graph_data: None,
            },
            ..Default::default()
        }
    }
}
