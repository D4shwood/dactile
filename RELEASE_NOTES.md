The sixth beta of Dactile 0.2, a terminal app for writing stage plays in the
syntax of the tiyatro XeLaTeX class. Linux x86_64 only for now.

```sh
tar xzf dactile-*-x86_64-linux.tar.gz
cd dactile-*-x86_64-linux
./dactile install
```

## New since 0.2.0-beta.5

**Notes beside the play**
- ^N shows notes on the right, from anyone: an AI, a workshop, a reader,
  yourself. They live in `oyun.notlar.md` beside the play, a Markdown task
  list you can edit by hand.
- Paste a reply with numbered items (`139: …`), read a file, or write a note
  on the selection (Alt+N). Each note finds its place again by what it
  quotes, underlined in its author's colour.
- Mark notes done, hide, archive, reply, edit. `u` makes a
  `“wrong” → right` correction, changing only the words that differ, and
  never one that would break the play; a note whose quote is gone looks
  fixed.
- A PDF copy with the open notes in the margin and a list of all of them at
  the end (class 1.6.0).

**A ribbon in the writer**
- Along the top, as in a word processor: Dosya, Giriş, Ekle, Görünüm, Oyun,
  Notlar, Yardım, each tab's buttons in groups with their keys.
- F10 shows a letter on each tab and then each button, so two letters do
  anything (F10 E A puts in a stage direction); the arrows and the mouse
  work too. ^F1 folds it to its tabs.
- Ekle puts in acts, scenes, characters, directions, a pause, a preface,
  director's notes. Dosya opens a page with the play's details, the PDF,
  the copies and templates.

**A start screen and templates**
- The home screen has a column (Giriş, Yeni, Aç, Ayarlar, Tuşlar) and a row
  of tiles over the plays: an empty play, a one-act and a three-act
  skeleton, open or import, a copy of the example play. The mouse works.
- Four skeletons (one act, three acts, a monologue, a short sketch), and
  your own: Dosya › Save as a template keeps a play's cover, cast, acts and
  scenes, not its lines.

**Fixes**
- A brace left open spoils only its own block, not the rest of the play.

It is a beta: keep a copy of anything precious, and report what breaks.

---

Dactile 0.2'nin altıncı betası. Oyunun yanında notlar: ^N bir AI'dan,
atölyeden, okurdan ya da kendinden gelen notları sağda gösteriyor; notlar
oyunun yanındaki `oyun.notlar.md` dosyasında duruyor. `u` bir notun
`“yanlış” → doğru` düzeltmesini yapıyor, oyunu bozacak bir düzeltmeyi hiç
yapmıyor; notlu bir PDF kopyası da çıkıyor (sınıf 1.6.0). Yazıcıda Word'deki
gibi bir şerit var: sekmeler, gruplanmış düğmeler, F10 ile harf kısayolları,
fare; ^F1 katlıyor. Ana ekran bir açılış ekranı oldu: solda bir sütun,
oyunların üstünde yeni oyun kutucukları; dört hazır şablon ve "Şablon olarak
kaydet" ile kendi şablonların.
