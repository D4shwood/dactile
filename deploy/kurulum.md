# Oracle'a kurulum

Sahip yalnızca Oracle panelinde makineyi açar ve bir kez `bash deploy/first.sh` çalıştırır. Sonrasında Claude, `sudo`suz `dactile` kullanıcısıyla `bash deploy/deploy.sh` ile kurar ve günceller; her kurulum sahibin onayıyla.

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

[duckdns.org](https://www.duckdns.org)'da bir ad al (ör. `ornek.duckdns.org`) ve sabit IP'yi yaz. IP değişmeyeceği için güncelleyici gerekmez.

## 3. Bu bilgisayarda, bir kez

Claude `deploy/site.env` dosyasını doldurur (git'e girmez):

```
IP=...
DOMAIN=ornek.duckdns.org
OWNER_KEY=~/indirilenler/ssh-key.key
OWNER=ubuntu
```

Sonra sahip:

```bash
bash deploy/first.sh
```

Bu, `setup.sh`'ı makineye gönderip `sudo` ile bir kez çalıştırır: paketler (Caddy, XeLaTeX, yazı tipleri, Tesseract, rclone), `dactile` kullanıcısı, iptables'ta 80/443, Caddy'nin HTTPS ayarı.

## 4. Sonrası (Claude)

- `bash deploy/deploy.sh`: arayüzü burada, sunucuyu orada derler, servisi yeniden başlatır. Commit'lenmemiş değişiklik varsa durur; önceki sürüm `~/bin/*.before` olarak yanında kalır.
- **Kişiler:** Herkesin kendi hesabı var: kullanıcı adı ve parola, ya da adresinin Google hesabı. İlk hesap (sahip) buradan açılır: `bash deploy/people.sh add ada "Ada Yıldız" --owner --mail ada@example.com`, sonra `bash deploy/people.sh reset ada` parola belirleme bağlantısını yazar (bir kez çalışır, yedi gün geçerli). Sonrası sitede: sahip Ayarlar → Kişiler'den e-postayla davet eder, davet edilen kullanıcı adını ve parolasını kendisi seçer. `people` listeyi gösterir, `remove ada` kişiyi çıkarır (oyunları kalır).
- **Posta:** Davet, "parolamı unuttum" ve adres doğrulama postaları bir posta servisiyle gider (ör. Brevo, ücretsiz). Servisin SMTP bilgileri `deploy/site.env`'e yazılır (git'e girmez), sonra `bash deploy/deploy.sh`:

```
MAIL_URL=smtps://smtp-relay.brevo.com:465
MAIL_USER=...
MAIL_PASS=...
MAIL_FROM=Dactile <dactile@ALAN-ADI>
```

  Posta yokken de site çalışır: davet ve parola bağlantılarını sahip üretir ve kendisi iletir.
- Yedek: saatte bir `~/data` git'e commit'lenir. Özel GitHub deposu ve Google Drive (`rclone`, `drive.file`) bağlanınca oraya da gider; Drive'da son 5 gün kalır. Geri yükleme bir kez denenir.

## Google ile giriş (istenirse)

Bağlantıyla giriş her zaman çalışır. Google ile giriş için bir kez:

1. [console.cloud.google.com](https://console.cloud.google.com) → yeni proje → APIs & Services → OAuth consent screen: "External", uygulama adı, destek e-postası. Kapsam olarak yalnızca `openid` ve `email`. Yayın durumu "Testing" kalırsa giriş yapacak adresler "Test users"a eklenir; "In production" yapılırsa gerekmez.
2. Credentials → Create credentials → OAuth client ID → "Web application". Authorized redirect URI: `https://ALAN-ADI/giris/google/donus`.
3. Çıkan Client ID ve Client secret `deploy/site.env`'e yazılır (git'e girmez):

```
GOOGLE_ID=...apps.googleusercontent.com
GOOGLE_SECRET=...
```

4. `bash deploy/deploy.sh`. Sonra hesabının adresi bir Google hesabı olan herkes "Google ile gir" ile girer; davet edilen de hesabını Google ile açabilir.

Yalnızca adresi bir hesabın doğrulanmış adresi olan girer; Google'ın doğrulamadığı adres kabul edilmez.

## Bilinenler

- Sunucuda Times New Roman yok; sayfalar TeX Gyre Termes ile çıkar. Masaüstündekiyle aynı görünüm istenirse `ttf-mscorefonts-installer` lisans onayıyla kurulur.
- XeLaTeX şimdilik yalıtımsız çalışır: tek kullanıcı varken sorun değil. Başka biri girmeden önce derleme `bwrap` içine alınır; ayrıntısı `docs/web.md`'de.
