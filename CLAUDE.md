# Dactile

Stage play writer for the tiyatro XeLaTeX class. Rust workspace: `core` (crate `dactile`), `backend` (the desktop's commands without Tauri), `app/src-tauri` + `app/src` (Dactile, the desktop app: Tauri 2, Svelte 5, CodeMirror 6; the window calls the backend through one `call` command). The class is in `tex/`. `tui/` is Dactile Light, the terminal app, retired at 0.3.0-beta.2: outside the workspace, not built or changed.

Dactile Web is here too, and its plan is `docs/web.md`. `server/`: `dactile-server`, the backend's commands over HTTP for one person, paths kept inside their folder (`confine.rs`); and `dactile-gate` (`gate.rs`), which signs people in (accounts by invitation, `people.rs`; mail, `mail.rs`; shared projects and their roles, `projects.rs`; a project's plays written live in their rooms, `room.rs`, with `app/src/lib/live.ts` on the page) and gives each a `dactile-server` of their own. `app/web/door/` is its sign-in page. `app/web/`: the browser's way in to the same interface (`app/src`), where `app/web/shim/` stands in for `@tauri-apps/api`; anything `app/src` takes from Tauri needs its shim. `web-wasm/` (`dactile-wasm`): the backend's text-only commands (`TEXT_ONLY` in `backend/src/commands.rs`) built for the browser, which runs them itself and sends the rest to the server; so `core` and `backend` must keep compiling for `wasm32-unknown-unknown`. `app/web/wasm.sh` builds it (the web scripts run it first) and needs `wasm-bindgen` of `Cargo.lock`'s version, on the PATH or in `target/tools`. `deploy/`: putting the server up, on two sites: the trial one (`deploy.sh pbe`) first, then the real one (`deploy.sh live`), which takes only what the trial one runs.

Checks: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo build -p dactile-wasm --target wasm32-unknown-unknown`, `npm run check`, `npm run check:web` and `npm test` (the themes' contrast) in `app/`, `tex/test/run.sh`. Ports: desktop dev 1420, web dev 1421 (passes `/api` on), server 1430.

Run the web locally: `DACTILE_DATA=<scratch> DACTILE_PASSWORD=<12+ chars> cargo run -p dactile-server`, and `npm run dev:web` in `app/`.

## Working

One session does the work, directly on `main`. Agents in `.claude/agents/` are there if a job really needs one; none run by default, and two only one after the other. This is a small project on a small machine: no team reports, no worktrees.

- Tests and clippy pass, the code that writes files gets a look, the user judges how it looks.
- One compile at a time: `flock ~/.cache/dactile-team/build.lock cargo ...`.

## Rules

- Tests never touch real user settings or files: `.cargo/config.toml` points `cargo test` and `cargo run` into `target`, and `core/tests/settings_stay_out.rs` checks it. A binary run directly needs `DACTILE_CONFIG` and `XDG_CONFIG_HOME` set to a temporary place.
- This repository is public: fictional names only in code, tests, comments and commits. Book text never enters it. Check the diff before each commit.
- Never take screenshots of the user's screen. Test in headless Weston or a browser pane.
- Kill test processes by explicit PID after listing them. No `pkill`.
- Don't push, tag, release or put the server up without the user's yes for that action.
- Commit with `git -c user.email=D4shwood@users.noreply.github.com commit` and end with the `Co-Authored-By` line.
- The cursor always blinks.
- The user runs sudo themselves. Reply to the user in Turkish, using they/them.
