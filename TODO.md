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
   - ~~No build errors~~ Done (`core/src/check.rs`, `dactile check <play>`; tested
     against XeLaTeX itself: `cargo test -p dactile --test broken_sources -- --ignored`),
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
2. ~~**M9 the board**~~ Done, published as 0.2.0-beta.2
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
3. ~~**M10 entrances and exits**~~ Done, kept small; its class commands in 0.2.0-beta.2
   - `\girer{…}` / `\cikar{…}` in the class, printed as a direction:
     "(SELİN girer.)", "(SELİN ile OZAN girerler.)"; in the `\` menu.
   - `core/src/stage.rs` reads who is on stage, quietly: the commands, and verbs
     in directions (girer, gelir, sahneye çıkar / çıkar, gider…); each scene
     starts empty, the opening directions and whoever speaks fill it.
   - Nothing on screen while writing: no chips, no underlines. It feeds M11
     (doubling, entrances in actor copies) and a check run on demand (1.x).
4. ~~**M11 actors**~~ Done, published as 0.2.0-beta.3
   - F9, full screen: castings in the sidecar (several per play, an A and a
     B lineup), an actor per role; the play's own PDF never shows them.
   - Actor copies, all at once (`oyun-kopyalar/`): the role's lines on a grey
     band, the rest faint, their entrances and exits bold (from
     `core/src/stage.rs`), scenes without them down to their headings, a cover
     with the actor, their scenes and pages, lines, words and time. Class
     1.4.0: `\tiyatrocopy`, `\tiyatromark`, `\tiyatrocopycover`.
   - The scene/role table: on F9, as CSV, as a landscape PDF.
   - Numbers: lines, words, time and share by role; scene and act lengths;
     the play's pace.
   - Shift+F8: the cursor's role lit, the rest faint.
   - Memorizing: the cues, the role's line hidden; Space shows first
     letters, then all of it.
   - Doubling (one actor in two roles never on at once) left for later.
   - A review after it, published as 0.2.0-beta.4: faster PDF builds, safer
     saves, a lighter undo, the writer split into parts, incremental layout.
5. **M12 import** → 0.2.0-beta.5
   - From `.docx` (italics as directions, bold as names), plain text and
     Markdown, `.odt`, and PDFs with text (the preview matters most there).
   - All through text, then the shaping a paste already gets: `NAME: …`,
     `(…)`, `#…`.
   - A preview of the names found: fix who is who, merge, confirm.
   - "Import" on the home screen.
   - Done: `i` on the home screen (the desktop's file chooser through its
     portal, a path where there is none) and `dactile import`; read in Rust
     (`pdftotext` for PDFs where it is installed); a cast list, act and
     scene headings, the preface and notes, four ways of writing a line.
   - Also: the class breaks a speech's lines itself (1.5.0), so the line
     ends of an imported speech print as they read.
6. ~~**M13 notes**~~ Done → 0.2.0-beta.6
   - Notes from anyone (an AI, a workshop, a reader, oneself) in
     `oyun.notlar.md` beside the play, a Markdown task list; ^N shows them
     on the right, each one's place underlined in its author's colour.
   - Pasted or read from a file (numbered items, headings as groups), or
     written on the selection; found again by what they quote.
   - Done, hidden, archived, replies, edits; `u` makes a `“wrong” → right`
     correction, changing only the words that differ; "looks fixed" when
     the quote is gone.
   - A PDF copy with the open notes in the margin and a list of all of them
     at the end (class 1.6.0).
7. ~~**M13b menus and a start screen**~~ Done → 0.2.0-beta.6
   - The writer's ribbon (Dosya, Giriş, Ekle, Görünüm, Oyun, Notlar,
     Yardım): buttons in groups with Unicode icons, key-tip letters on F10,
     the arrows, the mouse; folds with ^F1; Dosya is a page of its own
     (the play's details, PDF, copies, templates); quick access and a
     command search on the tabs' row; the palette is made from it.
   - The home screen as a word processor's start screen: a column (Giriş,
     Yeni, Aç, Ayarlar, Tuşlar), tiles over the plays (an empty play, two
     skeletons, open/import, the example).
   - Templates: four skeletons (one act, three acts, a monologue, a short
     sketch) and one's own, made from a play ("Save as a template": its
     shape and cast, not its words) in `~/.config/dactile/templates`.
8. **M14 1.0** → 1.0.0
   - A bug round; tests for lost work (lock, autosave, trash, renames); speed
     on long plays.
   - Renaming a character in plain text too ("Selin'in" → "Selma'nın",
     suffixes by vowel harmony), and its inline name (`\setinlinename`).
   - A GIF for the README, a short guide.
   - An AUR package (`dactile-bin`) from the release workflow.

## 1.x

- Scanned PDFs: a separate tool for OCR (Tesseract with Turkish), whose
  text then goes through the import; many plays only go round so


- Doubling: one actor in two roles never on stage at once, with time to
  change between an exit and the next entrance (from `core/src/stage.rs`)

- A continuity check, run on demand (never while writing): someone who
  went out speaks again, someone who never leaves; from `core/src/stage.rs`

- A brutalist home screen, on `b`: kept on the `brutal-home` branch (raw
  concrete grey, black rules, one oxide red), hidden until it is developed

- Revision tracking: version diffs and revision marks
- A play as a folder, acts in separate files with `\input`
- Sharing within a troupe; director's notes
- Tab or Alt+1..9 to pick the speaker
- The class: English command names; check fonts without TeX's missing-font noise

## Far off

- Windows, a phone app on the same core
- To discuss: a desktop app (not in a terminal) on the same core, where the
  board's cards and the notes could be dragged and dropped
