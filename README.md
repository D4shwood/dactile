# Dactile — Dynamic Stagewright

[Türkçe](README.tr.md)

> In development. Nothing to install yet; this page describes where it is going.

A terminal app for writing stage plays. You write the play in the syntax of the
**tiyatro** XeLaTeX class, and the macros come alive as you type: `\ma Merhaba`
turns into `MARİA : Merhaba` on the spot, `\sahne` opens a numbered scene, and
the characters who appear in a scene gather under its heading by themselves.

The file stays a plain `.tex` that compiles with XeLaTeX, so a play written in
Dactile still opens in any TeX editor.

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

Planned in milestones; details in [TODO.md](TODO.md).

## License

GPL-3.0-or-later, the app and the class alike. See [LICENSE](LICENSE).
