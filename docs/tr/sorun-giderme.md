# Sorun giderme

## PDF çıkmıyor

Önce XeLaTeX'i çalıştırmadan neyin takılacağına bak:

```sh
dactile-light check oyun.tex
```

| Hata | Nedeni | Çözüm |
|---|---|---|
| `Undefined control sequence` | Satırın başındaki kısaltma tanımlı değil | Kısaltmayı düzelt ya da kişiyi ekle. Kısaltmalar için `dactile-light cast oyun.tex` |
| `Missing \begin{document}` | `\begin{document}` öncesinde, kişi listesinde bozuk bir satır | Hatanın gösterdiği satıra bak |
| PDF'te boş kalan harfler | Metinde bozuk karakter, `�` | Harfi metinde düzelt; çoğunlukla içe aktarmadan kalır |
| Sahne başlığında kişiler eksik | Oyun başka yerde düzenlendi | Bir kez daha derle |
| PDF hiç oluşmuyor | XeLaTeX kurulu değil | `texlive-xetex` ve `texlive-latexextra` kur |

Bir kısaltmayı oyunun her yerinde değiştirmek için kişiler panelini kullan.

## Oyun salt okunur açılıyor

Oyun başka bir Dactile penceresinde açıktır. Açık olanı kapat. Oyunun yanındaki
`.oyun.tex.dactile-lock` dosyası bunu gösterir. Çöken bir uygulamadan kalan kilit
sorun çıkarmaz.

## Oyunlarım görünmüyor

Ayarlar'daki oyun klasörüne bak. Ayarlar `~/.config/dactile/config.toml`
dosyasındadır, klasör `plays_dir` satırındadır.

## Eski bir sürüme dönmek

Eski sürümler `~/.local/share/dactile/history` altındadır. Silinen oyunlar
masaüstünün çöpüne gider.

## Yazım denetimi çalışmıyor

`tr_TR.aff` ve `tr_TR.dic` dosyaları `~/.local/share/hunspell` ya da
`/usr/share/hunspell` altında olmalı.

## Taranmış PDF kötü okunuyor

Önizlemede **OCR ile yeniden oku** düğmesini dene. Hâlâ kötüyse bölümleri Belge
sekmesinde elle işaretle.

## Dosyalar

| Dosya | Ne işe yarar |
|---|---|
| `oyun.tex` | Oyunun kendisi |
| `oyun.dactile.toml` | Kişi kartları, sahne özetleri, dağıtımlar, oyunun kendi sözlüğü |
| `oyun.notlar.md` | Notlar |
| `.oyun.tex.dactile-lock` | Oyun açıkken duran kilit |
| `oyun-0.2.tex` | Taslak |
| `oyun-kopyalar/` | Oyuncu kopyaları |
| `oyun-tablo.csv`, `oyun-tablo.pdf` | Sahne ve rol tablosu |
| `~/.config/dactile/config.toml` | Ayarlar |
| `~/.config/dactile/templates` | Kendi şablonların |

## Komut satırı

| Komut | |
|---|---|
| `dactile-light new "Ad"` | Yeni oyun |
| `dactile-light cast oyun.tex` | Kişileri listeler, bir şey değiştirmez |
| `dactile-light outline oyun.tex` | Perde ve sahneleri listeler |
| `dactile-light check oyun.tex` | XeLaTeX'i durduracakları listeler |
| `dactile-light pdf oyun.tex` | PDF derler, `--open` ile açar |
| `dactile-light import DOSYA` | İçe aktarır |
| `dactile-light book kitap.tex` | Kitabı tek PDF yapar |
