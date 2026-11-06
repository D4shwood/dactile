# Dactile

Stage play writer for the tiyatro XeLaTeX class. Rust workspace: `core` (crate `dactile`), `tui` (Dactile Light), `app/src-tauri` + `app/src` (Dactile, the desktop app: Tauri 2, Svelte 5, CodeMirror 6). The class is in `tex/`.

`core` and `tex/` are also used by dactile-web, pinned to a commit: this team owns them, the web team asks. Keep `core` compiling for `wasm32-unknown-unknown`.

Checks: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `npm run check` in `app/`, `tex/test/run.sh`.

## Team

The session in this repository is the team lead; `.claude/agents/` holds the architect, developer, designer and qa. Every change goes through qa before it is reported done.

- At most 2 agents of this team active at once. The machine has 8 cores and 15 GB, shared with the user and the other team.
- One compile at a time across both teams: `flock ~/.cache/dactile-team/build.lock cargo ...`.
- One headless UI test at a time: `flock ~/.cache/dactile-team/ui.lock ...`.
- Agents that write code in parallel use separate worktrees and separate files.

## Rules

- Tests never touch real user settings or files: set `DACTILE_CONFIG` and `XDG_CONFIG_HOME` to a temporary place.
- This repository is public: fictional names only in code, tests, comments and commits. Book text never enters it. Check the diff before each commit.
- Never take screenshots of the user's screen. Test in headless Weston or a browser pane.
- Kill test processes by explicit PID after listing them. No `pkill`.
- Don't push, tag or release without the user's yes for that action.
- Commit with `git -c user.email=D4shwood@users.noreply.github.com commit` and end with the `Co-Authored-By` line.
- The cursor always blinks.
- The user runs sudo themselves. Reply to the user in Turkish, using they/them.
