# TODO

## Version 1

~~M0–M7~~ done: the skeleton and class, source and cast, lexer and outline,
the writer, the `\` menu and forms, panels and text tools, the home screen and
setup, the README and release. Published as 0.1.0-beta.3.

## 0.2, by milestone

Each milestone ends with a demo and a beta, published once approved.

1. **M8 PDF builds** → 0.2.0-beta.1
   - ~~One pass, not two.~~ Done: Dactile writes the scene casts into the
     `.aux` (from the outline, what the class would write) before each build,
     and on every save once there is one, so a plain `xelatex` is right in
     one pass too. No second file to go stale: the class's own run rewrites
     the `.aux`. At each scene's end the class checks the heading's cast
     against who spoke; if they differ, "Rerun to get them right", and the
     build runs once more.
   - ~~F5~~ Done: one XeLaTeX run in the background with the class that came
     with the app (`TEXINPUTS`; a class beside the play still wins), writing
     goes on, seconds in the status line; every output beside the play;
     `dactile pdf <play>` from the shell. Dim moved to F8.
   - ~~Never opened by itself.~~ Done: Alt+O in the writer ("PDF ready … Alt+O
     opens it"), "Open the PDF" on ^P, `p` on the home screen.
   - ~~No build errors~~ Done (`src/check.rs`, `dactile check <play>`; tested
     against XeLaTeX itself: `cargo test --test broken_sources -- --ignored`),
     and nothing that pulls the eye while writing:
     - A thin, faint underline where something would break; the detail in the
       status line only with the cursor there.
     - Fixed on save, quietly: `&`, `$`, `^`, a `_` outside a line, `%`
       before a number (not in the block being written as source, nor in
       environments). Typing and pasting escaped these already.
     - Before the build: a brace left open or one too many, a command nobody
       defined (`\sein` → "write `\se{}in`"). No build, the cursor goes
       there; F5 again builds anyway.
     - If XeLaTeX still fails: one line in the status line with the play's
       line number. Left: the log a key away; add each new case to the check.
2. **M9 structure** → 0.2.0-beta.2
   - An outline board, two views, a key between them:
     - a list: acts and scenes, a summary under each (the scene navigator,
       grown);
     - cards: acts as columns, scenes as cards with place, time, summary and
       who is in it.
   - Summaries and notes in the sidecar, never in the PDF.
   - A new card is a hidden scene in the source (`%~`), so the PDF shows no
     empty scene; writing in it shows it.
   - Moving scenes up and down, to another act; moving whole acts; splitting
     a scene at the cursor, merging two. Numbers are live already; each is
     one undo step.
   - Before the first move or merge of a session, a new draft (`f`) by itself.
   - Entrances and exits:
     - guessed from directions like "(SELİN girer.)"; where unsure, an
       explicit `\girer{…}` / `\cikar{…}` offered, a chip in the writer,
       printed as a direction;
     - the two commands added to the class;
     - a faint underline when someone off stage speaks;
     - the cards show who is on stage at each scene's start.
3. **M10 actors** → 0.2.0-beta.3
   - Casting, actor ↔ role, in the sidecar; in the cast panel, and on the
     cover if wanted.
   - Actor copies, a class option on top of M8: the role's lines highlighted,
     the others whole but faint, scenes without the role down to their heading.
   - A scene/actor table, for planning rehearsals; printable.
   - A role's view in the writer: its lines lit, the rest dimmed.
   - Statistics: lines, words and time by role; scene lengths.
   - Rehearsal mode: to be described; a placeholder for now.
4. **M11 import** → 0.2.0-beta.4
   - From `.docx` (italics as directions, bold as names), plain text and
     Markdown, `.odt`, and PDFs with text (the preview matters most there).
   - All through text, then the shaping a paste already gets: `NAME: …`,
     `(…)`, `#…`.
   - A preview of the names found: fix who is who, merge, confirm.
   - "Import" on the home screen.
5. **M12 1.0** → 1.0.0
   - A bug round; tests for lost work (lock, autosave, trash, renames); speed
     on long plays.
   - Renaming a character in plain text too ("Selin'in" → "Selma'nın",
     suffixes by vowel harmony), and its inline name (`\setinlinename`).
   - A GIF for the README, a short guide.
   - An AUR package (`dactile-bin`) from the release workflow.

## 1.x

- Revision tracking: version diffs and revision marks
- A play as a folder, acts in separate files with `\input`
- Sharing within a troupe; director's notes
- Tab or Alt+1..9 to pick the speaker
- The class: English command names; check fonts without TeX's missing-font noise

## Far off

- Windows, a phone app on the same core
