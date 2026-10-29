The second beta of Dactile 0.2, a terminal app for writing stage plays in the
syntax of the tiyatro XeLaTeX class. Linux x86_64 only for now.

```sh
tar xzf dactile-*-x86_64-linux.tar.gz
cd dactile-*-x86_64-linux
./dactile install
```

Run `dactile install` again after updating: the class is new too (1.3.0).

## New since 0.2.0-beta.1

**The board**
- ^G opens a strip beside the writing: the acts and scenes, the cursor's
  scene lit and open with its summary. It steps aside when the screen is too
  narrow for the page and it, and comes back when there is room.
- ^G twice quickly opens the board, full screen: a list of acts and scenes, or
  (Tab) cards, the acts as columns, scrolling sideways. ⏎ goes to the scene,
  Esc back to the writing.
- Alt+↑↓ moves a scene a place, over an act's edge into the next; Alt+←→ on
  the cards to another act. `m` picks it up, the arrows choose, ⏎ puts it
  down. Acts move whole, the marked ones (␣) together. Each move is one undo.
- `j` joins a scene with the next: the second heading becomes a direction
  with its place, nothing lost. Alt+B in the writer splits a scene at the
  cursor's line, with the same place, the cursor on the new heading.
- `n` opens a new card, a hidden scene (`%~`) until ⏎ opens it to write in.
- `h` hides and `t` trashes as before; the trash is now on `c`.
- Before the first move, split or join of a session the play as it was is
  kept as a draft beside it; the writing goes on in the play itself.

**Summaries**
- A line or two per scene, never printed: `e` on the board, Alt+E in the
  writer, shown faint under the heading.
- They live in the sidecar, tied to the scene by a short id at the heading's
  end (`#sahne Meydan %@k3`) that the PDF never sees, so a summary follows
  its scene through moves, renames and edits outside Dactile.

**The class (1.3.0)**
- `\girer{se,oz}` prints "(SELİN ile OZAN girerler.)", `\cikar{se}` "(SELİN
  çıkar.)"; both are in the `\` menu, and the characters count as in the
  scene for its heading.
- A `%` comment on a `#` line now ends there instead of joining the next line
  to it.

It is a beta: keep a copy of anything precious, and report what breaks.

---

Dactile 0.2'nin ikinci betası. Başlıca yenilikler: pano. ^G yazının yanında
sahne şeridini açar, ^G iki kez hızlıca panoyu tam ekran açar (liste ya da
kartlar). Sahneler Alt+oklarla ya da `m` ile taşınıyor, Alt+B ile bölünüyor,
`j` ile birleştiriliyor; her taşıma tek geri al. Oturumdaki ilk taşımadan önce
oyunun eski hâli taslak olarak saklanıyor. Her sahneye PDF'e girmeyen bir özet
yazılabiliyor (panoda `e`, yazıcıda Alt+E). Sınıfta `\girer` ve `\cikar`
komutları var ve `#` satırındaki `%` yorumu artık o satırda bitiyor.
Güncelledikten sonra `dactile install`'u yeniden çalıştır: sınıf da yeni
(1.3.0). Ayrıntılar README.tr.md dosyasında.
