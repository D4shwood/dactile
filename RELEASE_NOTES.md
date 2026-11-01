The fourth beta of Dactile 0.2, a terminal app for writing stage plays in the
syntax of the tiyatro XeLaTeX class. Linux x86_64 only for now.

```sh
tar xzf dactile-*-x86_64-linux.tar.gz
cd dactile-*-x86_64-linux
./dactile install
```

No new features: this one is a review of the code, for speed and safety.
The class is unchanged (1.4.0).

## New since 0.2.0-beta.3

**Faster**
- PDF builds take about half the time: XeLaTeX no longer searches with
  METAFONT for fonts the class tries but which are not installed.
- Typing in a long play lays out again only the blocks that changed, about
  twice as fast as before.
- The casting (F9) and the memorizing cues are worked out once per edit
  instead of on every frame, and marking misspelt words no longer copies
  the play each frame.

**Safer**
- A save waits until the play is on the disk, the folder too, so a power
  cut right after it cannot leave the file empty.
- A play opened through a symbolic link is saved through the link, not
  instead of it.
- Undo keeps only what each step changed instead of a copy of the whole
  play: on a long play its memory goes from tens of megabytes to a few
  kilobytes.

**Inside**
- The writer, one file of 5,100 lines, is now a dozen files, one per part.

It is a beta: keep a copy of anything precious, and report what breaks.

---

Dactile 0.2'nin dördüncü betası. Yeni özellik yok, kodun hız ve güvenlik
için gözden geçirilmiş hâli. PDF derlemesi yaklaşık iki kat hızlı. Uzun bir
oyunda yazarken yalnız değişen bloklar yeniden diziliyor. F9 ile ezber
ekranının verisi her düzenlemede bir kez hesaplanıyor. Kaydetme, dosya
diske yazılana kadar bekliyor. Bağlantı (symlink) üzerinden açılan oyunlar
bağlantı bozulmadan kaydediliyor. Geri alma artık her adımda yalnız
değişen parçayı tutuyor. Sınıf değişmedi (1.4.0).
