# The tiyatro class

tiyatro is the class that sets plays with XeLaTeX. Dactile carries it inside itself;
`dactile-light install` also copies it into your TeX tree, so plays compile from any
folder. This page covers every command in the class.

## The shape of a play

```latex
\documentclass[draft]{tiyatro}      % 1. options

\title{Kar Masalı}                  % 2. cover details
\author{Örnek Yazar}

\karakter*[Selin][Köy öğretmeni]{SELİN}{se}   % 3. characters
\karakter[Değirmenci]{OZAN}{oz}

\begin{document}
\maketitle                          % the cover
\kisiler                            % the cast table

#perde Kar Yağarken                 % 4. the play
#sahne İç, Değirmen — Akşam

\se Amca, yine mi taş durdu?
\oz Durdu kızım.
\end{document}
```

## Class options

Options go in the brackets of `\documentclass[…]{tiyatro}`, separated by commas:
`\documentclass[final,left]{tiyatro}`.

| Option | What it does |
|---|---|
| `draft` | Sans, 1.5× spacing, line numbers in the margin, "TASLAK" on the cover. The default |
| `final` | Serif, single spacing, no numbers, the starred characters on the cover |
| `center`, `orta` | Scene headings in the middle. The default |
| `left`, `sol` | Scene headings on the left |
| `right`, `sag` | Scene headings on the right |
| `nohash` | Turns `#` lines off; `#` is an ordinary character |
| `nonotes` | Leaves the director's notes out of this build |

You don't have to type the side. In Dactile, **Play › Heading side** goes through
middle, left and right; in Dactile Light the "Scene headings" field of the **Alt+I**
form does the same. The writer shows the side too.

## Cover details

These go before `\begin{document}`. All are optional.

| Command | What it does | Example |
|---|---|---|
| `\title{…}` | The play's title | `\title{Kar Masalı}` |
| `\author{…}` | The author | `\author{Örnek Yazar}` |
| `\draft{…}` | The draft number | `\draft{0.1}` |
| `\copyrightnotice{…}` | The copyright line | `\copyrightnotice{2026 Örnek Yazar}` |
| `\address{…}` | An address | `\address{İstanbul}` |
| `\email{…}` | An email address | `\email{ornek@example.com}` |
| `\maketitle` | Prints the cover, after `\begin{document}` | |

## Characters

Each character is declared once, before `\begin{document}`. The last two braces are
always the same: the **name** over the lines and the **alias** you type.

### `\karakter`

| Form | In the cast table | Name in the table |
|---|---|---|
| `\karakter{SELİN}{se}` | No | |
| `\karakter[Köy öğretmeni]{SELİN}{se}` | Yes, with the description | SELİN |
| `\karakter[Selin][Köy öğretmeni]{SELİN}{se}` | Yes, with the description | Selin |

With no brackets the character stays out of the table. One pair is the description; with
two, the first is the full name and the second the description. `[]` is an empty
description: the character is in the table with nothing beside the name.

`\karakter*` marks a lead: stressed in the table, and on the `final` cover.

### Groups

```latex
\karaktergrubu[Köylüler]{
  \gkarakter[Birinci Kadın]{KADIN}{ka}
  \gkarakter{ÇOBAN}{co}
}
```

| Command | What it does |
|---|---|
| `\karaktergrubu[Description]{…}`, `\chargroup` | A group, gathered on one line of the table |
| `\gkarakter*[Full name]{NAME}{alias}`, `\gcharacter` | A member; written only inside a group |

### Other character commands

| Command | What it does | Example |
|---|---|---|
| `\gizlikarakter{NAME}{alias}` | Speaks, but never appears in the table | `\gizlikarakter{HERKES}{hep}` |
| `\digerkisiler{…}`, `\othercast` | Writes those named together under the table | `\digerkisiler{Köylüler, müzisyenler.}` |
| `\setinlinename{alias}{Name}` | The name `\oz` gives inside a line | `\setinlinename{oz}{Ozan Usta}` |
| `\setmaxnamewidth{length}` | The width of the names' column | `\setmaxnamewidth{3cm}` |
| `\kisiler` | Prints the cast table, after `\begin{document}` | |

## What each alias gives you

`\karakter{SELİN}{se}` makes four commands:

| Command | Where | What it does |
|---|---|---|
| `\se Text` | At a line's start | A line for SELİN |
| `\se` | Inside a line | The character's name: "Selin" |
| `\sedo Text` | At a line's start | A direction: (Selin Text). Ends at the end of the line |
| `\sefont` | Anywhere | The font of the character's name |

Directions chain: `\sedo\ozdo Birbirlerine bakarlar.` gives (Selin, Ozan Birbirlerine
bakarlar.)

## How lines run

- A line runs on over the lines under it, up to a blank line or a line starting with a
  command or a `#`.
- Each source line is a line in print; verse and songs need nothing extra.
- A `%` at the end of a line joins the next one to it.
- A direction ends where its source line ends.
- Inside a line, `_word_` is underlined.

## Directions

| Command | What it does | Example |
|---|---|---|
| `\ic{…}` | A direction inside a line, in parentheses and italics | `\ic{laughing}` gives *(laughing)* |
| `\action{…}` | An action paragraph between lines | `\action{Işıklar yavaşça açılır.}` |
| `\stagedir{…}` | A centred stage direction | `\stagedir{Karlı bir dağ köyü.}` |
| `\middir{…}` | An italic direction, set to the right | `\middir{Kapı çalar.}` |
| `\beat` | Prints "(Sus.)" | |
| `\girer{se,oz}` | An entrance: (SELİN ile OZAN girerler.) | |
| `\cikar{se}` | An exit: (SELİN çıkar.) | |
| `\dipnot{…}`, `\footnote{…}` | A footnote, in lines and directions | |

## Parts

Every part has two forms. A `#` line is short and ends at the end of the line or at a `%`
comment. The command form works under `nohash` too.

| `#` line | Command | What it does |
|---|---|---|
| `#perde Title`, `#act` | `\perde[Title]`, `\act` | A numbered act |
| `#sahne Place — Time`, `#scene` | `\sahne[Place — Time]`, `\scene` | A numbered scene |
| `#sahne[2.1] Place` | `\sahne[Place][2.1]` | A scene numbered by hand |
| `#önsahne Place`, `#prologue` | `\onsahne[Place]`, `\prologue` | An unnumbered prologue; the next scene is still 1 |
| `#önsöz`, `#preface` | `\onsoz[Title]` | A preface on a page of its own |
| `#notlar`, `#notes` | `\notlar[Title]` | Notes for the director on a page of their own |

`\#` prints a `#`. Who is in a scene is listed under its heading by itself.

## Other languages

Language commands such as `\ru{…}` set text in another script.
`\yenidil{jp}{Noto Serif CJK JP}` adds `\jp{…}` in that font.

## Changing the look

Change these with `\renewcommand` and `\setlength`, before `\begin{document}`.

| What | Commands |
|---|---|
| Fonts | `\namefont`, `\starfont`, `\actionfont`, `\stagedirfont`, `\castfont`, `\caststarfont`, `\casttitlefont`, `\sectiontitlefont`, `\scenetitlefont`, `\scenecastfont`, `\acttitlefont`, `\linenumberfont` |
| Spacing | `\beforesceneskip`, `\aftersceneskip`, `\beforeactskip`, `\afteractskip`, `\linenumbersep` |
| Cast page | `\beforecasttitleskip`, `\aftercasttitleskip`, `\castpageleftmargin`, `\castpagerightmargin`, `\castnamewidth` |
| Words | `\casttitlename`, `\prefacename`, `\notesname`, `\acttitlename`, `\scenetitlename`, `\prologuename`, `\curtainlocalizedtext`, `\scriptendlocalizedtext`, `\draftlocalizedtext`, `\beattext` |
| Entrances and exits | `\entertext`, `\enterpluraltext`, `\exittext`, `\exitpluraltext`, `\andtext` |
| Separators | `\actseparator`, `\sceneseparator` |

```latex
\renewcommand\scenetitlename{Tablo}
\setlength\aftersceneskip{2\baselineskip}
```

## Commands Dactile writes

You don't need to type these; Dactile uses them for actor and notes copies.

| Command | What it does |
|---|---|
| `\tiyatrocopy{se,oz}{Actor}` | Those roles' lines in bold, the rest faint |
| `\tiyatrocopycover{…}` | Adds to the copy's cover |
| `\tiyatromark` | Makes the next direction bold |
| `\tiyatrogap{n}` | Leaves n lines' room for notes |
| `\tiyatronoend` | Drops "Perde Sonu" and "Son" at the end |
| `\tiyatronotes` | Widens the right margin for notes |
| `\tiyatronote{RRGGBB}{Who}{n}{text}` | Puts note n beside its line |
| `\tiyatronoted{RRGGBB}{words}` | Colours what a note is about |
| `\tiyatronoteslist{Title}{…}` | Lists all the notes at the end |
| `\tiyatronoteitem{…}`, `\tiyatronotereply{…}` | A note and a reply in that list |

## Compiling

```sh
xelatex oyun.tex
```

Who is in each scene comes from the `.aux`. Dactile writes it itself, so one run is enough.
After editing elsewhere the class warns "Rerun to get them right", and a second run
fixes it.
