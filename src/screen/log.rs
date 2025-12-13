use super::Screen;
use crate::{Res, config::Config, graph, items::log};
use git2::{Oid, Repository};
use ratatui::layout::Size;
use regex::Regex;
use std::{rc::Rc, sync::Arc};

pub(crate) fn create(
    config: Arc<Config>,
    repo: Rc<Repository>,
    size: Size,
    limit: usize,
    rev: Option<Oid>,
    msg_regex: Option<Regex>,
) -> Res<Screen> {
    let config_clone = Arc::clone(&config);
    Screen::new(
        Arc::clone(&config),
        size,
        Box::new(move || {
            let mut items = log(&repo, limit, rev, msg_regex.clone())?;
            // Compute graph layout if enabled
            if config_clone.general.log_graph_style != crate::config::LogGraphStyle::None {
                graph::compute_graph_layout(&mut items);
                // Insert graph line items after commits with post_commit_line data
                items = graph::insert_graph_lines(items);
            }
            Ok(items)
        }),
    )
}
