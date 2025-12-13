use crate::Res;
use crate::config::Config;
use crate::error::Error;
use crate::git::diff::Diff;
use crate::gitu_diff::Status;
use crate::highlight;
use crate::item_data::ItemData;
use crate::item_data::RefKind;
use crate::item_data::SectionHeader;
use git2::Oid;
use git2::Repository;
use ratatui::text::Line;
use ratatui::text::Span;
use regex::Regex;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::iter;
use std::rc::Rc;
use std::sync::Arc;

pub type ItemId = u64;

#[derive(Default, Clone, Debug)]
pub(crate) struct Item {
    pub(crate) id: ItemId,
    pub(crate) default_collapsed: bool,
    pub(crate) depth: usize,
    pub(crate) unselectable: bool,
    pub(crate) data: ItemData,
}

impl Item {
    pub fn to_line(&'_ self, config: Arc<Config>) -> Line<'_> {
        match self.data.clone() {
            ItemData::Raw(content) => Line::raw(content),
            ItemData::AllUnstaged(count) => Line::from(vec![
                Span::styled("Unstaged changes", &config.style.section_header),
                Span::raw(format!(" ({count})")),
            ]),
            ItemData::AllStaged(count) => Line::from(vec![
                Span::styled("Staged changes", &config.style.section_header),
                Span::raw(format!(" ({count})")),
            ]),
            ItemData::AllUntracked(_) => {
                Line::styled("Untracked files", &config.style.section_header)
            }
            ItemData::Reference { kind, prefix } => {
                let (reference, style) = match kind {
                    RefKind::Tag(tag) => (tag, &config.style.tag),
                    RefKind::Branch(branch) => (branch, &config.style.branch),
                    RefKind::Remote(remote) => (remote, &config.style.remote),
                };

                Line::from(vec![Span::raw(prefix), Span::styled(reference, style)])
            }
            ItemData::Commit {
                short_id,
                associated_references,
                summary,
                graph_data,
                ..
            } => {
                let mut spans = Vec::new();
                let has_graph = graph_data.is_some();
                // Always use fixed width of 13 chars for 7 branches (7 chars + 6 spaces)
                const GRAPH_WIDTH: usize = 13;

                // Track where graph spans end (before adding commit info)
                let graph_span_start = spans.len();
                let mut commit_branch_color = None;

                // Add graph visualization if available
                if let Some(graph) = graph_data {
                    use crate::item_data::GraphCharType;

                    let is_ascii = matches!(
                        config.general.log_graph_style,
                        crate::config::LogGraphStyle::Ascii
                    );

                    // Find the commit node's color (for separator)
                    for col in &graph.columns {
                        if col.char_type == GraphCharType::Node {
                            commit_branch_color = Some(col.color_index);
                            break;
                        }
                    }

                    for col in &graph.columns {
                        let color = if !config.style.graph_colors.is_empty() {
                            config.style.graph_colors
                                [col.color_index % config.style.graph_colors.len()]
                        } else {
                            ratatui::style::Color::Blue
                        };

                        let ch = if is_ascii {
                            match col.char_type {
                                GraphCharType::Node => '*',
                                GraphCharType::Vertical => '|',
                                GraphCharType::Horizontal => '-',
                                GraphCharType::RightTop => '\\',
                                GraphCharType::VerticalDashed => ':',
                                GraphCharType::LeftVertical => '|',
                                GraphCharType::Empty => ' ',
                            }
                        } else {
                            match col.char_type {
                                GraphCharType::Node => '•',
                                GraphCharType::Vertical => '│',
                                GraphCharType::Horizontal => '─',
                                GraphCharType::RightTop => '╮',
                                GraphCharType::VerticalDashed => '┊',
                                GraphCharType::LeftVertical => '├',
                                GraphCharType::Empty => ' ',
                            }
                        };

                        spans.push(Span::styled(
                            ch.to_string(),
                            ratatui::style::Style::default().fg(color),
                        ));
                        // Add horizontal padding if specified (for connector lines like ├─╮)
                        if col.horizontal_padding > 0 {
                            let horiz_ch = if is_ascii { '-' } else { '─' };
                            spans.push(Span::styled(
                                horiz_ch.to_string().repeat(col.horizontal_padding),
                                ratatui::style::Style::default().fg(color),
                            ));
                        } else {
                            // Add space after characters except horizontal, vertical dashed, and empty
                            if !matches!(
                                col.char_type,
                                GraphCharType::Horizontal
                                    | GraphCharType::VerticalDashed
                                    | GraphCharType::Empty
                            ) {
                                spans.push(Span::raw(" "));
                            }
                        }
                    }
                }

                // Pad to fixed width for alignment if graph is present
                if has_graph {
                    // Calculate actual rendered width in characters (not bytes)
                    let graph_len: usize = spans[graph_span_start..]
                        .iter()
                        .map(|s| s.content.chars().count())
                        .sum();
                    if graph_len < GRAPH_WIDTH {
                        spans.push(Span::raw(" ".repeat(GRAPH_WIDTH - graph_len)));
                    }

                    // Add separator ` ┃ ` with commit branch color
                    let separator_color = if let Some(color_index) = commit_branch_color {
                        if !config.style.graph_colors.is_empty() {
                            config.style.graph_colors[color_index % config.style.graph_colors.len()]
                        } else {
                            ratatui::style::Color::Blue
                        }
                    } else {
                        ratatui::style::Color::Blue
                    };

                    let is_ascii = matches!(
                        config.general.log_graph_style,
                        crate::config::LogGraphStyle::Ascii
                    );
                    let separator = if is_ascii { " | " } else { " ┃ " };
                    spans.push(Span::styled(
                        separator,
                        ratatui::style::Style::default().fg(separator_color),
                    ));
                }

                // Add commit info
                spans.extend(itertools::intersperse(
                    iter::once(Span::styled(short_id, &config.style.hash))
                        .chain(
                            associated_references
                                .into_iter()
                                .map(|reference| match reference {
                                    RefKind::Tag(tag) => Span::styled(tag, &config.style.tag),
                                    RefKind::Branch(branch) => {
                                        Span::styled(branch, &config.style.branch)
                                    }
                                    RefKind::Remote(remote) => {
                                        Span::styled(remote, &config.style.remote)
                                    }
                                }),
                        )
                        .chain([Span::raw(summary)]),
                    Span::raw(" "),
                ));

                Line::from(spans)
            }
            ItemData::GraphLine { columns } => {
                use crate::item_data::GraphCharType;

                let is_ascii = matches!(
                    config.general.log_graph_style,
                    crate::config::LogGraphStyle::Ascii
                );
                let mut spans = Vec::new();

                for col in columns {
                    let color = if !config.style.graph_colors.is_empty() {
                        config.style.graph_colors[col.color_index % config.style.graph_colors.len()]
                    } else {
                        ratatui::style::Color::Blue
                    };

                    let ch = if is_ascii {
                        match col.char_type {
                            GraphCharType::Node => '*',
                            GraphCharType::Vertical => '|',
                            GraphCharType::Horizontal => '-',
                            GraphCharType::RightTop => '\\',
                            GraphCharType::VerticalDashed => ':',
                            GraphCharType::LeftVertical => '|',
                            GraphCharType::Empty => ' ',
                        }
                    } else {
                        match col.char_type {
                            GraphCharType::Node => '•',
                            GraphCharType::Vertical => '│',
                            GraphCharType::Horizontal => '─',
                            GraphCharType::RightTop => '╮',
                            GraphCharType::VerticalDashed => '┊',
                            GraphCharType::LeftVertical => '├',
                            GraphCharType::Empty => ' ',
                        }
                    };

                    spans.push(Span::styled(
                        ch.to_string(),
                        ratatui::style::Style::default().fg(color),
                    ));
                    // Add horizontal padding if specified (for connector lines like ├─╮)
                    if col.horizontal_padding > 0 {
                        let horiz_ch = if is_ascii { '-' } else { '─' };
                        spans.push(Span::styled(
                            horiz_ch.to_string().repeat(col.horizontal_padding),
                            ratatui::style::Style::default().fg(color),
                        ));
                    } else {
                        // Add space after characters except horizontal, vertical dashed, and empty
                        if !matches!(
                            col.char_type,
                            GraphCharType::Horizontal
                                | GraphCharType::VerticalDashed
                                | GraphCharType::Empty
                        ) {
                            spans.push(Span::raw(" "));
                        }
                    }
                }

                Line::from(spans)
            }
            ItemData::Untracked(path) => Line::styled(
                path.to_string_lossy().into_owned(),
                &config.style.file_header,
            ),
            ItemData::Delta { diff, file_i } => {
                let file_diff = &diff.file_diffs[file_i];

                let content = format!(
                    "{:8}   {}",
                    format!("{:?}", file_diff.header.status).to_lowercase(),
                    match file_diff.header.status {
                        Status::Renamed | Status::Copied => format!(
                            "{} -> {}",
                            &file_diff.header.old_file.fmt(&diff.text),
                            &file_diff.header.new_file.fmt(&diff.text)
                        ),
                        Status::Deleted => file_diff.header.old_file.fmt(&diff.text).to_string(),
                        Status::Added => file_diff.header.new_file.fmt(&diff.text).to_string(),
                        Status::Modified => file_diff.header.new_file.fmt(&diff.text).to_string(),
                        Status::Unmerged => file_diff.header.new_file.fmt(&diff.text).to_string(),
                    }
                );

                Line::styled(content, &config.style.file_header)
            }
            ItemData::Hunk {
                diff,
                file_i,
                hunk_i,
            } => {
                let file_diff = &diff.file_diffs[file_i];
                let hunk = &file_diff.hunks[hunk_i];
                let content = &diff.text[hunk.header.range.clone()];

                Line::styled(content.to_string(), &config.style.hunk_header)
            }
            ItemData::HunkLine {
                diff,
                file_i,
                hunk_i,
                line_range,
                line_i,
            } => {
                let hunk_highlights =
                    highlight::highlight_hunk(self.id, &config, &Rc::clone(&diff), file_i, hunk_i);

                let hunk_content = &diff.hunk_content(file_i, hunk_i);
                let hunk_line = &hunk_content[line_range.clone()];

                let line_highlights = hunk_highlights.get_line_highlights(line_i);

                Line::from_iter(line_highlights.iter().map(|(highlight_range, style)| {
                    Span::styled(
                        hunk_line[highlight_range.clone()].replace("\t", "    "),
                        *style,
                    )
                }))
            }
            ItemData::Stash { message, id, .. } => Line::from(vec![
                Span::styled(format!("stash@{id}"), &config.style.hash),
                Span::raw(format!(" {message}")),
            ]),
            ItemData::Header(header) => {
                let content = match header {
                    SectionHeader::Remote(remote) => format!("Remote {remote}"),
                    SectionHeader::Tags => "Tags".to_string(),
                    SectionHeader::Branches => "Branches".to_string(),
                    SectionHeader::NoBranch => "No branch".to_string(),
                    SectionHeader::OnBranch(branch) => format!("On branch {branch}"),
                    SectionHeader::Rebase(head, onto) => format!("Rebasing {head} onto {onto}"),
                    SectionHeader::Merge(head) => format!("Merging {head}"),
                    SectionHeader::Revert(head) => format!("Reverting {head}"),
                    SectionHeader::Stashes => "Stashes".to_string(),
                    SectionHeader::RecentCommits => "Recent commits".to_string(),
                    SectionHeader::Commit(oid) => format!("commit {oid}"),
                };

                Line::styled(content, &config.style.section_header)
            }
            ItemData::BranchStatus(upstream, ahead, behind) => {
                let content = if ahead == 0 && behind == 0 {
                    format!("Your branch is up to date with '{upstream}'.")
                } else if ahead > 0 && behind == 0 {
                    format!("Your branch is ahead of '{upstream}' by {ahead} commit(s).",)
                } else if ahead == 0 && behind > 0 {
                    format!("Your branch is behind '{upstream}' by {behind} commit(s).",)
                } else {
                    format!(
                        "Your branch and '{upstream}' have diverged,\nand have {ahead} and {behind} different commits each, respectively."
                    )
                };

                Line::raw(content)
            }
            ItemData::Error(err) => Line::raw(err),
        }
    }
}

pub(crate) fn create_diff_items(
    diff: &Rc<Diff>,
    depth: usize,
    default_collapsed: bool,
) -> impl Iterator<Item = Item> + '_ {
    diff.file_diffs
        .iter()
        .enumerate()
        .flat_map(move |(file_i, file_diff)| {
            iter::once(Item {
                id: hash(diff.file_diff_header(file_i)),
                default_collapsed,
                depth,
                data: ItemData::Delta {
                    diff: Rc::clone(diff),
                    file_i,
                },
                ..Default::default()
            })
            .chain(file_diff.hunks.iter().cloned().enumerate().flat_map(
                move |(hunk_i, _hunk)| {
                    create_hunk_items(Rc::clone(diff), file_i, hunk_i, depth + 1)
                },
            ))
        })
}

fn create_hunk_items(
    diff: Rc<Diff>,
    file_i: usize,
    hunk_i: usize,
    depth: usize,
) -> impl Iterator<Item = Item> {
    let hunk_hash = hash([diff.file_diff_header(file_i), diff.hunk(file_i, hunk_i)]);
    iter::once(Item {
        id: hunk_hash,
        depth,
        data: ItemData::Hunk {
            diff: Rc::clone(&diff),
            file_i,
            hunk_i,
        },
        ..Default::default()
    })
    .chain(format_diff_hunk_items(
        diff,
        file_i,
        hunk_i,
        depth + 1,
        hunk_hash,
    ))
}

fn format_diff_hunk_items(
    diff: Rc<Diff>,
    file_i: usize,
    hunk_i: usize,
    depth: usize,
    hunk_hash: u64,
) -> Vec<Item> {
    let hunk_content = diff.hunk_content(file_i, hunk_i);

    highlight::line_range_iterator(hunk_content)
        .enumerate()
        .map(|(line_index, (line_range, line))| {
            Item {
                id: hunk_hash,
                // line is marked unselectable if it starts with a space character
                unselectable: line.starts_with(' '),
                depth,
                data: ItemData::HunkLine {
                    diff: Rc::clone(&diff),
                    file_i,
                    hunk_i,
                    line_i: line_index,
                    line_range,
                },
                ..Default::default()
            }
        })
        .collect()
}

pub(crate) fn stash_list(repo: &Repository, limit: usize) -> Res<Vec<Item>> {
    Ok(repo
        .reflog("refs/stash")
        .map_err(Error::StashList)?
        .iter()
        .enumerate()
        .map(|(i, stash)| -> Res<Item> {
            let stash_id = stash.id_new();
            let stash_ref = format!("stash@{{{}}}", i);
            Ok(Item {
                id: hash(stash_id),
                depth: 1,
                data: ItemData::Stash {
                    message: stash.message().unwrap_or("").to_string(),
                    stash_ref,
                    id: i,
                },
                ..Default::default()
            })
        })
        .map(|result| match result {
            Ok(item) => item,
            Err(err) => {
                let err = err.to_string();
                Item {
                    id: hash(&err),
                    data: ItemData::Error(err),
                    ..Default::default()
                }
            }
        })
        .take(limit)
        .collect::<Vec<_>>())
}

pub(crate) fn log(
    repo: &Repository,
    limit: usize,
    rev: Option<Oid>,
    msg_regex: Option<Regex>,
) -> Res<Vec<Item>> {
    let mut revwalk = repo.revwalk().map_err(Error::ReadLog)?;
    if let Some(r) = rev {
        revwalk.push(r).map_err(Error::ReadLog)?;
    } else if revwalk.push_head().is_err() {
        return Ok(vec![]);
    }

    let references: Vec<_> = repo
        .references()
        .map_err(Error::ReadLog)?
        .filter_map(Result::ok)
        .filter_map(
            |reference| match (reference.peel_to_commit(), reference.shorthand()) {
                (Ok(target), Some(name)) => {
                    if name.ends_with("/HEAD") || name.starts_with("prefetch/remotes/") {
                        return None;
                    }

                    let name = name.to_owned();

                    let ref_kind = if reference.is_remote() {
                        RefKind::Remote(name)
                    } else if reference.is_tag() {
                        RefKind::Tag(name)
                    } else {
                        RefKind::Branch(name)
                    };

                    Some((target, ref_kind))
                }
                _ => None,
            },
        )
        .collect();

    let items: Vec<Item> = revwalk
        .map(|oid_result| -> Res<Option<Item>> {
            let oid = oid_result.map_err(Error::ReadLog)?;
            let commit = repo.find_commit(oid).map_err(Error::ReadLog)?;

            let short_id = commit.as_object().short_id().map_err(Error::ReadOid)?;
            let short_id = String::from_utf8_lossy(&short_id).to_string();

            if let Some(re) = &msg_regex
                && !re.is_match(commit.message().unwrap_or(""))
            {
                return Ok(None);
            }

            let associated_references: Vec<_> = references
                .iter()
                .filter(|(commit, _)| commit.id() == oid)
                .map(|(_, reference)| reference.clone())
                .collect();

            let parent_ids: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();

            let data = ItemData::Commit {
                oid: oid.to_string(),
                short_id,
                associated_references,
                summary: commit.summary().unwrap_or("").to_string(),
                parent_ids,
                graph_data: None, // Will be filled in by graph layout algorithm
            };

            Ok(Some(Item {
                id: hash(oid),
                depth: 1,
                data,
                ..Default::default()
            }))
        })
        .filter_map(|result| match result {
            Ok(item) => item,
            Err(err) => {
                let err = err.to_string();
                Some(Item {
                    id: hash(&err),
                    data: ItemData::Error(err),
                    ..Default::default()
                })
            }
        })
        .take(limit)
        .collect();

    if items.is_empty() {
        Ok(vec![Item {
            data: ItemData::Raw("No commits found".to_string()),
            ..Default::default()
        }])
    } else {
        Ok(items)
    }
}

pub(crate) fn blank_line() -> Item {
    Item {
        depth: 0,
        unselectable: true,
        ..Default::default()
    }
}

pub(crate) fn hash<T: Hash>(x: T) -> ItemId {
    let mut hasher = DefaultHasher::new();
    x.hash(&mut hasher);
    hasher.finish()
}
