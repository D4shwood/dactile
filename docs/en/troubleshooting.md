# Troubleshooting

## The PDF won't build

First see what would stop XeLaTeX, without running it:

```sh
dactile-light check oyun.tex
```

| Error | Why | Fix |
|---|---|---|
| `Undefined control sequence` | The alias at the line's start isn't declared | Fix the alias or add the character. `dactile-light cast oyun.tex` lists them |
| `Missing \begin{document}` | A broken line before `\begin{document}`, in the cast | Look at the line the error names |
| Letters missing in the PDF | A broken character in the text, `�` | Correct it in the text; imports usually leave these |
| Characters missing under a scene heading | The play was edited elsewhere | Build once more |
| No PDF at all | XeLaTeX isn't installed | Install `texlive-xetex` and `texlive-latexextra` |

To change an alias all through a play, use the cast panel.

## A play opens read-only

It is open in another Dactile window: close that one. The file
`.oyun.tex.dactile-lock` beside the play says so. A lock left by a crash does no harm.

## My plays are gone

Check the plays folder under Settings. Settings are in
`~/.config/dactile/config.toml`, the folder on its `plays_dir` line.

## Going back to an earlier version

Earlier versions are under `~/.local/share/dactile/history`. Deleted plays go to the
desktop's trash.

## Spell checking doesn't work

`tr_TR.aff` and `tr_TR.dic` must be in `~/.local/share/hunspell` or
`/usr/share/hunspell`.

## A scanned PDF reads badly

Try **Read again with OCR** in the preview. If it is still poor, mark the parts by
hand under Document.

## Files

| File | What it is |
|---|---|
| `oyun.tex` | The play itself |
| `oyun.dactile.toml` | Character cards, scene summaries, castings, the play's own words |
| `oyun.notlar.md` | Notes |
| `.oyun.tex.dactile-lock` | The lock while the play is open |
| `oyun-0.2.tex` | A draft |
| `oyun-kopyalar/` | Actor copies |
| `oyun-tablo.csv`, `oyun-tablo.pdf` | The scene and role table |
| `~/.config/dactile/config.toml` | Settings |
| `~/.config/dactile/templates` | Your own templates |

## The command line

| Command | |
|---|---|
| `dactile-light new "Name"` | A new play |
| `dactile-light cast oyun.tex` | Lists the characters, changes nothing |
| `dactile-light outline oyun.tex` | Lists acts and scenes |
| `dactile-light check oyun.tex` | Lists what would stop XeLaTeX |
| `dactile-light pdf oyun.tex` | Builds the PDF; `--open` opens it |
| `dactile-light import FILE` | Imports a file |
| `dactile-light book kitap.tex` | Builds a book as one PDF |
