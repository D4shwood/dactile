# TODO

## Version 1

~~M0–M7~~ done: the skeleton and class, source and cast, lexer and outline,
the writer, the `\` menu and forms, panels and text tools, the home screen and
setup, the README and release. Published as 0.1.0-beta.3.

## 0.2, by milestone

Each milestone ends with a demo and a beta, published once approved.

1. ~~**M8 PDF builds**~~ Done, published as 0.2.0-beta.1
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
       line number, and Alt+L opens the log. Each new case goes into the check.
2. ~~**M9 the board**~~ Done, to be published as 0.2.0-beta.2
   - Ctrl+G opens a strip beside the writing: acts and scenes as a tree, the
     cursor's scene lit and open with its summary. Hides itself when the
     paper and the strip don't fit, comes back when they do.
   - Ctrl+G twice quickly (within 0.4 s): the board, full screen. Tab
     between two views:
     - a list: acts and scenes, the summary under each;
     - cards: acts as columns, scenes as cards (place, time, summary, who is
       in it), scrolling sideways, a small strip of acts on top.
     - Esc back to the writing, ⏎ to the scene; h, t, Space as today.
   - A summary per scene, never in the PDF: typed on the card (`e`) or at the
     scene's heading in the writer, shown faint under it.
   - Summaries live in the sidecar, tied to the scene by a short id the PDF
     never sees at the heading's end (`%@k3`): it survives moves, renames and
     edits outside Dactile.
   - A new card (`n`) is a hidden scene (`%~`); writing in it shows it.
   - Moving: Alt+↑/↓ shifts the scene (or the marked ones) a place, over an
     act's edge into the next; Alt+←/→ to another act on the cards; `m`
     picks it up, the arrows choose, ⏎ puts it down, Esc doesn't. Whole acts
     the same. Each move is one undo step.
   - Splitting at the cursor: a `#sahne` with the same place, the cursor on
     it to change it. Merging two: the second heading becomes a direction,
     `(Dış, Meydan — Gece)`, nothing lost.
   - Before the first move, split or merge of a session the play as it was
     is kept as a new draft; the writing goes on in the play itself, the
     status line names the draft.
3. **M10 entrances and exits** → 0.2.0-beta.3
   - `\girer{…}` / `\cikar{…}` in the class, printed as a direction:
     "(SELİN girer.)", "(SELİN ile OZAN girerler.)".
   - Guessed from directions, a wide list with its endings: girer, gelir,
     döner, sahneye çıkar / çıkar, gider, çekilir, kaybolur…
   - Each scene starts empty; who the opening direction names is on stage,
     and whoever speaks has come in.
   - Where unsure ("SELİN kapıya yürür."), a faint chip at the direction's
     end: Alt+⏎ writes the command, Alt+⌫ says no, never asked again
     (in the sidecar).
   - A faint underline when someone who went out speaks; (içeriden),
     (dışarıdan), (sesi), (telefonda) before the line lift it.
   - The cards and the strip show who is on stage at each scene's start and
     at the cursor.
4. **M11 actors** → 0.2.0-beta.4
   - Casting, actor ↔ role, in the sidecar; in the cast panel, and on the
     cover if wanted.
   - Actor copies, a class option on top of M8: the role's lines highlighted,
     the others whole but faint, scenes without the role down to their heading.
   - A scene/actor table, for planning rehearsals; printable.
   - A role's view in the writer: its lines lit, the rest dimmed.
   - Statistics: lines, words and time by role; scene lengths.
   - Rehearsal mode: to be described; a placeholder for now.
5. **M12 import** → 0.2.0-beta.5
   - From `.docx` (italics as directions, bold as names), plain text and
     Markdown, `.odt`, and PDFs with text (the preview matters most there).
   - All through text, then the shaping a paste already gets: `NAME: …`,
     `(…)`, `#…`.
   - A preview of the names found: fix who is who, merge, confirm.
   - "Import" on the home screen.
6. **M13 1.0** → 1.0.0
   - A bug round; tests for lost work (lock, autosave, trash, renames); speed
     on long plays.
   - Renaming a character in plain text too ("Selin'in" → "Selma'nın",
     suffixes by vowel harmony), and its inline name (`\setinlinename`).
   - A GIF for the README, a short guide.
   - An AUR package (`dactile-bin`) from the release workflow.

## 1.x

- A brutalist home screen, on `b`: kept on the `brutal-home` branch (raw
  concrete grey, black rules, one oxide red), hidden until it is developed

- Revision tracking: version diffs and revision marks
- A play as a folder, acts in separate files with `\input`
- Sharing within a troupe; director's notes
- Tab or Alt+1..9 to pick the speaker
- The class: English command names; check fonts without TeX's missing-font noise

## Far off

- Windows, a phone app on the same core
