The second beta of Dactile, a terminal app for writing stage plays in the
syntax of the tiyatro XeLaTeX class. Linux x86_64 only for now.

```sh
tar xzf dactile-*-x86_64-linux.tar.gz
cd dactile-*-x86_64-linux
./dactile install
```

## Fixed since beta.1

- On a TeX install without Latin Modern, the class failed to load its
  fallback font (Liberation Serif) unless the font sat in the TeX tree. It now
  finds Liberation Serif wherever the system has it. Run `dactile install`
  again to update the class in `~/texmf`.

## In this beta

- Live macros: a character's alias becomes their line, `\sahne` and `\perde`
  open numbered scenes and acts, and each scene gathers its cast.
- The `\` suggestion menu, the new-character form, directions with a verbs
  list, find and replace.
- The cast panel (rename an alias everywhere, order, character cards) and the
  scene navigator with running times.
- Turkish spell checking with a Hunspell dictionary.
- Typewriter, scene focus and dimming; margin numbers that match the draft PDF.
- Autosave, a lock against a second instance, and a history of earlier
  versions.
- A home screen, first-run setup and settings, in Turkish and English.
- The tiyatro class 1.0.0, rewritten, with draft and final modes.

It is a beta: keep a copy of anything precious, and report what breaks.

---

Dactile'ın ikinci betası. beta.1'den bu yana: Latin Modern'i olmayan TeX
kurulumlarında sınıf yedek fontu (Liberation Serif) bulamıyordu; artık
sistemde nerede kuruluysa buluyor. Sınıfı güncellemek için `dactile install`'u
yeniden çalıştır. Kurulum yukarıdaki gibi; ayrıntılar README.tr.md dosyasında.
