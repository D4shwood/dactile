# TODO

## Version 1

~~M0–M7~~ done: the skeleton and class, source and cast, lexer and outline,
the writer, the `\` menu and forms, panels and text tools, the home screen and
setup, the README and release. Published as 0.1.0-beta.3.

## 0.2, by milestone

Each milestone ends with a demo and a beta, published once approved.

1. **M8 PDF builds** → 0.2.0-beta.1
   - One pass, not two:
     - The class reads `play.cast` if it is there: who is in each scene, for
       the headings. Dactile writes it on every save and before each build,
       from the outline (the same as the class's `.aux`, which `--aux` checks).
     - A plain `xelatex` run is right in one pass too.
     - At each scene's end the class checks the file against who spoke; if
       they differ, LaTeX's "rerun" warning, and the `.aux` as before. With no
       file, the old way.
     - Later data can use the same file: entrances, actors.
   - F5 or "PDF" on ^P: one XeLaTeX run in the background, writing goes on,
     progress in the status line. Every output beside the play, as by hand
     (`library::OUTPUTS` already follows them). One PDF, the play's own
     options.
   - Never opened by itself. Open it with `xdg-open` from the "PDF ready — o
     to open" message, from "Open PDF" on ^P, or with a key on the home screen.
   - No build errors, and nothing that pulls the eye while writing:
     - A thin, faint underline where something would break; the detail in the
       status line only with the cursor there.
     - Simple ones fixed on save, quietly: unescaped `& % $ # _` in dialogue,
       as the forms already do.
     - A check before the build: an unclosed `{`, an unknown command, an
       undefined character, a broken `\ic` or direction. If it finds one, no
       build, and the cursor goes there.
     - If XeLaTeX still fails (`-interaction=nonstopmode -halt-on-error`):
       one line in the status line, the log a key away, and the case added to
       the check.
     - A test with broken sources in `tex/test`.
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
