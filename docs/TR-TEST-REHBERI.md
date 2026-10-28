# DAC ses-OSD ölçüm testi — adım adım (TR)

Bu rehber, DAC ses düğmesini çevirdikten sonra GxOled'in ~6 sn geç dönmesinin **nerede** oluştuğunu
ölçmek içindir. Test paketi bir **ölçüm aracıdır**, sorunu çözdüğünü iddia etmez.

## 1. Paketi nereden indireceksin?

Hazır paket, kaynak kodun yanında GitHub'da **ön-sürüm (pre-release)** olarak yayınlanır. Derleme yapman gerekmez.

1. Tarayıcıda şu sayfayı aç:
   <https://github.com/MehmetNY/GxOled-Steelseries-Dac-Oled-Control-Panel/releases>
2. En üstlerde **turuncu "Pre-release" etiketli** ve başlığı **"OSD ölçüm paketi (xxxxxxx) — ürün sürümü değildir"**
   olan kaydı bul. (Adı `diag-osd-stamp-xxxxxxx` etiketiyle başlar; `v0.5.1`, `v0.5.0` ve `stable` ürün sürümleridir, onları kullanma.)
3. O kaydın altındaki **Assets** bölümünü aç (gerekirse "Show all assets").
4. **`GxOled-OSD-Diagnostic-Package-xxxxxxx.zip`** dosyasına tıkla ve indir.
   * "Source code (zip)" veya "Source code (tar.gz)" dosyalarını **indirme**; onlar kaynak koddur, EXE içermez.
5. İsteğe bağlı doğrulama: aynı yerdeki `.zip.sha256` dosyasındaki değeri, PowerShell'de
   `Get-FileHash .\indirdigin.zip -Algorithm SHA256` çıktısıyla karşılaştır.

## 2. ZIP'i çıkar

`ZIP → sağ tık → Tümünü ayıkla...` → hedef olarak **`C:\gx-test`** yaz. ZIP'in içinden doğrudan çalıştırma.

İçinde şunlar olmalı: `gxoled.exe`, `Test-VolumeOSD.ps1`, `1-OTOMATIK-TEST.cmd`, `2-Loglari-ac.cmd`,
`BUILD-INFO.txt`, `BASLA-BURADAN.txt`.

## 3. Testi yap (otomatik, tek çift tık)

Ayrıntılı adımlar paketin içindeki **`BASLA-BURADAN.txt`** dosyasındadır (kopyası:
[`../scripts/diagnostics-package/BASLA-BURADAN.txt`](../scripts/diagnostics-package/BASLA-BURADAN.txt)). Özet:

1. Çalışan GxOled'i tepsiden kapat.
2. Telefonla OLED'i 1080p 60 fps **tek video** olarak kaydetmeye başla.
3. `1-OTOMATIK-TEST.cmd` dosyasına çift tıkla.
4. Saat ekrandayken 5 sn bekle, düğmeyi tek seferde 3-5 tık çevir, elini çek, saat dönene kadar bekle. **En az 10 kez** tekrarla; sağ alttaki sayının ilk hanesi deney numarasıdır ve kendiliğinden artar.
5. Tepsiden "Çıkış" de (cihaz ayarları otomatik eski haline döner), videoyu durdur.
6. Videoyu ve en yeni `osd-default-lab-*.log` dosyasını (`2-Loglari-ac.cmd` klasörü açar) gönder.

Sonuçları video + log'dan ben otomatik okuyorum; numara not etmen gerekmiyor.

## 4. Önceki ölçümün sonucu (28 Ekim 2026, 1080p 60 fps videolar)

| Bulgu | Kanıt |
| --- | --- |
| Cihaz, ses göstergesi sürerken gönderilen kareleri **atıyor** (gecikmeli göstermiyor) | Ekrana ilk gelen kare, hep göstergenin bittiği andan sonra yazılan ilk kare; aradaki numaralı kareler hiç görünmüyor. |
| Gösterge, bir kare yazıldığı anda kapanıyor | Üç varsayılan kayıtta "ilk görünen kare → videoda gösterge kapanışı" gecikmesi 2,157-2,160 sn ile sabit. |
| Kilit süresi ≈ 5-6 sn ve yazılım zamanlamasından bağımsız | Bekleme-yok modu ve varsayılan mod aynı sonucu veriyor. |

## 5. Neden programı baştan yazmak 6 sn'yi kendiliğinden çözmez?

Ölçümler şunu gösterdi: 18 Ekim'de fiziksel olarak iyi çalıştığı bildirilen eski EXE, aynı cihaz
üzerinde bugün de ~6 sn veriyor ve kaynak kodu karşılaştırınca **HID gönderim yöntemi birebir aynı**
(iki 64 sütunluk 1024 baytlık feature report, komut 0x93). Yani sıfırdan yazılan bir sürüm de aynı cihaza aynı komutları gönderecekse aynı gecikmeyi görür.
Videolu ölçüm bunu doğruladı: gecikme yazılımda değil, cihazın kendi ana ekranının kareleri
~5-6 sn boyunca yok saymasında. Bu yüzden çözüm, cihazın kilidini açabilecek komutları aramaktır
(yukarıdaki otomatik deney döngüsü); kodu yeniden yazmak bunu değiştirmez.

## 6. Dokunulmayanlar

`v0.5.1`, `v0.5.0` ve `stable` etiketleri/EXE'leri bu çalışmada değişmez. Düzeltme DAC üzerinde
doğrulandıktan sonra, senin onayınla v0.5.1 üzerine yazılacaktır
([`BRANCHES-AND-RELEASES.md`](BRANCHES-AND-RELEASES.md)).
