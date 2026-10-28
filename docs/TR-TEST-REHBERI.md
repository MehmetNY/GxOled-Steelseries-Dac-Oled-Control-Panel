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

İçinde şunlar olmalı: `gxoled.exe`, `Test-VolumeOSD.ps1`, `1-STAMP-varsayilan.cmd`, `2-STAMP-bekleme-yok.cmd`,
`3-Loglari-ac.cmd`, `BUILD-INFO.txt`, `BASLA-BURADAN.txt`.

## 3. Testi yap

Tüm ayrıntılı adımlar (kamera kurulumu, kaç tekrar, ne not edilecek) paketin içindeki
**`BASLA-BURADAN.txt`** dosyasındadır; aynı metin bu depoda
[`../scripts/diagnostics-package/BASLA-BURADAN.txt`](../scripts/diagnostics-package/BASLA-BURADAN.txt) olarak durur.
Özeti:

1. Çalışan GxOled'i tepsiden kapat (sağ tık → Çıkış/Quit). Görev Yöneticisi → Ayrıntılar'da `gxoled.exe`/`ggoled.exe` kalmamalı.
2. Telefonu OLED'e sabitle, **en az 60 fps** (mümkünse 120/240 fps yavaş çekim) kayda başla.
3. `1-STAMP-varsayilan.cmd` dosyasına çift tıkla. Düğmeyi birkaç tık çevir, DOKUNMA, saat dönene kadar bekle, o anda köşedeki 4 haneli numarayı not et. **En az 5 tekrar.**
4. Tepsiden "Çıkış" ile kapat.
5. `2-STAMP-bekleme-yok.cmd` ile aynısını yap.
6. `3-Loglari-ac.cmd` ile log klasörünü aç; en yeni iki `osd-*-stamp-*.log` dosyasını ve numara notlarını bana gönder.

## 4. Sonuç ne anlama gelir?

| Gözlem | Anlamı (hipotez) |
| --- | --- |
| Saat dönünce görünen numara, ~2,5 sn'de gönderilen ilk karenin numarası | Kare cihaza ulaşmış, cihaz kendi göstergesi bitene kadar tutup sonra göstermiş (**geç gösterim**). |
| Numara, o ilk kareden birkaç adım büyük | Aradaki kareler cihaz tarafından **atılmış**; cihaz yalnızca göstergesi bittikten sonra gelen kareyi kabul ediyor. |
| İki tur arasında fark var | Gönderim zamanlamasının etkisi var; bu da ayrıca bilgidir. |

Bunlar yorumlama kurallarıdır; kesin hüküm ölçüm sonuçları gelince verilir.

## 5. Neden programı baştan yazmak 6 sn'yi kendiliğinden çözmez?

Ölçümler şunu gösterdi: 18 Ekim'de fiziksel olarak iyi çalıştığı bildirilen eski EXE, aynı cihaz
üzerinde bugün de ~6 sn veriyor ve kaynak kodu karşılaştırınca **HID gönderim yöntemi birebir aynı**
(iki 64 sütunluk 1024 baytlık feature report, komut 0x93). Yani sıfırdan yazılan bir sürüm de aynı
cihaza aynı komutları gönderecekse, aynı gecikmeyi görme olasılığı yüksektir. Önce gecikmenin
yazılımda mı, cihazda mı oluştuğunu bu ölçümle ayırıyoruz; çözüm bu kanıta göre yazılacak.

## 6. Dokunulmayanlar

`v0.5.1`, `v0.5.0` ve `stable` etiketleri/EXE'leri bu çalışmada değişmez. Düzeltme DAC üzerinde
doğrulandıktan sonra, senin onayınla v0.5.1 üzerine yazılacaktır
([`BRANCHES-AND-RELEASES.md`](BRANCHES-AND-RELEASES.md)).
