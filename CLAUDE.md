# Dactile

Stage play writer for the tiyatro XeLaTeX class. Rust workspace: `core` (crate `dactile`), `tui` (Dactile Light), `backend` (the desktop's commands without Tauri, also served by dactile-web), `app/src-tauri` + `app/src` (Dactile, the desktop app: Tauri 2, Svelte 5, CodeMirror 6; the window calls the backend through one `call` command). The class is in `tex/`.

`core` and `tex/` are also used by dactile-web, pinned to a commit: this team owns them, the web team asks. Keep `core` compiling for `wasm32-unknown-unknown`.

Checks: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `npm run check` in `app/`, `tex/test/run.sh`.

## Working

One session does the work, directly on `main`. Agents in `.claude/agents/` are there if a job really needs one; none run by default, and two only one after the other. This is a small project on a small machine: no team reports, no worktrees.

- Tests and clippy pass, the code that writes files gets a look, the user judges how it looks.
- One compile at a time: `flock ~/.cache/dactile-team/build.lock cargo ...`.

## Rules

- Tests never touch real user settings or files: `.cargo/config.toml` points `cargo test` and `cargo run` into `target`, and `core/tests/settings_stay_out.rs` checks it. A binary run directly needs `DACTILE_CONFIG` and `XDG_CONFIG_HOME` set to a temporary place.
- This repository is public: fictional names only in code, tests, comments and commits. Book text never enters it. Check the diff before each commit.
- Never take screenshots of the user's screen. Test in headless Weston or a browser pane.
- Kill test processes by explicit PID after listing them. No `pkill`.
- Don't push, tag or release without the user's yes for that action.
- Commit with `git -c user.email=D4shwood@users.noreply.github.com commit` and end with the `Co-Authored-By` line.
- The cursor always blinks.
- The user runs sudo themselves. Reply to the user in Turkish, using they/them.
