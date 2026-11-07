# tiyatro sınıfı

tiyatro, oyunları XeLaTeX ile dizen sınıftır. Dactile onu içinde taşır;
`dactile-light install` TeX ağacına da kopyalar, böylece oyunlar her klasörden
derlenir. Bu sayfa sınıfın bütün komutlarını anlatır.

## Bir oyunun iskeleti

```latex
\documentclass[draft]{tiyatro}      % 1. seçenekler

\title{Kar Masalı}                  % 2. kapak bilgileri
\author{Örnek Yazar}

\karakter*[Selin][Köy öğretmeni]{SELİN}{se}   % 3. kişiler
\karakter[Değirmenci]{OZAN}{oz}

\begin{document}
\maketitle                          % kapak
\kisiler                            % kişi tablosu

#perde Kar Yağarken                 % 4. oyun
#sahne İç, Değirmen — Akşam

\se Amca, yine mi taş durdu?
\oz Durdu kızım.
\end{document}
```

## Sınıf seçenekleri

`\documentclass[…]{tiyatro}` satırının köşeli parantezine, virgülle ayırarak yazılır:
`\documentclass[final,nonotes]{tiyatro}`.

| Seçenek | Ne yapar |
|---|---|
| `draft` | Taslak: final ile aynı serif yazı, 1,15 satır aralığı, kenarda satır numarası, kapakta "TASLAK". Varsayılan budur |
| `final` | Son hâl: serif yazı, tek aralık, numara yok, kapakta yıldızlı kişiler |
| `nohash` | `#` satırlarını kapatır, `#` sıradan karakter olur |
| `nonotes` | Yönetmene notları bu derlemeden çıkarır |

## Sahne başlıklarının hizası

Sınıf seçeneği değil, `\begin{document}` öncesinde kendi satırıdır:

```latex
\documentclass{tiyatro}
\scenetitleflush[left]
```

`left` (`sol`), `right` (`sağ`, `sag`) ya da `center` (`orta`) alır. Satır yoksa
başlıklar ortadadır. Birden çok yazılırsa sonuncusu geçerlidir, başka bir sözcük
derlemede hata verir.

Hizayı elle yazmak zorunda değilsin. Dactile'da **Oyun › Başlık hizası** düğmesi
orta, sol, sağ arasında geçer; Dactile Light'ta **Alt+I** formundaki "Sahne
başlıkları" alanı aynı işi yapar. Hiza yazı ekranında da görünür.

## Kapak bilgileri

`\begin{document}` öncesine yazılır. Hepsi isteğe bağlıdır.

| Komut | Ne yapar | Örnek |
|---|---|---|
| `\title{…}` | Oyunun adı | `\title{Kar Masalı}` |
| `\author{…}` | Yazar | `\author{Örnek Yazar}` |
| `\editedby{…}` | Yazarın altında "Düzenleyen: …" | `\editedby{Örnek Kişi}` |
| `\draft{…}` | Taslak numarası | `\draft{0.1}` |
| `\copyrightnotice{…}` | Telif satırı | `\copyrightnotice{2026 Örnek Yazar}` |
| `\address{…}` | Adres | `\address{İstanbul}` |
| `\email{…}` | E-posta | `\email{ornek@example.com}` |
| `\telephone{…}` | Telefon | `\telephone{000 000 00 00}` |
| `\playfont{…}` | Oyunun yazı tipi. Bilgisayarda yoksa uyarır, sınıfın yazı tipi kalır | `\playfont{Noto Serif}` |
| `\maketitle` | Kapağı basar; `\begin{document}` sonrasında | |

Boş bırakılan satır kapağa basılmaz.

### Kapak ve kişiler sayfasının görünümü

```latex
\coverstyle{title}{align=left, size=Huge, style=smallcaps, before=2cm, after=1cm}
\caststyle{title}{align=center, size=Large}
```

Kapakta `title`, `author`, `editedby`, `draft`, `starred`, `copyright`, `address`,
`telephone`, `email`; kişiler sayfasında `title`, `names`, `stars`, `descriptions`,
`others` ayarlanır. Yanlış ad ya da değer derlemeyi durdurur. Dactile'da Giriş
sekmesindeki **Kapak ve kişiler** bu satırları yazar.

`\kisiler` sonunda sayfa kapanır; `\kisiler*` yazılırsa oyun aynı sayfada başlar.
`\dramatispersonae`, kişiler başlığını oyunun dili ne olursa olsun Latince yapar.

## Kişiler

Her kişi `\begin{document}` öncesinde bir kez tanımlanır. Son iki süslü parantez her
zaman aynıdır: replik üstündeki **ad** ve yazarken kullanılan **kısaltma**.

### `\karakter`

| Biçim | Kişi tablosunda | Tablodaki ad |
|---|---|---|
| `\karakter{SELİN}{se}` | Hayır | |
| `\karakter[Köy öğretmeni]{SELİN}{se}` | Evet, açıklamayla | SELİN |
| `\karakter[Selin][Köy öğretmeni]{SELİN}{se}` | Evet, açıklamayla | Selin |

Köşeli parantez hiç yoksa kişi tabloda görünmez. Tek köşeli parantez açıklamadır, iki
tane varsa ilki tam ad, ikincisi açıklamadır. `[]` boş açıklamadır: kişi tabloda yer
alır, yanında bir şey yazmaz.

`\karakter*` başroldür: tabloda vurgulanır, `final` kapakta görünür.

### Gruplar

```latex
\karaktergrubu[Köylüler]{
  \gkarakter[Birinci Kadın]{KADIN}{ka}
  \gkarakter{ÇOBAN}{co}
}
```

| Komut | Ne yapar |
|---|---|
| `\karaktergrubu[Açıklama]{…}`, `\chargroup` | Tabloda tek satırda toplanan grup |
| `\gkarakter*[Tam ad]{AD}{kısaltma}`, `\gcharacter` | Grubun üyesi; yalnızca grubun içinde yazılır |

### Öteki kişi komutları

| Komut | Ne yapar | Örnek |
|---|---|---|
| `\sirkarakter{AD}{kısaltma}` | Konuşur ama tabloda hiç görünmez. İngilizcesi `\secretcharacter`; eski `\gizlikarakter` de çalışır | `\sirkarakter{HERKES}{hep}` |
| `\digerkisiler{…}`, `\othercast` | Tablonun altına toplu kişileri yazar | `\digerkisiler{Köylüler, müzisyenler.}` |
| `\setinlinename{kısaltma}{Ad}` | Replik içinde `\oz` yazınca çıkacak ad | `\setinlinename{oz}{Ozan Usta}` |
| `\setmaxnamewidth{ölçü}` | Replik adı sütununun genişliği | `\setmaxnamewidth{3cm}` |
| `\kisiler` | Kişi tablosunu basar; `\begin{document}` sonrasında | |

## Her kısaltmanın verdiği komutlar

`\karakter{SELİN}{se}` yazınca dört komut oluşur:

| Komut | Nerede | Ne yapar |
|---|---|---|
| `\se Metin` | Satır başında | SELİN'in repliği |
| `\se` | Replik içinde | Kişinin adını yazar: "Selin" |
| `\sedo Metin` | Satır başında | Yönerge: (Selin Metin). Satır sonunda biter |
| `\sefont` | Her yerde | Kişi adının yazı tipi |

Yönergeler zincirlenir: `\sedo\ozdo Birbirlerine bakarlar.` → (Selin, Ozan Birbirlerine
bakarlar.)

## Replik nasıl yürür

- Replik, altındaki satırlara boş satıra ya da komutla veya `#` ile başlayan satıra
  kadar sürer.
- Her kaynak satırı baskıda da ayrı satırdır; şiir ve şarkı için bir şey gerekmez.
- Satır sonundaki `%` sonraki satırı buna ekler.
- Yönerge kendi kaynak satırının sonunda biter.
- Replik içinde `_kelime_` altı çizili yazılır.

## Yönergeler

| Komut | Ne yapar | Örnek |
|---|---|---|
| `\ic{…}` | Replik içinde yönerge, parantezli ve italik | `\ic{gülerek}` → *(gülerek)* |
| `\action{…}` | Replikler arasında eylem paragrafı | `\action{Işıklar yavaşça açılır.}` |
| `\stagedir{…}` | Ortalanmış sahne yönergesi | `\stagedir{Karlı bir dağ köyü.}` |
| `\middir{…}` | Sağa yaslı italik yönerge | `\middir{Kapı çalar.}` |
| `\beat` | "(Sus.)" yazar | |
| `\girer{se,oz}` | Giriş: (SELİN ile OZAN girerler.) | |
| `\cikar{se}` | Çıkış: (SELİN çıkar.) | |
| `\dipnot{…}`, `\footnote{…}` | Dipnot; replikte ve yönergede çalışır | |

## Bölümler

Her bölüm iki biçimde yazılır. `#` satırları kısadır ve satır sonunda ya da bir `%`
yorumunda biter. Komut biçimi `nohash` ile de çalışır.

| `#` satırı | Komut | Ne yapar |
|---|---|---|
| `#perde Başlık`, `#act` | `\perde[Başlık]`, `\act` | Numaralı perde |
| `#sahne Yer — Zaman`, `#scene` | `\sahne[Yer — Zaman]`, `\scene` | Numaralı sahne |
| `#sahne[2.1] Yer` | `\sahne[Yer][2.1]` | Numarası elle verilmiş sahne |
| `#önsahne Yer`, `#prologue` | `\onsahne[Yer]`, `\prologue` | Numarasız ön sahne; sonraki sahne yine 1'dir |
| `#önsöz`, `#preface` | `\onsoz[Başlık]` | Kendi sayfasında önsöz |
| `#notlar`, `#notes` | `\notlar[Başlık]` | Kendi sayfasında yönetmene notlar |

`\#` bir `#` basar. Sahne başlığının altına o sahnede olanlar kendiliğinden yazılır.

## Başka diller

Türkçe tireleme dışında kalacak metin için `\ru{…}` gibi dil komutları vardır.
`\yenidil{jp}{Noto Serif CJK JP}` o yazı tipiyle `\jp{…}` komutunu ekler.

## Görünümü değiştirmek

Bunlar `\begin{document}` öncesinde `\renewcommand` ve `\setlength` ile değiştirilir.

| Ne | Komutlar |
|---|---|
| Yazı tipleri | `\namefont`, `\starfont`, `\actionfont`, `\stagedirfont`, `\castfont`, `\caststarfont`, `\casttitlefont`, `\sectiontitlefont`, `\scenetitlefont`, `\scenecastfont`, `\acttitlefont`, `\linenumberfont` |
| Boşluklar | `\beforesceneskip`, `\aftersceneskip`, `\beforeactskip`, `\afteractskip`, `\linenumbersep` |
| Kişi sayfası | `\beforecasttitleskip`, `\aftercasttitleskip`, `\castpageleftmargin`, `\castpagerightmargin`, `\castnamewidth` |
| Sözcükler | `\casttitlename`, `\prefacename`, `\notesname`, `\acttitlename`, `\scenetitlename`, `\prologuename`, `\curtainlocalizedtext`, `\scriptendlocalizedtext`, `\draftlocalizedtext`, `\beattext` |
| Giriş ve çıkış | `\entertext`, `\enterpluraltext`, `\exittext`, `\exitpluraltext`, `\andtext` |
| Ayırıcılar | `\actseparator`, `\sceneseparator` |

```latex
\renewcommand\scenetitlename{Tablo}
\setlength\aftersceneskip{2\baselineskip}
```

## Dactile'ın yazdığı komutlar

Bunları elle yazman gerekmez; oyuncu ve not kopyalarını üretirken Dactile kullanır.

| Komut | Ne yapar |
|---|---|
| `\tiyatrocopy{se,oz}{Oyuncu}` | O rollerin repliklerini kalın, gerisini soluk basar |
| `\tiyatrocopycover{…}` | Kopyanın kapağına ekler |
| `\tiyatromark` | Sonraki yönergeyi kalın yapar |
| `\tiyatrogap{n}` | n satırlık not boşluğu bırakır |
| `\tiyatronoend` | Sondaki "Perde Sonu" ve "Son" yazılarını kaldırır |
| `\tiyatronotes` | Notlar için sağ kenarı genişletir |
| `\tiyatronote{RRGGBB}{Kim}{n}{metin}` | n numaralı notu satırın yanına koyar |
| `\tiyatronoted{RRGGBB}{kelimeler}` | Notun konusunu renklendirir |
| `\tiyatronoteslist{Başlık}{…}` | Bütün notları sona dizer |
| `\tiyatronoteitem{…}`, `\tiyatronotereply{…}` | Listedeki not ve yanıtı |

## Derleme

```sh
xelatex oyun.tex
```

Sahne başlığındaki kişi listesi `.aux` dosyasından gelir. Dactile bunu kendi yazar, tek
geçiş yeter. Başka yerde düzenlediysen sınıf "Rerun to get them right" der, ikinci
geçiş düzeltir.
