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

\perde[Kar Yağarken]
\sahne[İç, Değirmen — Akşam]

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

| Type | You get |
|---|---|
| `\se ` (an alias and a space) | a line for SELİN; an alias nobody declared opens the new-character form |
| `\sedo ` | a direction, `(Selin …)`, with the verbs this play uses most |
| `\` | the suggestion menu: characters (those in the scene first), commands, macros |
| `\sahne`, `\perde` | a form for the place and time, or the act's title |
| `((` | a direction inside a line, `\ic{…}` |

A macro is a chip: the cursor steps over it, and Backspace into it shows the
line's source until you leave it. F2 shows the whole play as source.

![Suggestions](docs/menu.png)

| Key | |
|---|---|
| ^O | the cast: edit, rename an alias everywhere, order, character cards |
| ^G | the scenes, who is in each, running times |
| ^K | search characters and commands |
| ^F | find and replace |
| Alt+I | cover details (title, author, draft number…) and draft/final |
| Alt+S | spelling suggestions for the word at the cursor |
| Alt+P | show the cast and cover lines of the source |
| F3 / F4 / F5 | typewriter, scene focus, dim the rest |
| F6 | the play on a paper page, or dark |
| ^Z ^Y, ^C ^X ^V | undo, redo, copy, cut, paste |
| ^S, ^Q | save (it saves by itself anyway), quit |
| F1 | all of the above |

![The cast](docs/cast.png)

Next to a play Dactile keeps `oyun.dactile.toml`, with the character cards
and the play's own words for the spell checker (the PDF never sees it), and
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
- the cast table (`\kisiler`) and a cover
- **draft** and **final** modes:

  | Mode | Font | Spacing | Margin | Cover |
  |---|---|---|---|---|
  | draft | sans | 1.5× | one number per line of dialogue or direction | "TASLAK" |
  | final | serif | single | none | the starred characters |

```sh
xelatex oyun.tex && xelatex oyun.tex
```

The class grew out of [tiyatro-latex](https://github.com/D4shwood/tiyatro-latex)
and was rewritten for Dactile. The idea of characters as commands comes from
[theatre.cls](https://github.com/DavideFauri/theatre) by Davide Fauri; none of
its code is used here.

## Roadmap

PDFs from inside the app, actor copies, importing from Word and more: see
[TODO.md](TODO.md).

## License

GPL-3.0-or-later, the app and the class alike. See [LICENSE](LICENSE).
