# GxOled – İki sütunlu pencere düzeni düzeltmesi (24.10.2026)

Bu paket, kullanıcının Windows üzerinde başarılı biçimde derlediği **GxOled-Layout-Recovery-2026-10-24** kaynak sürümünü temel alır.

## Ne değişti?

- Genel ayar penceresi yeniden **solda kategoriler, ortada ayarlar ve sağda OLED önizlemesi** şeklinde iki sütunlu oldu. Önizleme, formun altına taşınmıyor.
- Boyut değiştirildiğinde form ve önizleme sütunları kullanılabilir genişliğe uyarlanıyor; yazı ve butonların DPI ölçeklemesi yalnızca Windows'un ekran DPI'sına bağlı. Pencere genişliği değiştikçe ölçek katsayısı değişmiyor.
- Soldaki ayarlar kendi dikey kaydırma alanını kullanıyor. OLED önizlemesi ve About QR sağda sabit kalıyor.
- Animations kısmındaki `Custom profiles folder` notu üç kısa satıra bölünerek dar form genişliğinde de okunması kolaylaştırıldı. Dosya konumu değişmedi: `%APPDATA%\GxOled\profiles\custom`.
- Brightness trackbar'ı, fare tekerleği üzerinde gezinirken değiştirmesin diye Win32 alt-sınıflandırma ile `WM_MOUSEWHEEL` mesajını yutuyor. Kaydırıcı fareyle çekilerek veya klavyeyle ayarlanabilir.
- About bölümündeki **tek GitHub butonu** daha kısa bir başlık kullanıyor, URL değişmedi.
- About bölümünde temeli sağlayan [JerwuQu/ggoled](https://github.com/JerwuQu/ggoled) ve animasyonları sağlayan [pozitronik/steelclock-go](https://github.com/pozitronik/steelclock-go) projelerine öncelikli teşekkür bulunuyor. Bağış çağrısı ayrı kısa bir paragraf.
- Sabit USDT/TRON QR'si sağ sütunda, resmin altında küçük fontlu iki açıklama satırıyla görüntüleniyor. Orijinal PNG varlığı değiştirilmedi.
- `Quit GxOled` düğmesi **tüm sekmelerde sol alt köşede** sabit.
- Kopya `716` etiket eşlemesi kaldırıldı (derlemedeki `unreachable pattern` uyarısı için).

## Değişmeyenler

- SteelClock-Go tabanlı dört animasyonun kaynakları, JSON varsayılan profilleri ve derlenmiş `gxoled-animations.exe` aynen korundu.
- Saatler, medya davranışı, Casio çizimi, donanım iletişimi, bağış QR'sinin pikselleri, kayıtlar ve yapılandırma yolu değişmedi.
- `Validate-Windows.cmd` / `.ps1` aynı kaldı.

## Windows doğrulama

1. Eski çalışan EXE'yi saklayın.
2. Yeni ZIP'i ayrı bir kısa dizine çıkarın.
3. `Validate-Windows.cmd` çalıştırın. `cargo check`, `cargo test`, `cargo build --release` sırayla tamamlanmalı.
4. Settings penceresinde normal ve dar boyutları, About'u ve uzun Animation formunu test edin.
5. Display'de fare imlecini parlaklık kaydırıcısına **tıklamadan** getirin ve tekerleği çevirin: değer değişmemeli; kaydırıcıyı sürükleyince değişmeli.
6. Alt sol Quit düğmesinin bütün sekmelerde görünmesini ve çalışmasını doğrulayın.

## Test sınırı

Kaynak düzeyinde, geometri hesaplarında ve Go testlerinde kontrol yapılmıştır; bu ortamda Windows Rust derlemesi ve Win32 grafik arayüzünün gerçek çalışma testi mümkün olmamıştır. Bu nedenle yeni kaynak henüz kararlı sürüm olarak etiketlenmemelidir.
