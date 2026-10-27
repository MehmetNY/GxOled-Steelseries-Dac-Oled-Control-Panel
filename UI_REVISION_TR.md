# GxOled — Windows UI ve profil revizyonu (24 Ekim 2026)

Bu paket **GxOled-UI-Unit-Test-Fix** (kullanıcının başarılı derlediği sürüm) temelinde hazırlanmıştır. Orijinal dört Go animasyon uygulaması ve yerleşik `gxoled-animations.exe` değiştirilmedi.

## Güncellemeler

- **Uygulama penceresi:** Kullanılabilir pencere boyutuna ve Windows DPI değerine göre yeniden yerleşim; kaydırılabilir sekmeler, görünür dikey kaydırma çubuğu, küçük ekranlarda önizlemenin aşağıya taşınması. İlk pencere boyutu ekran alanına göre sınırlandırılır. 640×480 için tasarlanmıştır; gerçek 480p monitörde test edilmemiştir.
- **Genel:** Varsayılan tema koyu; Tray / Window / Hybrid; English / Türkçe; Windows başlangıcı, kapanış davranışı ve çıkış düğmesi.
- **Saat adları:** `Digital Original` → `Digital Standard`; `Casio F-91W` → `Casio Digital Dial`. Kaydedilmiş ayarlardaki Rust enum değerleri **değiştirilmemiştir**.
- **Animasyon profilleri:** Dört orijinal motor için sınırsız sayıda JSON konfigürasyonu kopyalama, içe aktarma, dışa aktarma, silme, seçme. Profil dosyaları `%APPDATA%\GxOled\profiles\custom\` klasöründedir. Bu konum artık **Animations** sekmesinde küçük ve İngilizce/Türkçe dil seçimine uyumlu bir not olarak da gösterilir. Profil adı otomatik artan şekilde oluşturulur. Kullanıcıdan Go yüklemesi veya tarayıcı gerekmez. Yeni motor türü ekleme ayrı bir geliştirmedir.
- **Advanced Editor:** Koyu renkli Consolas kod alanı, JSON doğrulama ve düzgün biçimlendirme, Import/Export, Save & Apply ve Reset. `refresh_rate_ms` 10–1000 ms, ekran 128×64, tam bir desteklenen widget olmak zorundadır.
- **Display:** OLED Screen Shift (Off/Low/High), Clock Burn-in Protection (Off/Random Movements/No overlapping Pixels), 10–100% parlaklık, idle screensaver, bağlantı durum simgesi seçimleri.
- **Media:** `Clock + media` ve `Full-screen media` düzenleri. Casio kadranı ortak medya düzeninden hariçtir. Ortak düzende üst saat küçülür; alt 128×24 alan şarkı/sanatçı bilgisini kırpar, taşan metin saat üzerine kaymaz. Genel screen shift ve clock burn-in konfigürasyonları korunur.
- **About:** GitHub kullanıcı/depo sayfası bir butonla açılır. Bağış görseli gönderilen fotoğraftan **QR ve adres bölgesi piksel değiştirilmeden kırpılarak** uygulamaya gömülmüştür; kullanıcı tarafından değiştirilmez.
- **GIF:** Add GIF ve Start/Stop mevcut pencerenin Animations sayfasında da bulunur.

## Kontroller

Windows kaynak klasöründen `Validate-Windows.cmd` dosyasını çalıştır. `cargo check`, `cargo test` ve `cargo build --release` sırasıyla yürütülür. Bu geliştirme ortamında Rust/Cargo kurulu olmadığından bu aşamalar burada çalıştırılamamıştır. Gerçek Windows/GxOled donanımıyla görünüm ve davranış testleri yapılmalıdır.

## Bilinen sınırlar

- Koyu tema, Win32 sistem temasıyla birleşen yerel kontroller kullanır; sistem sürümüne göre bazı düğme görünümleri farklı olabilir.
- 640×480 ve yüksek DPI için yerleşim mantığı eklendi ancak çeşitli donanım/Windows sürümlerinde görsel doğrulama gerektirir.
- Profil düzenleyicisi gelişmiş JSON doğrulama/biçimlendirme aracıdır; SteelClock-Go'nun tarayıcı formundaki bütün slider'ların eşdeğeri henüz değildir.
- Windows medya oturumunu raporlamayan oynatıcılar için ayrı entegrasyon eklenmedi. Animasyonlar aktifken medya gösterilmez.
