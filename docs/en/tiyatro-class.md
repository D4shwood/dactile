# The tiyatro class

Dactile carries the class inside itself. `dactile-light install` also copies it into
your TeX tree, so plays compile from any folder.

```latex
\documentclass[draft]{tiyatro}

\title{Kar Masalı}
\author{Örnek Yazar}
\draft{0.1}

\karakter*[Selin][Köy öğretmeni]{SELİN}{se}

\begin{document}
\maketitle
\kisiler

#perde
#sahne Meydan

\se Geldim.
\end{document}
```

## Class options

| Option | |
|---|---|
| `draft` | Sans, 1.5× spacing, line numbers in the margin, "TASLAK" on the cover |
| `final` | Serif, single spacing, the starred characters on the cover |
| `nohash` | `#` is an ordinary character again |
| `nonotes` | Leaves the director's notes out of a copy |

## Characters

| Command | |
|---|---|
| `\karakter*[Full name][Description]{NAME}{alias}` | A character; `*` puts them on the cover |
| `\karaktergrubu[Description]{…}`, `\chargroup` | A group |
| `\gkarakter*[Full name]{NAME}{alias}`, `\gcharacter` | A member of a group |
| `\gizlikarakter{NAME}{alias}` | Left out of the cast table |
| `\digerkisiler{…}`, `\othercast` | Those under the list, named together |
| `\setinlinename{alias}{Name}` | The name used inside a line |

Each character gains these commands: `\se` a line, `\sedo` a direction.

## Directions

| Command | |
|---|---|
| `\ic{…}` | A direction inside a line |
| `\action{…}` | An action paragraph |
| `\stagedir{…}`, `\middir{…}` | Stage directions |
| `\beat` | A beat |
| `\girer{se,oz}`, `\cikar{se}` | Entrances and exits: (SELİN ile OZAN girerler.) |
| `\dipnot{…}`, `\footnote{…}` | A footnote |

## Parts

| `#` line | Command |
|---|---|
| `#perde Title`, `#act` | `\perde[…]`, `\act` |
| `#sahne Place — Time`, `#scene` | `\sahne[…]`, `\scene` |
| `#sahne[2.1] Place` | Gives the number by hand |
| `#önsahne Place`, `#prologue` | `\onsahne[…]`, `\prologue` |
| `#önsöz`, `#preface` | `\onsoz` |
| `#notlar`, `#notes` | `\notlar` |

`\#` prints a `#`. A `#` line ends at the end of the source line or at a `%` comment.
`\kisiler` prints the cast table.

## How lines run

- A line runs on up to a blank line, or a line starting with a command or a `#`.
- Each source line is a line in print.
- A `%` at the end of a line joins the next one to it.

## Compiling

```sh
xelatex oyun.tex
```

Who is in each scene comes from the `.aux`. Dactile writes it itself, so one run is
enough. After editing elsewhere the class warns "Rerun to get them right", and a
second run fixes it.
