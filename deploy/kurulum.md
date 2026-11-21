# Oracle'a kurulum

Sahip yalnızca Oracle panelinde makineyi açar ve bir kez `bash deploy/first.sh` çalıştırır. Sonrasında Claude, `sudo`suz `dactile` kullanıcısıyla kurar ve günceller: önce deneme sitesine (`bash deploy/deploy.sh pbe`), orada denenince gerçek siteye (`bash deploy/deploy.sh live`). Her kurulum sahibin onayıyla.

## 1. Oracle panelinde

1. **Hesap:** Kullandıkça öde (PAYG) hesabına geçiş ve bir bütçe uyarısı (ör. 1 $). Boşta makine geri alma kuralını kaldırır; Always Free kaynaklar ücretsiz kalır. Geçmeden önce Oracle'ın "Always Free Resources" sayfasından doğrulanır.
2. **Makine:** Compute → Instances → Create instance.
   - Image: Canonical Ubuntu 24.04 (varsa daha yenisi de olur).
   - Shape: Ampere, `VM.Standard.A1.Flex`, 2 OCPU, 12 GB.
   - Ağ: hesaptaki başka bir makineyle aynı VCN ve public subnet olabilir; "Assign a public IPv4 address" açık.
   - SSH: kendi açık anahtarın ya da Oracle'ın ürettiği anahtar çifti (özel anahtarı indir, sakla).
   - Hesaptaki öteki makinelere dokunulmaz.
3. **Sabit IP:** Networking → IP Management → Reserved Public IPs → Reserve. Sonra makine → Attached VNICs → IPv4 Addresses → Edit: önce "No public IP", kaydet; tekrar Edit → "Reserved public IP" ve az önceki IP.
4. **Kapılar:** Makinenin subnet'i → Security List → Add Ingress Rules: kaynak `0.0.0.0/0`, TCP, hedef port `80`; aynısı `443`.

## 2. Alan adı

[duckdns.org](https://www.duckdns.org)'da bir ad al (ör. `ornek.duckdns.org`) ve sabit IP'yi yaz. IP değişmeyeceği için güncelleyici gerekmez. Deneme sitesi için ikinci bir ad (ör. `ornek-pbe.duckdns.org`), aynı IP'ye.

## 3. Bu bilgisayarda, bir kez

Claude `deploy/site.env` dosyasını doldurur (git'e girmez):

```
IP=...
DOMAIN=ornek.duckdns.org
PBE_DOMAIN=ornek-pbe.duckdns.org
OWNER_KEY=~/indirilenler/ssh-key.key
OWNER=ubuntu
```

Sonra sahip:

```bash
bash deploy/first.sh
```

Bu, `setup.sh`'ı makineye gönderip `sudo` ile bir kez çalıştırır: paketler (Caddy, XeLaTeX, yazı tipleri, Tesseract, rclone), `dactile` kullanıcısı, iptables'ta 80/443, Caddy'nin HTTPS ayarı (iki site; deneme sitesine arama motorları girmez). `PBE_DOMAIN` sonradan eklenirse `first.sh` bir kez daha çalıştırılır: tekrar çalışması zararsız.

## 4. Sonrası (Claude)

- **İki site:** Gerçek site (`DOMAIN`) ve deneme sitesi (`PBE_DOMAIN`, PBE). Birbirinden ayrılar: her birinin kendi klasörü, hesapları, oyunları ve servisi var. Deneme sitesi `~/pbe` içinde, 1431'de çalışır, yedeği alınmaz, her sayfasında "DENEME SİTESİ" yazar.
- `bash deploy/deploy.sh pbe` ya da `live`: arayüzü burada, sunucuyu orada derler, servisi yeniden başlatır. Commit'lenmemiş değişiklik varsa durur; önceki sürüm `bin/*.before` olarak yanında kalır. `live`, deneme sitesinde çalışan commit'ten başkasını kurmaz: yeni bir şey önce denemeye gider. Aynı commit'in masaüstü AppImage'ı da derlenip imzalanır ve sitenin `/indir/` yoluna konur; bağlı masaüstleri oradan güncellenir. İmza anahtarı yoksa durur; `--no-desktop` bu adımı atlar. Windows kurulumu da burada derlenir; bunun için bir kez `sudo pacman -S lld llvm`, `paru -S nsis`, `rustup target add x86_64-pc-windows-msvc` ve `cargo install --locked cargo-xwin` gerekir (cargo-xwin ilk derlemede Microsoft'un CRT ve SDK dosyalarını indirir). `--no-windows` yalnızca bunu atlar.
- **Kişiler:** Herkesin kendi hesabı var: kullanıcı adı ve parola, ya da adresinin Google hesabı. İlk hesap (sahip) buradan açılır: `bash deploy/people.sh live add ada "Ada Yıldız" --owner --mail ada@example.com`, sonra `bash deploy/people.sh live reset ada` parola belirleme bağlantısını yazar (bir kez çalışır, yedi gün geçerli). Sonrası sitede: sahip Ayarlar → Kişiler'den e-postayla davet eder, davet edilen kullanıcı adını ve parolasını kendisi seçer. `people` listeyi gösterir, `remove ada` kişiyi çıkarır (oyunları kalır). Deneme sitesinin hesapları için `live` yerine `pbe`.
- **Posta:** Davet, "parolamı unuttum" ve adres doğrulama postaları bir posta servisiyle gider (ör. Brevo, ücretsiz). Servisin SMTP bilgileri `deploy/site.env`'e yazılır (git'e girmez), sonra iki siteye de kurulur:

```
MAIL_URL=smtps://smtp-relay.brevo.com:465
MAIL_USER=...
MAIL_PASS=...
MAIL_FROM=Dactile <dactile@ALAN-ADI>
```

  Posta yokken de site çalışır: davet ve parola bağlantılarını sahip üretir ve kendisi iletir.
- Yedek (yalnızca gerçek site): saatte bir `~/data` git'e commit'lenir. Özel GitHub deposu ve Google Drive (`rclone`, `drive.file`) bağlanınca oraya da gider; Drive'da son 5 gün kalır. Geri yükleme bir kez denenir.

## Google ile giriş (istenirse)

Bağlantıyla giriş her zaman çalışır. Google ile giriş için bir kez:

1. [console.cloud.google.com](https://console.cloud.google.com) → yeni proje → APIs & Services → OAuth consent screen: "External", uygulama adı, destek e-postası. Kapsam olarak yalnızca `openid` ve `email`. Yayın durumu "Testing" kalırsa giriş yapacak adresler "Test users"a eklenir; "In production" yapılırsa gerekmez.
2. Credentials → Create credentials → OAuth client ID → "Web application". Authorized redirect URI: `https://ALAN-ADI/giris/google/donus` (deneme sitesi için ikincisi, kendi adıyla).
3. Çıkan Client ID ve Client secret `deploy/site.env`'e yazılır (git'e girmez):

```
GOOGLE_ID=...apps.googleusercontent.com
GOOGLE_SECRET=...
```

4. Önce `pbe`, sonra `live` kurulur. Sonra hesabının adresi bir Google hesabı olan herkes "Google ile gir" ile girer; davet edilen de hesabını Google ile açabilir.

Yalnızca adresi bir hesabın doğrulanmış adresi olan girer; Google'ın doğrulamadığı adres kabul edilmez.

## Bilinenler

- Sunucuda Times New Roman yok; sayfalar TeX Gyre Termes ile çıkar. Masaüstündekiyle aynı görünüm istenirse `ttf-mscorefonts-installer` lisans onayıyla kurulur.
- XeLaTeX şimdilik yalıtımsız çalışır: tek kullanıcı varken sorun değil. Başka biri girmeden önce derleme `bwrap` içine alınır; ayrıntısı `docs/web.md`'de.
