The third beta of Dactile 0.2, a terminal app for writing stage plays in the
syntax of the tiyatro XeLaTeX class. Linux x86_64 only for now.

```sh
tar xzf dactile-*-x86_64-linux.tar.gz
cd dactile-*-x86_64-linux
./dactile install
```

Run `dactile install` again after updating: the class is new too (1.4.0).

## New since 0.2.0-beta.2

**Casting (F9)**
- Full screen, three views on Tab: the casting, the scene/role table and the
  numbers.
- An actor per role, ⏎ to write one in. A play can have several castings,
  an A and a B lineup: ←→ between them, `n` a new one, `r` renames, `x`
  deletes. Actors' names stay in the sidecar; the play's PDF never shows
  them.
- The table: scenes down, roles across, ● where a role speaks, ○ where it is
  on stage without a line. `x` writes it as CSV, `p` as a landscape PDF.
- The numbers: lines, words, time and share by role, scene and act lengths;
  `w` sets the pace in words a minute.

**Actor copies**
- `k` on F9 builds a copy for each actor, and for each role nobody is cast in
  yet (`ROLE.pdf`). Three kinds: 1 full, 2 rehearsal, 3 rehearsal light.
- The role's lines in bold, the rest faint, its entrances and exits marked,
  the scenes without it down to their headings, and a cover with its scenes,
  pages and how much it says.
- A rehearsal copy keeps who speaks when but leaves the others' words out,
  with room for notes where they were; the light one keeps the first one to
  three words of each, the same on every build.

**Memorizing and the role view**
- `z` on F9, or "Memorize" on ^P: the cues before each of the role's lines,
  the line hidden; Space shows its first letters, Space again all of it.
- Shift+F8 lights the cursor's role and fades the rest.

**Groups in the cast panel (^O)**
- `g` a new group, `t` moves a character into a group or out of it, `n` on a
  group a new member, `e` renames it, `d` dissolves it into single
  characters, Alt+↑↓ reorders its members. No more writing
  `\karaktergrubu` by hand.
- In the writer every speaker's name is bold alike, group members and hidden
  characters too.

**The class (1.4.0)**
- Speakers' names print in capitals, in a name column as wide as the widest
  of them.
- `#önsahne Place` (also `#prologue`, `\onsahne[Place]`): an unnumbered
  "Ön Sahne" before an act's first scene; the next `#sahne` is still
  Scene 1.
- For actor copies: `\tiyatrocopy`, `\tiyatromark`, `\tiyatrocopycover`,
  `\tiyatrogap`, `\tiyatronoend`.

It is a beta: keep a copy of anything precious, and report what breaks.

---

Dactile 0.2'nin üçüncü betası. Başlıca yenilik oyuncular. F9 tam ekranda
dağıtımı, sahne/rol tablosunu ve sayıları açıyor. Bir oyunda birden çok
dağıtım olabiliyor. Tablo CSV ya da yatay PDF olarak alınabiliyor. `k` her
oyuncu için, oyuncusu henüz olmayan roller için de bir kopya çıkarıyor:
tam, prova (ötekilerin sözleri yerine not boşluğu) ya da hafif prova (ilk
birkaç kelime). `z` ile rol ezberleniyor, Shift+F8 imlecin rolünü öne
çıkarıyor. ^O kişiler panelinde gruplar artık elle yazılmadan kuruluyor,
taşınıyor ve dağıtılıyor. Sınıfta konuşan adları büyük harfle basılıyor.
`#önsahne` ile numarasız bir ön sahne yazılabiliyor. Güncelledikten sonra
`dactile install`'u yeniden çalıştır: sınıf da yeni (1.4.0). Ayrıntılar
README.tr.md dosyasında.
