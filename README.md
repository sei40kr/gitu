## It's Gitu! - A Git porcelain *outside* of Emacs
[![CI](https://github.com/altsem/gitu/actions/workflows/ci.yml/badge.svg)](https://github.com/altsem/gitu/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/altsem/gitu/graph/badge.svg?token=5YWPU7GWFW)](https://codecov.io/gh/altsem/gitu)

A terminal user interface for Git. Inspired by Magit.

<img style="width: 720px" src="vhs/rec.gif"/>

### Features
Gitu aims to implement many of the core features of Magit over time.
It should be familiar to any previous Magit users.\
Here's a list of so-far supported features:
- **Staging/Unstaging** _(file, hunk, line)_ 
- **Showing** _(view commits / open EDITOR at line)_
- **Branching** _(checkout, checkout new)_
- **Committing** _(commit, amend, fixup)_
- **Fetching**
- **Logging** _(current, other)_
- **Pulling / Pushing** _to/from configured upstream/pushDefault_
- **Rebasing** _(elsewhere, abort, continue, autosquash, interactive)_
- **Resetting** _(soft, mixed, hard)_
- **Reverting** _(commit)_
- **Stashing** _(save, pop, apply, drop)_

### Keybinds
Keybinds try mimic Magit, while staying Vim-like.
A help-menu can be shown by pressing the `h` key, or by configuring `general.always_show_help.enabled = true`


<img style="width: 720px" src="vhs/help.png"/>

### Configuration
The environment variables `VISUAL`, `EDITOR` or `GIT_EDITOR` (checked in this order) dictate which editor Gitu will open. This means that e. g. commit messages will be opened in the `GIT_EDITOR` by Git, but if the user wishes to do edits to the actual files in a different editor, `VISUAL` or `EDITOR` can be set accordingly.

Configuration is also loaded from:
- Linux:   `~/.config/gitu/config.toml`
- macOS:   `~/.config/gitu/config.toml`
- Windows: `%USERPROFILE%\AppData\Roaming\gitu\config.toml`

, refer to the [default configuration](src/default_config.toml).

### Architecture
Gitu is built with a clean separation of concerns, combining Rust's type system with functional programming patterns for a maintainable and extensible codebase.

#### High-Level Architecture

```mermaid
graph TB
    subgraph Input["Input Layer"]
        Keys["Keyboard Events<br/>crossterm"]
        Mouse["Mouse Events"]
        Resize["Resize Events"]
    end

    subgraph App["App Layer"]
        AppStruct["App<br/>├─ State<br/>├─ screens<br/>├─ pending_menu<br/>└─ pending_cmd"]
    end

    subgraph Config["Configuration"]
        ConfigStruct["Config<br/>├─ general<br/>├─ style<br/>└─ bindings"]
    end

    subgraph Binding["Binding Resolution"]
        BindingMatch["Bindings::match_bindings<br/>Menu + Keys → Op"]
    end

    subgraph Operations["Operations Layer"]
        OpTrait["OpTrait<br/>├─ get_action<br/>├─ is_target_op<br/>└─ display"]
        OpEnum["Op Enum<br/>70+ operations<br/>Stage, Commit, Rebase..."]
    end

    subgraph Git["Git Integration"]
        GitLibgit2["libgit2<br/>Repository, Status,<br/>Diff, Refs"]
        GitCLI["git CLI<br/>Interactive ops<br/>Rebase, Apply..."]
        GitParse["gitu_diff Parser<br/>Diff → FileDiff ranges"]
    end

    subgraph Screen["Screen System"]
        ScreenStruct["Screen<br/>├─ items<br/>├─ cursor<br/>├─ collapsed<br/>└─ refresh_items"]
        Items["ItemData Enum<br/>Raw, Commit, Hunk,<br/>Delta, Reference..."]
        ItemGen["Item Factory<br/>closure → Vec&lt;Item&gt;"]
    end

    subgraph Render["Rendering Layer"]
        LayoutEngine["Custom Layout<br/>Tree-based<br/>Compute positions"]
        UIFrame["ui() Function<br/>Frame rendering"]
        Highlight["Highlighting<br/>├─ Syntax<br/>├─ Diff<br/>└─ Selection"]
    end

    subgraph Term["Terminal Output"]
        Ratatui["Ratatui TUI<br/>Framework"]
        Crossterm["Crossterm<br/>Terminal Backend"]
        Display["Terminal Display"]
    end

    subgraph Support["Support Systems"]
        Prompt["Prompt<br/>User Input"]
        CmdLog["CmdLog<br/>Command Output"]
        FileWatch["FileWatcher<br/>Auto-refresh"]
    end

    Input -->|Event| AppStruct
    Config -->|Bindings, Style| BindingMatch
    Keys -->|KeyEvent| BindingMatch
    Mouse -->|MouseEvent| AppStruct
    Resize -->|ResizeEvent| AppStruct

    BindingMatch -->|Op| OpEnum
    OpEnum -->|Implementation| OpTrait

    AppStruct -->|ItemData| OpTrait
    OpTrait -->|Action Closure| Git

    Git -->|Status, Commits| Screen
    Git -->|Diff Data| GitParse
    GitParse -->|FileDiff| Screen

    Screen -->|Items| ItemGen
    ItemGen -->|Items| ScreenStruct
    ScreenStruct -->|Items| Render

    Items -->|to_line()| UIFrame
    LayoutEngine -->|Positions| UIFrame
    Highlight -->|Styles| UIFrame

    UIFrame -->|Ratatui Widgets| Ratatui
    Ratatui -->|Backend| Crossterm
    Crossterm -->|Output| Display

    Support -->|Data| AppStruct
    AppStruct -->|Updates| Screen

    style Input fill:#e1f5ff
    style Git fill:#fff3e0
    style Render fill:#f3e5f5
    style Support fill:#e8f5e9
```

#### Core Components

- **App & State**: Central state machine managing screens, menus, and git repository
- **Screen System**: Stack-based view management (Status, Log, Show, etc.) with hierarchical item trees
- **Operations**: Trait-based dispatch system with 70+ context-aware git operations
- **Git Integration**: Hybrid approach using libgit2 for simple operations and git CLI for complex scenarios
- **Rendering**: Custom layout engine with Ratatui, supporting syntax and diff highlighting
- **Item System**: Type-safe representation of git objects (commits, hunks, deltas) with context menus

#### Key Patterns

- **Event-Driven Architecture**: Non-blocking event loop with async command support
- **Trait-Based Polymorphism**: OpTrait enables contextual operation dispatch
- **Closure-Based Factories**: Screens use closures to regenerate items from git state
- **Immutable Rendering**: UI is read-only view of state, updated only through operations

### Installing Gitu
Follow the install instructions: [Installing Gitu](docs/installing.md)\
Or install from your package manager:

[![Packaging status](https://repology.org/badge/vertical-allrepos/gitu.svg)](https://repology.org/project/gitu/versions)

### Contributing
PRs are welcome!
This may help to get you started: [Development & Tooling](docs/dev-tooling.md)
