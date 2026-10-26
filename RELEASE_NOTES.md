The third beta of Dactile, a terminal app for writing stage plays in the
syntax of the tiyatro XeLaTeX class. Linux x86_64 only for now.

```sh
tar xzf dactile-*-x86_64-linux.tar.gz
cd dactile-*-x86_64-linux
./dactile install
```

Run `dactile install` again after updating: the class is new too (1.1.0).

## New since beta.2

**The class (1.1.0)**
- Headings as `#` lines: `#perde Title`, `#sahne Place — Time`,
  `#sahne[2.1] Place`, `#önsöz`, `#notlar` (and `#act`, `#scene`, `#preface`,
  `#notes`). `\perde[…]` and `\sahne[…]` work as before; `\#` prints a `#`.
- A preface (`\onsoz`) and notes for the director (`\notlar`), each on a page
  of its own. Options: `nohash`, and `nonotes` to leave the notes out of a copy.
- A play with nobody in its cast yet compiles (it did not, so new plays failed).

**The page**
- The play on a paper page in the middle of the screen; five themes on F6:
  paper, sepia, night, high contrast, dark.
- The cover and the cast table drawn as the PDF prints them; prefaces, notes,
  old `\section*` headings and `\clearpage` too; the curtain at each act's end.
- F7: page view, A4-shaped sheets broken where the PDF breaks, numbered.
- On a wide screen, the play's outline in the margin; a click goes there.

**Writing**
- `#` at a line's start opens the headings; `SELİN:` makes a line for SELİN;
  Tab takes the faint suggestion of who speaks next; a pasted draft takes the
  play's shape (`NAME: …` lines, `(…)` directions, `#…` headings).
- ^G: mark acts and scenes, hide them (as `%~` lines, out of the PDF), throw
  them in the play's trash, put them back, or destroy them (a copy stays in
  the history).
- ^O: merge two characters, or split one, giving the scenes you pick to a new
  character.
- ^P: the command palette.
- F1: a keyboard; press a key or point at it to see what it does.
- Forms escape `& % $ # _` in what they write (a `&` on the cover broke the
  PDF before).

**Around it**
- The home screen: the plays beside a card of the chosen one; `d` copies a
  play, `f` starts a new draft of it.
- Animations: gliding scroll, headings typing themselves out, windows
  unrolling, messages fading. A setting turns them off.

It is a beta: keep a copy of anything precious, and report what breaks.

---

Dactile'ın üçüncü betası. Başlıca yenilikler: `#sahne`, `#perde`, `#önsöz`,
`#notlar` başlıkları; kâğıt sayfa ve beş tema; A4 sayfa görünümü (F7); kenar
haritası; sahneleri gizleme, çöp kutusu ve yok etme (^G); karakter birleştirme
ve bölme (^O); komut paleti (^P); klavyeli yardım (F1); ana ekranda kopya ve
yeni taslak; animasyonlar. Güncelledikten sonra `dactile install`'u yeniden
çalıştır: sınıf da yeni (1.1.0). Ayrıntılar README.tr.md dosyasında.
