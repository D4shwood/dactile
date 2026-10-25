# Dactile — Dynamic Stagewright

[English](README.md)

> Geliştirme aşamasında. Henüz kurulacak bir şey yok; bu sayfa nereye gittiğini anlatıyor.

Tiyatro oyunu yazmak için bir terminal uygulaması. Oyunu **tiyatro** XeLaTeX
sınıfının sözdizimiyle yazarsın; makrolar yazarken canlanır: `\ma Merhaba`
anında `MARİA : Merhaba` olur, `\sahne` numaralı bir sahne açar, sahnede
görünen karakterler kendiliğinden sahne başlığının altında toplanır.

Dosya düz bir `.tex` olarak kalır ve XeLaTeX ile derlenir. Dactile'da yazılan
bir oyun her TeX düzenleyicisinde açılmaya devam eder.

```latex
\karakter*[Selin][Köy öğretmeni]{SELİN}{se}
\karakter[Ozan Usta][Değirmenci]{OZAN}{oz}

\perde[Kar Yağarken]
\sahne[İç, Değirmen — Akşam]

\se Amca, yine mi taş durdu?
\oz Durdu kızım. \ic{elini taşa koyar} Soğuk her şeyi durdurur.
\sedo\ozdo Birbirlerine bakarlar.
```

Tüm özellikler için: [examples/ornek-oyun.tex](examples/ornek-oyun.tex).

## tiyatro sınıfı

Sınıf [tex/](tex/) klasöründe; Dactile onu kendi içinde taşır.

- **Karakterler** kısaltmalarla tanımlanır:
  - `\karakter`
  - gruplar (`\karaktergrubu`)
  - kişiler tablosuna girmeyenler (`\gizlikarakter`)
  - replik içinde geçen ad (`\setinlinename`)
- **Replik** kaynak satırın sonuna kadar sürer; ad kendi sütununda durur.
- **Yönergeler:** `\aliasdo` zincirleri, `\action`, `\stagedir`, `\middir`, `\ic`, `\beat`.
- **Perdeler ve sahneler** otomatik numaralanır. Sahne başlığında o sahnede kimlerin olduğu yazar; bunun için iki derleme gerekir.
- **Kişiler tablosu** (`\kisiler`) ve kapak.
- **Taslak ve final modları:**

  | Mod | Font | Satır aralığı | Kenar | Kapak |
  |---|---|---|---|---|
  | taslak | sans | 1,5 | her repliğe ve yönergeye bir numara | "TASLAK" |
  | final | serif | tek | yok | yıldızlı karakterler |

```sh
xelatex oyun.tex && xelatex oyun.tex
```

Sınıf [tiyatro-latex](https://github.com/D4shwood/tiyatro-latex)'ten doğdu ve
Dactile için yeniden yazıldı. Karakterleri komut olarak tanımlama fikri Davide
Fauri'nin [theatre.cls](https://github.com/DavideFauri/theatre) sınıfından
geliyor; onun kodu burada kullanılmıyor.

## Yol haritası

Kilometre taşları halinde planlandı; ayrıntılar [TODO.md](TODO.md) dosyasında.

## Lisans

Uygulama da sınıf da GPL-3.0-or-later. Bkz. [LICENSE](LICENSE).
