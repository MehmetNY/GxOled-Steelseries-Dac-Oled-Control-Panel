# GxOled — Animation Scroll / GIF Studio UI Revision (24.10.2026)

Bu paket `GxOled-Fixed-Settings-2026-10-24-source.zip` temel alınarak hazırlanmıştır.

## Yapılan değişiklikler

- Animations sayfası dikey kaydırılırken eski düğme/metin izi kalmasına karşı tüm Win32 child kontrolleri, son konumları hesaplandıktan sonra topluca yeniden çiziliyor (`RedrawWindow`, `RDW_ALLCHILDREN`). Birbirine taşan kardeş pencerelerin çizimi de kırpılıyor (`WS_CLIPSIBLINGS`). Değişmeyen kontroller tekrar tekrar gösterilip gizlenmiyor.
- Sol menüye **GIF Studio** adlı bağımsız sekme eklendi; mevcut `Add GIF...` ve `Start / Stop GIF` fonksiyonları buraya taşındı. GxOled'in mevcut GIF oynatma kodu değiştirilmedi.
- **New copy** yerine **New Profile**: seçilen animasyon türü için henüz dosya yazmadan boş `{}` JSON editörünü açar. Sadece geçerli 128×64 profil, `Save & Apply` ile kaydedildiğinde özel profil klasöründe görünür. Editördeki `Reset`, o animasyonun orijinal şablonunu yükleyebilir.
- JSON editöründeki çok satırlı Windows EDIT kontrolüne **Ctrl+A** tümünü seç kısayolu eklendi.
- About sayfasına `Ko-fi · Support GxOled` düğmesi eklendi. Şu anda **https://ko-fi.com/** anasayfasını açar; yayıncının kişisel Ko-fi adresi tanımlanmamıştır.
- Sol alttaki Quit GxOled düğmesi küçültüldü.

## GGIF-Studio'dan gelecekte alınabilecek arayüz fikirleri

Kaynak: https://github.com/D4rkTT/GGIF-Studio (MIT lisansı). Bu sürümdeki GIF sekmesi yalnızca başlangıçtır, GGIF-Studio'nun kodu veya Electron altyapısı uygulamaya eklenmedi.

- Merkezi 128×64 canlı OLED önizlemesi.
- Katman listesi (yazı, GIF/video görsel katmanları).
- Kareler için zaman çizelgesi, sürükle/ara, önceki/sonraki kare ve zamanlamalar.
- GIF/video kaynakları, sığdır/kapla/uzat; kırpma, döndürme, aynalama, offset.
- Ton dönüştürme ve dithering yöntemleri için kullanıcı dostu ayarlar.
- GIF döngüsü, hız ve başlangıç/bitiş kırpma.

Önemli: Proje şimdilik doğrudan USB/HID GxOled bağlantısını korur; GameSense kullanımı şart koşulmaz. GIF düzenleme motorunu geliştirmek başka bir aşamadır.

## Windows doğrulama

1. ZIP'i ayrı klasöre çıkarın. `Validate-Windows.cmd` ile check → unit tests → release build çalıştırın.
2. `gxoled.exe` → Animations → fare tekerleğiyle aşağı/yukarı kaydırın. Metinlerin ve düğmelerin arkada iz bırakmadığını doğrulayın.
3. New Profile: editör boş `{}` açılmalı. Ctrl+A tümünü seçmeli. Reset ile geçerli JSON şablonu getirip Save & Apply ile yeni profil oluşturulabilmeli.
4. GIF Studio sekmesini açıp mevcut Add GIF ve Start / Stop GIF işlevlerini doğrulayın.
5. About'ta GitHub ve Ko-fi bağlantılarını ayrı ayrı kontrol edin.
6. Quit düğmesinin bütün sekmelerde sabit kaldığını kontrol edin.

**Not:** Bu ortamda Windows/Rust derleyicisi veya SteelSeries OLED bulunmadığından Win32 çalışma zamanı doğrulanmadı. Kaynak düzeyindeki kontroller, Windows build/test ve gerçek arayüz testi yerine geçmez.
