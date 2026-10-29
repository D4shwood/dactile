# Dactile — Dynamic Stagewright

[Türkçe](README.tr.md)

A terminal app for writing stage plays. You write the play in the syntax of the
**tiyatro** XeLaTeX class, and the macros come alive as you type: `\se Amca`
turns into `SELİN   Amca` on the spot, `\sahne` asks for a place and a time
and opens a numbered scene, and the characters who appear in a scene gather
under its heading by themselves.

The file stays a plain `.tex` that compiles with XeLaTeX, so a play written in
Dactile still opens in any TeX editor, and Dactile opens plays written
elsewhere.

![The writer](docs/writer.png)

> Beta. Every change is an edit of the text itself, saved atomically, with
> earlier versions kept; but the app is new, so keep a copy of anything
> precious.

```latex
\karakter*[Selin][Köy öğretmeni]{SELİN}{se}
\karakter[Ozan Usta][Değirmenci]{OZAN}{oz}

#perde Kar Yağarken
#sahne İç, Değirmen — Akşam

\se Amca, yine mi taş durdu?
\oz Durdu kızım. \ic{elini taşa koyar} Soğuk her şeyi durdurur.
\sedo\ozdo Birbirlerine bakarlar.
```

See [examples/ornek-oyun.tex](examples/ornek-oyun.tex) for every feature.

## Install

On Linux, from [Releases](https://github.com/D4shwood/dactile/releases):

```sh
tar xzf dactile-*-x86_64-linux.tar.gz
cd dactile-*-x86_64-linux
./dactile install
```

`dactile install` puts the `dactile` command in `~/.local/bin`, adds a menu
entry, and copies the tiyatro class into your TeX tree (`~/texmf`), so plays
compile from any folder. `dactile uninstall` removes all three; your plays
and settings stay.

From source: `cargo install --path .`, then `dactile install`.

- **Spell checking** needs a Turkish Hunspell dictionary (`tr_TR.aff` and
  `tr_TR.dic`) in `~/.local/share/hunspell` or `/usr/share/hunspell`; on
  Arch, `paru -S hunspell-tr`. Without one, everything else works.
- **PDFs** need XeLaTeX (`texlive-xetex` and `texlive-latexextra`, or a full
  TeX Live).

## Using it

```sh
dactile                   # the home screen: your plays, a new one, settings
dactile oyun.tex          # straight into a play
dactile new "Kar Masalı"  # a new play in your plays folder
```

The first run asks for the language (Turkish or English), the folder your
plays live in, your name, and how the suggestion menu opens.

![Home](docs/home.png)

On the home screen `d` copies the chosen play under a new title, and `f`
starts a new draft of it next to it (`kar-masali-0.2.tex`, the draft number
one up, the character cards along). Drafts stack up above the play they came
from as a tree, the newest on top, and the home screen opens on the play you
worked on last. F2 renames a play right in its row, as a file manager does:
type the new title, and its file, and the files that go with it, follow (a
draft keeps its number at the end of its file name). Shift+F2 gives the title
and the file name separately. `p` opens its PDF. Del, pressed twice, sends
it to the desktop's trash, and `u` brings it back; `?` lists every key.
Beside the list stands the chosen play's first page, as tall as the screen,
laid out as the writer lays it out, in the writer's theme. Under the list,
level with the page's foot: the play as a strip of its scenes, each as long
as it plays, then its scenes with their times and who speaks how much.

| Type | You get |
|---|---|
| `\se ` (an alias and a space) | a line for SELİN; an alias nobody declared opens the new-character form |
| `\sedo ` | a direction, `(Selin …)`, with the verbs this play uses most |
| `\` | the suggestion menu: characters (those in the scene first), commands, macros |
| `#` at a line's start | the headings: `#sahne`, `#perde`, `#önsöz` (a preface), `#notlar` (notes for the director) |
| `#sahne İç, Değirmen — Akşam` | a scene, typed straight; choosing it from the `#` menu asks for place and time |
| `SELİN:` at a line's start | a line for SELİN (any of her names, any case); an unknown name in capitals opens the new-character form |
| Tab on an empty line | who speaks next, shown faint: the one before the last speaker |
| `((` | a direction inside a line, `\ic{…}` |
| ^V at a line's start | a draft pasted from elsewhere: `NAME: …` lines become lines, `(…)` lines directions, `#…` lines headings |

A macro is a chip: the cursor steps over it, and Backspace into it shows the
line's source until you leave it. F2 shows the whole play as source.

The page reads like the PDF, the curtain at the end of an act; F7 cuts it
into A4 sheets where the PDF starts a new page (the cover, the preface, the
cast table, each act) and where a sheet fills up, with page numbers. On a wide
screen the play's outline sits in the margin; a click goes there.

![Suggestions](docs/menu.png)

| Key | |
|---|---|
| ^P | the command palette: everything by name — new scene, themes, page view, animations… |
| ^O | the cast: edit, rename an alias everywhere, order, character cards; m merges one into another, b splits one, giving the scenes you pick to a new character |
| ^G, ^G ^G | the scene strip beside the writing; twice quickly, the board (below) |
| Alt+E, Alt+B, Alt+J | the cursor's scene's summary; split the scene at the cursor's line; join it with the next |
| ^K | search characters and commands |
| ^F | find and replace |
| Alt+I | cover details (title, author, draft number…) and draft/final |
| Alt+S | spelling suggestions for the word at the cursor |
| Alt+P | show the cast and cover lines of the source |
| F5, Alt+O, Alt+L | build the PDF next to the play (in the background, one XeLaTeX run); open it in the viewer. What would stop XeLaTeX is underlined faintly as you write (the cursor on it says what); `&`, `$`, `^`, a stray `_` and `%50` are escaped quietly on save; a brace left open or a command nobody defined stops F5 there, and F5 again builds anyway; if XeLaTeX still fails, Alt+L opens its log |
| F3 / F4 / F8 | typewriter, scene focus, dim the rest |
| F6 | the theme: paper, sepia, night, high contrast, dark |
| F7 | page view: A4 sheets broken where the PDF breaks, numbered |
| ^Z ^Y, ^C ^X ^V | undo, redo, copy, cut, paste |
| ^S, ^Q | save (it saves by itself anyway), quit |
| F1 | a keyboard: press any key to see what it does |

![The keys](docs/help.png)

![The cast](docs/cast.png)

### The board

^G opens a strip beside the writing: the acts and scenes, the cursor's scene
lit and open, with its summary, its length and who is in it. It steps aside
when the screen is too narrow for the page and it. ^G twice quickly opens the
board, full screen, as a list or (Tab) as cards: the acts as columns, the
scenes as cards with place, length, cast and summary.

- ⏎ goes to the scene; `e` writes its summary (a line or two, never printed,
  shown faint under the heading in the writer too; Alt+E there).
- Alt+↑↓ moves the scene a place, over an act's edge into the next one;
  Alt+←→ on the cards to the act before or after. `m` picks it up, the arrows
  choose where, ⏎ puts it down. Acts move whole, the marked ones (␣) together.
  Each move is one undo.
- `j` joins the scene with the next: the second heading becomes a direction
  with its place. Alt+B in the writer splits a scene at the cursor's line.
- `n` opens a new card: a hidden scene (`%~`), out of the PDF until ⏎ opens it
  to write in.
- `h` hides (as `%~` lines), `t` moves to the play's trash, `c` shows the
  trash: r puts back, x x destroys.

Before the first move, split or join of a session the play as it was is kept
as a draft beside it (`oyun-0.2.tex`); you go on in the play itself. A scene
with a summary carries a short id at its heading's end, `#sahne Meydan %@k3`,
which the PDF never sees: the summary follows the scene wherever it goes.

Next to a play Dactile keeps `oyun.dactile.toml`, with the character cards,
the scene summaries and the play's own words for the spell checker (the PDF
never sees it), and
`.oyun.tex.dactile-lock` while the play is open, so a second Dactile opens it
read-only. Earlier versions go to `~/.local/share/dactile/history`.

## The tiyatro class

[tex/](tex/) holds the class, which Dactile carries inside itself:

- characters with aliases: `\karakter`, groups (`\karaktergrubu`), characters
  left out of the cast table (`\gizlikarakter`), and the name used inside a
  line (`\setinlinename`)
- a line of dialogue runs to the end of the source line, with the name in a
  column of its own
- directions: `\aliasdo` chains, `\action`, `\stagedir`, `\middir`, `\ic`, `\beat`
- acts and scenes with automatic numbers. Each scene heading lists who appears
  in it; this needs a second run.
- headings as `#` lines: `#perde Title`, `#sahne Place — Time`,
  `#sahne[2.1] Place`, `#önsöz`, `#notlar` (also `#act`, `#scene`, `#preface`,
  `#notes`). `\#` prints a `#`; `\perde[…]` and `\sahne[…]` work as before.
- a preface (`\onsoz`) and notes for the director (`\notlar`), each on a page
  of its own
- the cast table (`\kisiler`) and a cover
- **draft** and **final** modes:

  | Mode | Font | Spacing | Margin | Cover |
  |---|---|---|---|---|
  | draft | sans | 1.5× | one number per line of dialogue or direction | "TASLAK" |
  | final | serif | single | none | the starred characters |

- options: `nohash` makes `#` an ordinary character again; `nonotes` leaves the
  director's notes out of a copy. A `#` line, like a line of dialogue, ends at
  the end of the source line, so a `%` there joins the next line to it.

```sh
xelatex oyun.tex
```

Who is in each scene, under its heading, the class only learns at the scene's
end, so it keeps the casts in the `.aux` for the next run. Dactile writes them
there before each F5, and on every save once there is an `.aux`, so one run is
enough; after editing elsewhere, the class
warns "Rerun to get them right", and a second run fixes it.

The class grew out of [tiyatro-latex](https://github.com/D4shwood/tiyatro-latex)
and was rewritten for Dactile. The idea of characters as commands comes from
[theatre.cls](https://github.com/DavideFauri/theatre) by Davide Fauri; none of
its code is used here.

## Roadmap

Entrances and exits, actor copies, importing from Word and more: see
[TODO.md](TODO.md).

## License

GPL-3.0-or-later, the app and the class alike. See [LICENSE](LICENSE).
