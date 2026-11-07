# tiyatro sınıfı

Dactile sınıfı içinde taşır. `dactile-light install` onu TeX ağacına da kopyalar,
böylece oyunlar her klasörden derlenir.

```latex
\documentclass[draft]{tiyatro}

\title{Kar Masalı}
\author{Örnek Yazar}
\draft{0.1}

\karakter*[Selin][Köy öğretmeni]{SELİN}{se}

\begin{document}
\maketitle
\kisiler

#perde
#sahne Meydan

\se Geldim.
\end{document}
```

## Sınıf seçenekleri

| Seçenek | |
|---|---|
| `draft` | Sans yazı, 1,5 aralık, kenarda satır numarası, kapakta "TASLAK" |
| `final` | Serif yazı, tek aralık, kapakta yıldızlı kişiler |
| `nohash` | `#` sıradan karakter olur |
| `nonotes` | Yönetmene notları kopyadan çıkarır |

## Kişiler

| Komut | |
|---|---|
| `\karakter*[Tam ad][Açıklama]{AD}{kısaltma}` | Kişi; `*` kapakta gösterir |
| `\karaktergrubu[Açıklama]{…}`, `\chargroup` | Grup |
| `\gkarakter*[Tam ad]{AD}{kısaltma}`, `\gcharacter` | Gruptaki kişi |
| `\gizlikarakter{AD}{kısaltma}` | Kişi tablosunda görünmez |
| `\digerkisiler{…}`, `\othercast` | Listenin altındaki toplu kişiler |
| `\setinlinename{kısaltma}{Ad}` | Replik içinde kullanılan ad |

Her kişi şu komutları kazanır: `\se` replik, `\sedo` yönerge.

## Yönergeler

| Komut | |
|---|---|
| `\ic{…}` | Replik içinde yönerge |
| `\action{…}` | Eylem paragrafı |
| `\stagedir{…}`, `\middir{…}` | Sahne yönergeleri |
| `\beat` | Kısa duraklama |
| `\girer{se,oz}`, `\cikar{se}` | Giriş ve çıkış: (SELİN ile OZAN girerler.) |
| `\dipnot{…}`, `\footnote{…}` | Dipnot |

## Bölümler

| `#` satırı | Komut |
|---|---|
| `#perde Başlık`, `#act` | `\perde[…]`, `\act` |
| `#sahne Yer — Zaman`, `#scene` | `\sahne[…]`, `\scene` |
| `#sahne[2.1] Yer` | Numarayı elle verir |
| `#önsahne Yer`, `#prologue` | `\onsahne[…]`, `\prologue` |
| `#önsöz`, `#preface` | `\onsoz` |
| `#notlar`, `#notes` | `\notlar` |

`\#` bir `#` basar. `#` satırı kaynak satırının sonunda ya da bir `%` yorumunda
biter. Kişi tablosunu `\kisiler` basar.

## Replik kuralları

- Replik altındaki satırlara, boş satıra ya da komutla, `#` ile başlayan satıra kadar sürer.
- Her kaynak satırı baskıda da ayrı satırdır.
- Satır sonundaki `%` sonraki satırı buna ekler.

## Derleme

```sh
xelatex oyun.tex
```

Sahne başlığındaki kişi listesi `.aux` dosyasından gelir. Dactile bunu kendi yazar,
tek geçiş yeter. Başka yerde düzenlediysen sınıf "Rerun to get them right" der,
ikinci geçiş düzeltir.
