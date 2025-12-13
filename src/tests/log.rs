use super::*;
use crate::config::LogGraphStyle;

fn setup(ctx: TestContext) -> TestContext {
    commit(&ctx.dir, "third commit", "");
    commit(&ctx.dir, "second commit", "");
    commit(&ctx.dir, "first commit", "");
    ctx
}

#[test]
fn limit_prompt() {
    snapshot!(setup(setup_clone!()), "l-n-n");
}

#[test]
fn limit_set_10() {
    snapshot!(setup(setup_clone!()), "l-n-n10<enter>");
}

#[test]
fn limit_invalid() {
    snapshot!(setup(setup_clone!()), "l-n-nfff<enter>");
}

#[test]
fn limit_2_commits() {
    snapshot!(setup(setup_clone!()), "l-n-n2<enter>l");
}

#[test]
fn limit_2_commits_other() {
    snapshot!(setup(setup_clone!()), "l-n-n2<enter>l");
}

#[test]
fn grep_prompt() {
    snapshot!(setup(setup_clone!()), "l-F");
}

#[test]
fn grep_set_example() {
    snapshot!(setup(setup_clone!()), "l-Fexample<enter>");
}

#[test]
fn grep_second() {
    snapshot!(setup(setup_clone!()), "l-Fsecond<enter>l");
}

#[test]
fn grep_no_match() {
    snapshot!(setup(setup_clone!()), "l-Fdoesntexist<enter>l");
}

#[test]
fn grep_second_other() {
    snapshot!(setup(setup_clone!()), "l-Fsecond<enter>omain<enter>");
}

#[test]
fn log_other_prompt() {
    snapshot!(setup(setup_clone!()), "lljlo");
}

#[test]
fn log_other() {
    snapshot!(setup(setup_clone!()), "lljlo<enter>");
}

#[test]
fn log_other_input() {
    snapshot!(setup(setup_clone!()), "lomain~1<enter>");
}

#[test]
fn log_other_invalid() {
    snapshot!(setup(setup_clone!()), "lo <enter>");
}

// Graph visualization tests

#[test]
fn graph_linear_ascii() {
    let mut ctx = setup_clone!();
    ctx.config().general.log_graph_style = LogGraphStyle::Ascii;

    commit(&ctx.dir, "fourth", "content4");
    commit(&ctx.dir, "third", "content3");
    commit(&ctx.dir, "second", "content2");

    snapshot!(ctx, "ll");
}

#[test]
fn graph_linear_unicode() {
    let mut ctx = setup_clone!();
    ctx.config().general.log_graph_style = LogGraphStyle::Unicode;

    commit(&ctx.dir, "fourth", "content4");
    commit(&ctx.dir, "third", "content3");
    commit(&ctx.dir, "second", "content2");

    snapshot!(ctx, "ll");
}

#[test]
fn graph_merge_commit() {
    let mut ctx = setup_clone!();
    ctx.config().general.log_graph_style = LogGraphStyle::Ascii;

    // Create a branch
    run(&ctx.dir, &["git", "checkout", "-b", "feature"]);
    commit(&ctx.dir, "feature1", "feature work 1");
    commit(&ctx.dir, "feature2", "feature work 2");

    // Go back to main and make a commit
    run(&ctx.dir, &["git", "checkout", "main"]);
    commit(&ctx.dir, "main-work", "main work");

    // Merge feature branch
    run(
        &ctx.dir,
        &[
            "git",
            "merge",
            "feature",
            "--no-ff",
            "-m",
            "Merge feature branch",
        ],
    );

    snapshot!(ctx, "ll");
}

#[test]
fn graph_complex_branching() {
    let mut ctx = setup_clone!();
    ctx.config().general.log_graph_style = LogGraphStyle::Unicode;

    // Create initial commits on main
    commit(&ctx.dir, "base1", "base content 1");

    // Create feature-a branch
    run(&ctx.dir, &["git", "checkout", "-b", "feature-a"]);
    commit(&ctx.dir, "feature-a1", "feature a work 1");
    commit(&ctx.dir, "feature-a2", "feature a work 2");

    // Go back to main and create feature-b
    run(&ctx.dir, &["git", "checkout", "main"]);
    run(&ctx.dir, &["git", "checkout", "-b", "feature-b"]);
    commit(&ctx.dir, "feature-b1", "feature b work 1");

    // Back to main, make another commit
    run(&ctx.dir, &["git", "checkout", "main"]);
    commit(&ctx.dir, "main2", "main work 2");

    // Merge feature-a
    run(
        &ctx.dir,
        &[
            "git",
            "merge",
            "feature-a",
            "--no-ff",
            "-m",
            "Merge feature-a",
        ],
    );

    // Merge feature-b
    run(
        &ctx.dir,
        &[
            "git",
            "merge",
            "feature-b",
            "--no-ff",
            "-m",
            "Merge feature-b",
        ],
    );

    snapshot!(ctx, "ll");
}

#[test]
fn graph_octopus_merge() {
    let mut ctx = setup_clone!();
    ctx.config().general.log_graph_style = LogGraphStyle::Ascii;

    // Create base commit
    commit(&ctx.dir, "base", "base");

    // Create multiple branches
    run(&ctx.dir, &["git", "checkout", "-b", "branch1"]);
    commit(&ctx.dir, "file1", "content1");

    run(&ctx.dir, &["git", "checkout", "main"]);
    run(&ctx.dir, &["git", "checkout", "-b", "branch2"]);
    commit(&ctx.dir, "file2", "content2");

    run(&ctx.dir, &["git", "checkout", "main"]);
    run(&ctx.dir, &["git", "checkout", "-b", "branch3"]);
    commit(&ctx.dir, "file3", "content3");

    // Octopus merge (merge all branches at once)
    run(&ctx.dir, &["git", "checkout", "main"]);
    run(
        &ctx.dir,
        &[
            "git",
            "merge",
            "branch1",
            "branch2",
            "branch3",
            "-m",
            "Octopus merge",
        ],
    );

    snapshot!(ctx, "ll");
}

#[test]
fn graph_criss_cross_merge() {
    let mut ctx = setup_clone!();
    ctx.config().general.log_graph_style = LogGraphStyle::Unicode;

    // Create initial state
    commit(&ctx.dir, "initial", "initial");

    // Create two branches
    run(&ctx.dir, &["git", "checkout", "-b", "left"]);
    commit(&ctx.dir, "left1", "left work 1");

    run(&ctx.dir, &["git", "checkout", "main"]);
    run(&ctx.dir, &["git", "checkout", "-b", "right"]);
    commit(&ctx.dir, "right1", "right work 1");

    // Cross-merge: left merges right
    run(&ctx.dir, &["git", "checkout", "left"]);
    run(
        &ctx.dir,
        &[
            "git",
            "merge",
            "right",
            "--no-ff",
            "-m",
            "left merges right",
        ],
    );
    commit(&ctx.dir, "left2", "left work 2");

    // Cross-merge: right merges left
    run(&ctx.dir, &["git", "checkout", "right"]);
    run(
        &ctx.dir,
        &["git", "merge", "left", "--no-ff", "-m", "right merges left"],
    );
    commit(&ctx.dir, "right2", "right work 2");

    // Final merge on main
    run(&ctx.dir, &["git", "checkout", "main"]);
    run(
        &ctx.dir,
        &[
            "git",
            "merge",
            "left",
            "right",
            "--no-ff",
            "-m",
            "Final criss-cross merge",
        ],
    );

    snapshot!(ctx, "ll");
}

#[test]
fn graph_disabled() {
    let mut ctx = setup_clone!();
    ctx.config().general.log_graph_style = LogGraphStyle::None;

    // Create a merge commit
    run(&ctx.dir, &["git", "checkout", "-b", "feature"]);
    commit(&ctx.dir, "feature1", "feature work");
    run(&ctx.dir, &["git", "checkout", "main"]);
    commit(&ctx.dir, "main-work", "main work");
    run(
        &ctx.dir,
        &["git", "merge", "feature", "--no-ff", "-m", "Merge feature"],
    );

    // Graph should not be displayed
    snapshot!(ctx, "ll");
}
