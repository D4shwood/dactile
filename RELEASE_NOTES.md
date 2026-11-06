The first beta of Dactile 0.3, and the first of Dactile with a window of its
own: a play on a page, in the syntax of the tiyatro XeLaTeX class. Linux x86_64 only for now.

```sh
chmod +x Dactile-*-x86_64.AppImage
./Dactile-*-x86_64.AppImage
```

On Arch, `dactile-bin` from the AUR. PDFs need XeLaTeX; spell checking needs
a Turkish Hunspell dictionary.

## Two commands now

- `dactile` is Dactile.
- `dactile-light` is Dactile Light, in a terminal, in its own archive.
  If you installed an earlier beta, run `dactile uninstall` first, then
  `./dactile-light install`. Your plays and settings stay.

## Dactile

**Writing**
- The play on a page you can zoom, without the macros in the way: chips for
  speakers and directions, Enter for the answering speaker, Tab for the next.
- A ribbon as in a word processor, with key tips on Alt and a search box.
- Paper, sepia and night themes.
- F4 folds the other scenes, F8 dims all but the block you are in.

**Beside the page**
- The PDF, with a jump to a build error's line.
- Notes from anyone, on the page and in a side panel.
- A board where scenes and acts are dragged, in one undo.

**For the company**
- F9, casting: actors for roles, a role's scenes, a table as CSV or PDF, and
  rehearsal copies for each actor.
- Memorize: a role's cues with the line hidden, its first letters on Space,
  then all of it.

**Your plays**
- Import from .docx, .odt, PDF, Markdown or text, with a preview first: fix
  the names and aliases, merge two spellings of one person, say who is not
  a character. A play in a book is found by itself: where people answer each
  other, and in a PDF where page 1 follows Roman numerals. The cover, imprint,
  contents and the author's life stay out of the play and the cast; choose
  paragraphs with Shift, mark them as a part, or say where the play starts
  and ends. Misread names are put right, those under the cast list go under
  yours, and footnotes come with their lines.
- Scanned books: the app reads a PDF from its page images with Tesseract, which
  it carries with its models. A PDF with poor text can be read again the same
  way.
- A volume of several plays offers its plays one by one, directions in
  italics are read as directions, and a premiere's actors stay out of the cast.
- The class has `\digerkisiler` for those under the cast list and `\dipnot`
  for footnotes; the cast panel and the ribbon write them.
- New drafts, copies, renaming, the trash with an undo.
- A play's settings: its pace, its dictionary, the play a draft came from.

## Fixed since 0.2.0-beta.6

- Import gave two people the same alias when their names began alike, in
  Dactile Light too.

## Not yet

- The PDF with notes is greyed on Dactile's ribbon; Dactile Light makes it.
- The role view (Shift+F8) is in Dactile Light only.
