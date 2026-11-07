# Writing

A play is written in the syntax of the [tiyatro class](tiyatro-class.md). The
commands come alive as you type: `\se Amca` turns into `SELİN   Amca` on the spot.

## Characters

Each character has a name and an alias:

```latex
\karakter*[Selin][Köy öğretmeni]{SELİN}{se}
\karakter[Ozan Usta][Değirmenci]{OZAN}{oz}
```

- `{SELİN}`: the name over the lines.
- `{se}`: what you type, `\se`.
- `*`: the character appears on the final version's cover.

The cast panel (**Ctrl+O**) is the easier way: edit, rename an alias everywhere,
order, character cards. In Dactile Light the same panel has `m` to merge two
characters, `b` to split one by the scenes you pick, `g` for a new group.

## Lines and directions

| Type | You get |
|---|---|
| `\se ` | A line for SELİN; an unknown alias opens the new-character form |
| `\sedo ` | A direction, `(Selin …)`, with the verbs this play uses most |
| `\` | The suggestion menu: characters (those in the scene first), commands, macros |
| `SELİN:` at a line's start | A line for SELİN, under any of the names |
| Tab on an empty line | Who speaks next, shown faint: the one before the last speaker |
| `((` | A direction inside a line, `\ic{…}` |
| Ctrl+V at a line's start | A pasted draft: `NAME: …` become lines, `(…)` directions, `#…` headings |

A line runs on over the source lines under it, up to a blank line. Each source
line is a line in print too, so verse and songs need nothing extra.

## Headings

A `#` at a line's start opens the headings menu:

| Type | What it does |
|---|---|
| `#perde Kar Yağarken` | An act |
| `#sahne İç, Değirmen — Akşam` | A numbered scene |
| `#önsahne Place` | An unnumbered prologue; the next `#sahne` is still 1 |
| `#önsöz` | A preface, on a page of its own |
| `#notlar` | Notes for the director, on a page of their own |

Who appears in a scene gathers under its heading by itself.

## The look

**F6** changes the theme: paper, sepia, night, high contrast, dark. In Dactile,
**Surprise me** under Settings makes new colours at every press. The accent, font,
size, line spacing and page width are there too.

**F7** is the A4 page view: sheets break where the PDF breaks.

## Keys

In both apps:

| Key | |
|---|---|
| Ctrl+S | Save (it saves by itself anyway) |
| F2 | All of the source |
| F3, F4, F8 | Typewriter, scene focus, dim the rest |
| F5 | Build the PDF |
| F6, F7 | Theme, page view |
| F9 | Casting, table, numbers |
| Ctrl+G | The board |
| Ctrl+K | Characters and commands |
| Ctrl+N, Alt+N | Notes panel; a note on the selection |
| Ctrl+O | The cast |
| Alt+E | The scene's summary (never printed) |
| Alt+B, Alt+J | Split the scene at the cursor; join it with the next |
| Alt+S | Spelling suggestions |

In Dactile only: **F12** the PDF panel, **Ctrl + / − / 0** page zoom, **Ctrl+F1**
fold the ribbon, **Alt** ribbon letters.

In Dactile Light only: **F10** the ribbon, **Ctrl+P** the command palette, **F1**
a keyboard that shows what each key does.

## The start screen

In Dactile, **Shift** and **Ctrl** choose several plays. A **right click** on a play
gives open, rename, copy, new draft, show in the file manager and trash. For a
book, its PDF is there too.

In Dactile Light's list, `d` copies, `f` starts a new draft, F2 renames, `p` opens
the PDF. Del pressed twice trashes, `u` brings back. `?` lists every key.

Drafts stand next to the play: `kar-masali-0.2.tex`.
