# TODO

## Version 1, by milestone

Each milestone ends with a demo.

1. ~~**M0 skeleton.**~~ Done.
   - the repo and CI
   - the class rewritten for Dactile:
     - one line number per line of dialogue or direction
     - scenes numbered from 1 in every act
     - plain parentheses, with `\ic{}` for inline directions
   - the example play, which CI compiles in both modes
2. ~~**M1 source and cast.**~~ Done; `dactile cast <play>` shows what is read.
   - Read a play into preamble, body and tail.
   - Read and write every character declaration.
   - Write the file back byte for byte.
3. ~~**M2 lexer and outline.**~~ Done; `dactile outline <play>` shows it, and `--aux` matches the class's scene lists.
   - Tokens, with the class's rules: the first macro on a line is the speaker, and a line ends at the end of the line.
   - Live act and scene numbers.
   - Who appears in each scene, and word counts.
4. ~~**M3 the writer.**~~ Done: `dactile oyun.tex`.
   - Live macros as chips; one Backspace shows the raw text.
   - Hanging-indent wrap.
   - Escaping TeX's special characters.
   - Selection, clipboard and undo.
   - Autosave, draft history, atomic writes and a lock file.
5. ~~**M4 the `\` menu, forms and look.**~~ Done.
   - The `\` suggestion menu, the verbs list, and a fuzzy search window.
   - The new-character, scene and act forms.
   - The theme, typewriter / scene focus / dimming modes, margin numbers, and the mouse.
6. ~~**M5 panels and text tools.**~~ Done.
   - The cast panel: rename, order and character cards, saved in a sidecar file.
   - The scene navigator and running time.
   - Turkish spell check (hunspell-tr), play info, find and replace, help.
7. ~~**M6** home screen, setup wizard, settings, Turkish and English.~~ Done.
8. ~~**M7** README, `dactile install`, Linux release, beta.~~ Done (screenshots; a GIF later).

## Later

- PDF builds from the app: XeLaTeX twice in the background, draft or final, errors shown at the line
- Actor copies, rehearsal mode, a scene/actor table, statistics
- Importing plays from Word or plain text
- An outline board: build the skeleton first, fill it in later
- Casting (actor ↔ role), role-specific views
- Revision tracking: version diffs and revision marks
- Entrances and exits, with a warning when someone speaks off stage
- Tab or Alt+1..9 to pick the speaker
- A play as a folder, acts in separate files with `\input`
- Sharing within a troupe; director's notes
- Moving scenes
- Renaming a character in plain text too ("Selin'in" → "Selma'nın", suffixes by vowel harmony); on hold
- On renaming a character, its inline name (`\setinlinename`) too; on hold
- Windows, a phone app on the same core
- The class: English command names; check fonts without TeX's missing-font noise
