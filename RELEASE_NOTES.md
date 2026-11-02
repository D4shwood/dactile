The fifth beta of Dactile 0.2, a terminal app for writing stage plays in the
syntax of the tiyatro XeLaTeX class. Linux x86_64 only for now.

```sh
tar xzf dactile-*-x86_64-linux.tar.gz
cd dactile-*-x86_64-linux
./dactile install
```

## New since 0.2.0-beta.4

**Import a play**
- `i` on the home screen opens the desktop's file chooser (or asks for a
  path where there is none); `dactile import FILE` does it from the shell.
- Reads Word (.docx), OpenDocument (.odt), PDF with text in it, Markdown and
  plain text, with nothing else installed. PDFs go through `pdftotext` where
  it is installed, which keeps the layout better.
- Finds the speeches written as `NAME: text`, `NAME — text`, `NAME. text` or
  the name alone on its line; the cast list, acts and scenes, the title and
  the author; italics become stage directions. What it doesn't recognise
  stays as plain text.
- PDFs lose their running heads, page numbers and margin line numbers, and
  wrapped lines are joined again; short lines, as in verse, are kept.
- Before writing the play, a preview shows every name with its alias, its
  number of lines and its first line: rename it, change the alias, merge two
  names into one, or mark one as not a character.
- Scanned PDFs, which need OCR, are not read yet.

**Line breaks in print**
- The class (now 1.5.0) breaks a speech where its line ends in the source,
  as a word processor does with Enter. A blank line, a `#` heading or a
  command on the next line still ends the speech; `\\` at a line end is not
  doubled.

**Lost work and safety**
- Every save, the autosave too, looks at the disk just before: it never
  writes over a change made elsewhere that it hasn't seen, and a save over
  one keeps the other version in the history first.
- A save that fails on quit keeps the writer open with the play in it.
- Notes (`.dactile.toml`) that can't be read are moved aside, not saved
  over.
- XeLaTeX runs without shell escape; the lock between two Dactiles is made
  atomically and taken over with ^S twice.

It is a beta: keep a copy of anything precious, and report what breaks.

---

Dactile 0.2'nin beşinci betası. Artık oyun içe aktarılabiliyor: ana ekranda
`i` (ya da `dactile import DOSYA`) Word, OpenDocument, metinli PDF, Markdown
ve düz metin dosyalarını oyuna çeviriyor; ek bir şey kurmak gerekmiyor.
Replikler (`İSİM: metin`, `İSİM — metin`, `İSİM. metin`, ya da ad tek
satırda), kişiler listesi, perde ve sahneler, başlık ve yazar tanınıyor;
italikler yönerge oluyor. Yazmadan önce bir önizlemede adlar düzeltilebiliyor,
birleştirilebiliyor ya da "karakter değil" diye işaretlenebiliyor. Taranmış
PDF'ler (OCR) henüz okunmuyor. Sınıf 1.5.0: replik içindeki satır sonu,
Word'deki Enter gibi baskıda da satırı kırıyor. Kaydetme artık başka yerde
yapılmış bir değişikliğin üstüne yazmıyor; okunamayan notlar kenara alınıyor.
