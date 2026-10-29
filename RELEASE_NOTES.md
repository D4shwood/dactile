The first beta of Dactile 0.2, a terminal app for writing stage plays in the
syntax of the tiyatro XeLaTeX class. Linux x86_64 only for now.

```sh
tar xzf dactile-*-x86_64-linux.tar.gz
cd dactile-*-x86_64-linux
./dactile install
```

Run `dactile install` again after updating: the class is new too (1.2.0).

## New since 0.1.0-beta.3

**The PDF, from inside the app**
- F5 builds the PDF next to the play, in the background, in one XeLaTeX run:
  Dactile writes who is in each scene into the `.aux` first, so the scene
  headings are right the first time. A plain `xelatex` by hand is right in one
  run too once Dactile has saved the play.
- The class (1.2.0) says "Rerun to get them right" when a heading showed
  another cast than the scene had; the build then runs once more by itself.
- Alt+O opens the PDF, `p` on the home screen too, "Open the PDF" on ^P; it
  never opens by itself. If a build fails, Alt+L opens its log.
- `dactile pdf <play>` builds from the shell.
- Dim the rest moved from F5 to F8.

**No build errors**
- What would stop XeLaTeX is underlined faintly as you write; the cursor on it
  says what in the status line.
- `&`, `$`, `^`, a stray `_` and a `%` before a number (`%50`) are escaped
  quietly on save.
- A brace left open or one too many, or a command nobody defined, stops F5 at
  its place (`\sein` → "write `\se{}in`"); F5 again builds anyway.
- `dactile check <play>` lists them without changing anything.

**The home screen**
- The chosen play's first page stands beside the list, as tall as the screen,
  laid out as the writer lays it out, in its theme.
- Under the list: the play as a strip of its scenes, the scenes with their
  times, and who speaks how much.
- Drafts stack upward as a tree, the newest on top, and the screen opens on
  the play worked on last.
- F2 renames a play in its row, as a file manager does; its files follow and a
  draft keeps its number. Shift+F2 gives the title and file name apart.
- Del twice sends a play to the desktop's trash; `u` brings it back.
- `?` shows every key; the key bar keeps the common ones.

It is a beta: keep a copy of anything precious, and report what breaks.

---

Dactile 0.2'nin ilk betası. Başlıca yenilikler: PDF artık uygulamanın içinden
F5 ile, arka planda ve tek XeLaTeX geçişinde derleniyor (Alt+O açar, Alt+L
logu açar); XeLaTeX'i durduracak yerler yazarken soluk çizgiyle gösteriliyor,
basit olanlar kayıtta sessizce düzeltiliyor; ana ekranda seçili oyunun ilk
sayfası, sahne şeridi ve kim ne kadar konuşuyor; taslaklar yukarı doğru ağaç;
F2 ile satırda ad değiştirme; çöp kutusu. "Gerisini soldur" F5'ten F8'e
taşındı. Güncelledikten sonra `dactile install`'u yeniden çalıştır: sınıf da
yeni (1.2.0). Ayrıntılar README.tr.md dosyasında.
