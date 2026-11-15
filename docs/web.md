# Dactile Web

Dactile'ın tarayıcıdaki hali: kurulum gerektirmeyen bir site. Karışık bir ekip (yazar, dramaturg, yönetmen, oyuncu) aynı oyun üzerinde çalışır, oyuncu rolünü telefonda görür, herkes aynı PDF'i alır. Önce kendi ekibimize kapalı beta; herkese açık site sonra.

Masaüstü uygulaması ve Dactile Light'ın davranışı bu işten etkilenmez.

## Bugün ne var

- **`server/`**, iki program:
  - **`dactile-server`**: masaüstünün komutları (`backend/`) HTTP üzerinden, **bir süreç bir kişi**. Kişinin her şeyi tek klasörde (`DACTILE_DATA`); dosya yolları o klasörün dışına çıkamaz (`confine.rs`). Tek başına parolayla da çalışır (yerelde denemek için).
  - **`dactile-gate`** (`gate.rs`): tarayıcının konuştuğu kapı. Kimin girdiğini bilir, gelen kişinin `dactile-server`'ını kendi klasörü (`people/<kimlik>/`) üzerinde başlatır, isteklerini yalnızca ikisinin bildiği bir sözle ona yollar, yirmi dakika boş kalınca durdurur. Sunucu durmuşken gelen istek onu yeniden başlatır; açık oyun kendiliğinden yeniden açılır, yazılan kaybolmaz. Kapı giderse sunucular da gider.
  - **Hesaplar:** Herkesin kullanıcı adı (klasörünün adı, değişmez), görünen adı, e-posta adresi ve parolası var; ya da parolasız, adresinin Google hesabıyla girer (`people.rs`). Kimse kendi kendine hesap açamaz: sahip e-postayla davet eder, davet edilen kullanıcı adını ve parolasını kendisi seçer. Parolalar argon2id ile, yedeğe girmeyen ayrı bir dosyada. Unutulan parola postayla gelen bir bağlantıyla (bir saat) ya da sahibin ürettiği bir bağlantıyla (yedi gün) yenilenir; parola değişince kişinin öteki oturumları kapanır. Yanlış parola hem adres hem hesap başına sayılır: beşte on beş dakika bekletir. Oturum otuz gün sürer. Listeden çıkarılan o an dışarıdadır; yazdıkları klasöründe kalır. Giriş ekranları ayrı, küçük bir Svelte sayfası (`app/web/door/`).
- **`app/web/`**: tarayıcı girişi. Arayüz masaüstününkiyle aynı dosyalar (`app/src`); `@tauri-apps/api` yerine `app/web/shim/` geçer ve sunucuyla konuşur. Dosya seçme yükleme, dosya açma indirme olur.
- **`web-wasm/`** (`dactile-wasm`): arka ucun yalnızca metin okuyan 23 komutu (`backend/src/commands.rs`, `TEXT_ONLY`) tarayıcı için; sıkıştırılmış yaklaşık 195 KB. Tarayıcı bunları kendisi çalıştırır: yazarken görünüm, pano, kadro ve `\` menüsü sunucuyu beklemez. Yüklenemezse ya da bozulursa aynı komutlar sunucuya gider.
- **`deploy/`**: kurulum (Ubuntu, Caddy, kullanıcı systemd'si, saatlik yedek), iki site olarak: gerçek site ve önce her şeyin gittiği deneme sitesi (PBE, `DACTILE_STAGE=pbe`). Adımlar `deploy/kurulum.md`'de.

Yerelde çalıştırma, tek kişi:

```
DACTILE_DATA=<boş klasör> DACTILE_PASSWORD=<12+ karakter> cargo run -p dactile-server
npm run dev:web        # app/ içinde; http://localhost:1421
```

Birkaç kişi (önce `cargo build -p dactile-server`, iki programı da derler):

```
export DACTILE_HOME=<boş klasör>
target/debug/dactile-gate add ada "Ada Yıldız" --owner
target/debug/dactile-gate invite ada      # /giris/... yolunu yazar
target/debug/dactile-gate                 # 1430'da dinler
```

## Mimari kararlar

1. **Yazarken tarayıcı, dosyada sunucu.** Yalnızca metin okuyan komutlar tarayıcıda WebAssembly olarak çalışır; yazarken ağ beklenmez. Kayıt, PDF, içe aktarma ve OCR sunucudadır. Vekil katman (`shim/core.ts`) komutu adına göre birine yollar; arayüz bunu bilmez. Yazarken hâlâ sunucuya gidenler: yazım denetimi (sözlük sunucuda), özetler ve notlar (oyunun yanındaki dosyalarda).
2. **Bir süreç bir kişi.** Çekirdek ayarların, önbelleğin ve çöpün yerini süreç genelinde bulur; kilit de süreç numarasına bakar. Bunları sökmek yerine her kişiye kendi süreci ve klasörü verilir; öndeki kapı isteği doğru sürece yollar. Bugünkü sınır: süreçlerin hepsi aynı sistem kullanıcısıyla çalışır. Kişileri ayıran, her sürecin yolları kendi klasöründe tutması ve XeLaTeX'in kutusudur; işletim sistemi düzeyinde ayrım (kişi başına sistem kullanıcısı ya da sürecin kendisinin de kutuda çalışması) sonraki iştir.
3. **PDF sunucuda, kutu içinde.** Herkes aynı yazı tipleriyle aynı sayfaları görsün diye PDF tek yerde derlenir. `DACTILE_SANDBOX=bwrap` ile XeLaTeX ağsız, yalnızca oyunun klasörünü ve sınıfı gören bir `bwrap` kutusunda çalışır (`core/src/build.rs`, `tex`; testi `core/tests/sandbox.rs`). Kurulum betiği kutu çalışıyorsa açar, çalışmıyorsa bağırır. Başka biri sunucuya girmeden önce şarttır. İçe aktarmanın araçları da aynı kutuda, yalnızca kendilerine verilen belgeyi görerek çalışır (`build::tool`).
4. **Giriş: kullanıcı adı ve parolayla, ya da Google ile; hesap yalnızca davetle.** Davet, parola yenileme ve adres doğrulama postaları bir posta servisinin SMTP'siyle gider (`mail.rs`, sistemin `curl`'ü; parola komut satırına girmez). Posta yoksa sahip aynı bağlantıları kendisi üretip iletir. Google ile giriş (`google.rs`), siteye bir Google istemcisi tanıtılınca açılır: yalnızca bir hesabın doğrulanmış adresi olan ve Google'ın doğruladığı hesap girer. Google'a soruyu sistemin `curl`'ü sorar; sunucuya ayrı bir HTTPS kitaplığı eklenmedi. Gerçek Google'a karşı henüz denenmedi (sahte bir yanıtlayıcıya karşı testi var).
5. **Yedek iki yere.** Oyunlar düz metin olarak saatte bir özel bir git deposuna, günde bir tarihli bir Drive klasörüne (son beşi kalır). Oyunların oralarda da durduğu ekibe söylenir. Yedek anahtarları Dactile'ı çalıştıran kullanıcıdan ayrı durur.

## Birlikte çalışma

- **Birlikte:** Yazma izni olan herkes aynı anda yazar, imleçler görünür.
- **Sırayla:** Bir "kalem" vardır; diğerleri okur ve not bırakır.
- **Geçiş her an:** Sahip ya da yazar iki biçim arasında istediği an geçer. Sıraya geçen kalemi alır; açık editörler o anda kilitlenir ya da açılır, yazılanlar kaybolmaz.
- **Yapısal işlem kilidi:** Sahne taşıma, bölme, birleştirme, karakter yeniden adlandırma kısa bir kilitle yapılır: eşitleme oyunun yapısını bilmez.
- **Geri alma** yalnızca kişinin kendi yazdığını geri alır.
- **Masaüstünden `.tex` yükleme** sitedeki metni ezmez: yeni sürüm olur, fark gösterilip onaylanır.
- **Roller:** Sahip, yazar, notçu, okur, oyuncu. Yönetmen notları ve dağıtımdaki kişi bilgileri oyunculara rol bazında gizlenir.
- **Oyuncu görünümü:** Telefona göre ayrı, küçük bir sayfa: kendi rolü vurgulu, ezber, provada not.

Herkesin oyunları kendi klasöründedir; ortak oyunlar projelerdedir (`server/src/projects.rs`). Projeyi açan yöneticisidir; üyeleri yönetici, yazar ya da okurdur. Kapı her kişinin sunucusunu başlatırken ona projelerini ve rollerini söyler (`DACTILE_PROJECTS`); sunucu yolları bu klasörlerle sınırlar (`confine::Roots`) ve okurun bir projede yazan komut çalıştırmasını reddeder (`confine::WRITES`, her komut ya orada ya `READS`'te). Üyelik değişince kapı o kişinin sunucusunu yeniden başlatır.

Proje oyunları canlı yazılır (`server/src/room.rs`, tarayıcıda `app/src/lib/live.ts`). Bir proje oyunu açılınca kapı onun için bir oda açar: oyun bir Yjs belgesi olarak (`yrs`) kapıda durur, tarayıcılar ona WebSocket ile (`/api/oda`) Yjs'nin kendi protokolüyle bağlanır. Kapı kimin kim olduğunu ve rolünü bildiği için oda değişikliği yalnızca yazabilenden alır: birlikte modunda her yazardan, sırayla modunda kalemi tutandan; okur canlı izler. İmleçteki ad kapının oturumundan yazılır. Kalem ilk gelen yazara verilir, bırakılınca bekleyen bir yazara geçer, sahibi 30 saniyeden uzun yoksa boşa çıkar; yönetici her an alabilir. Yönetici modu değiştirince açık odalar hemen öğrenir; sıraya geçerken kalem geçişi yapana gider. Sahne taşıma ve benzeri yapısal işlemler birlikte modunda kısa bir "yalnız" anıyla yapılır.

Oyun `.tex` olarak kalır: oda onu yazma durduktan 2 saniye sonra, Ctrl+S'te, kişinin sunucusuna bir komut gitmeden önce (PDF diskten okunur), son kişi çıkınca ve kapı kapanırken yazar. Geçmiş projenin kendi klasöründedir (`<proje>/history`, `.dactile-project` işaretiyle). Yanında `.<ad>.dactile-y` belgenin Yjs halini tutar: kapı yeniden başlayınca kopan editörler dönüp birleşir. Dosya odanın dışında değiştiyse belge yeni bir adla (epoch) baştan kurulur; eski editör birleştirmez, kopukken yazdıklarını gösterip oyunu yeniden açar. Kişilerin sunucuları proje oyunlarında kilit almaz ve onları kaydetmez. Oyun silinir ya da taşınırsa oda kapanır.

## Sıra

1. **Tek kişilik kurulum.** Bugünkü haliyle bir sunucuya; kurulum betikleri gerçek makinede sınanır.
2. **Güvenlik ve yedek.** Yapılanlar: XeLaTeX kutusu (Ubuntu'da sınanacak), kötü giden yedeğin ana ekranda söylenmesi, yüklenen belgelerin temizliği, diskte duran süreli oturumlar, yanlış parola sınırı. Sonra eklenenler: içe aktarma araçları (`pdftotext`, `pdftoppm`, `tesseract`) da kutuda; kişi başına disk sınırı (`DACTILE_QUOTA_MB`, yalnızca yüklemeyi durdurur, yazmayı asla). Kalanlar: yedek anahtarlarının Dactile'ı çalıştıran kullanıcıdan ayrılması, bir kez gerçek geri yükleme.
3. **Yazarken tarayıcı.** Metin komutları WebAssembly'de (yapıldı). Kalan: sözlük siteyle gelsin ki yazım denetimi de tarayıcıda çalışsın; özetler ve notlar metinle birlikte gelsin.
4. **Çok kişi.** Yapılanlar: kişi başına süreç, giriş kapısı, hesaplar (kullanıcı adı ve parola, e-postayla davet, parolamı unuttum, adres doğrulama), ana ekranda kim olduğu ve çıkış, Ayarlar'da "Hesabım" ve sahibe "Kişiler" (davet, geri alma, parola bağlantısı, çıkarma). Google girişi de yazıldı (gerçek Google'a karşı denenecek). Kalanlar: işletim sistemi düzeyinde ayrım; herkes aynı sayfa (özgür bir yazı tipi iki tarafta varsayılan, sunucuda sabit TeX Live).
5. **Ekip.** Yapılanlar: projeler ve proje başına roller (yönetici, yazar, okur), Projeler sayfası, ana ekranda proje sekmeleri, projeye yeni oyun, sırayla yazmada yazanın adı. Yapılanlar ayrıca: canlı yazma, sırayla ve birlikte arasında geçiş, okurun canlı görünümü. Kalanlar: notların canlı paylaşımı, notçu ve oyuncu rolleri, oyuncu görünümü, sürüm geçmişi ekranı.

Sonraya: herkese açık site, kendi kurulum rehberi, çevrimdışı yazma, tarayıcıda derleme, uçtan uca şifreleme.

## Denemelerden bilinenler

İlk aşamada dört şey ayrı ayrı denendi; kodları bu depoya alınmadı, bulguları şunlar:

- **Çekirdek tarayıcıda:** Görünüm çıktısı yereldekiyle birebir aynı; yaklaşık 54 KB (gzip); 100 sayfalık oyunda bir görünüm 10 ms.
- **Canlı yazma (Yjs) gerçek editörde çalışıyor:** İki yazar ve bir okurla; Türkçe harfler ve emoji iki yönde sağlam; geri alma yalnızca kişinin kendi yazdığını alıyor; sunucu kapalıyken yazılanlar dönünce birleşiyor. Kesintisiz yazarken görünüm en geç yarım saniyede tazeleniyor.
  - Kalanlar: sekme yenilenince eski imleç bir süre hayalet kalıyor; imleçteki adı sunucu denetlemiyor; süresi dolan geçişte arayüz yenisini istemeli.
- **Kutuda derleme çalışıyor:** `bwrap` içinde ağ, süreç ve kullanıcı alanları ayrı; `\input{/etc/passwd}`, ev klasörü, `/proc`, `\write18`, dışarıya yazma, sonsuz döngü, bellek ve disk doldurma denemelerinin hepsi durduruldu. 211 sayfalık oyun 22 saniyede, tepe bellek 350 MB.
  - **`openin_any=p` yeni TeX Live'da hiçbir şey yapmıyor:** okumaya karşı tek duvar kutu. Kutuya bağlanan her yer oyun tarafından okunabilir; oralarda gizli bir şey durmamalı.
  - **Ubuntu 24.04 ve sonrası `bwrap`'ı engelleyebilir** (ayrıcalıksız kullanıcı alanları AppArmor ile kısıtlı). Sunucuda ilk denenecek şey bu.
  - `xdvipdfmx`, `/etc/paperspecs` olmadan `a4`'ü tanımıyor.
  - Yazı tipleri makineye bağlı: sunucudaki paket sabitlenmeli. Toplam bellek sınırı `cgroup` ile konmalı.

## Güvenlik notları

- Tarayıcıdan gelen her yol kişinin klasörüne göre denetlenir; dosya adı olan alanlar yalnızca düz ad olabilir. Yeni bir komut yol alıyorsa `confine.rs`'e eklenir.
- Yazan her istek arayüzün kendi başlığını taşımak zorundadır; başka sitenin formu ekleyemez.
- Site sahibi oyunları teknik olarak okuyabilir; ekibe söylenir. Kayıtlara oyun metni yazılmaz.
- Gerçek ortam ile test ortamı ayrıdır. Testler yalnızca uydurma oyunlarla çalışır.
