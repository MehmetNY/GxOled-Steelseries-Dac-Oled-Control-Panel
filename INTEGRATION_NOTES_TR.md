# GxOled / SteelClock-Go — yeniden entegrasyon (deneysel)

Bu dal, **kullanıcının gönderdiği orijinal GxOled v0.4.0** kaynak kodu üzerine kurulmuştur. Hatalı ilk Rust animasyonları kullanılmadı. Windows uygulamasının mevcut SteelSeries bağlantısı, OLED parlaklık denetimi, ses kadranına 2,5 sn öncelik verme, GIF, saat ve config kayıt sistemi korunmaya çalışılmıştır.

## Değişenler

1. `go_animations/` içinde SteelClock-Go v1.7.6.2 ile birlikte gönderilen dört widget'in **orijinal Go kaynak dosyaları** yer alır: `matrix`, `hyperspace`, `starwarsintro`, `gameoflife`. Bu dosyaların animasyon mantığına müdahale edilmedi. Font glifleri, çizim ve Floyd–Steinberg dönüşümü de upstream Go kodundan alındı. Kaynak profiller release ZIP'indeki `profiles/128x64/*.json` dosyalarıdır.
2. Go yardımcısı (`gxoled-animations.exe`) **GxOled ikili dosyasına gömülür**, ilk kullanımda GxOled önbelleğine çıkarılır. Ayrı Go kurulumu veya SteelClock uygulaması çalıştırılması **gerekmez**. Go yardımcı işlemi yalnızca seçili animasyon esnasında çalışır; değiştirme, durdurma, idle veya GxOled çıkışında kapatılır. Yardımcı **USB/HID aygıtına erişmez**; yalnızca 1024 baytlık MSB-first monokrom kareler gönderir. OLED'in tek sahibi Rust GxOled kalır.
3. Animasyon menüsünde `Animation > SteelClock 128x64` seçimi, yerel pencerede `Animations` sekmesi bulunur. `Add GIF` eskisi gibi devam eder. Kullanılmayan GIF kareleri başlangıçta decode edilmez, oynatma kapatıldığında bellek bırakılır.
4. `Clock style > Digital > Casio F-91W` seçeneği kullanıcı tarafından verilen **128×64 referans ekran yerleşimine** göre yeniden oluşturulmuştur. Büyük saat/dakika, üstte Türkçe gün ve ayın günü, dört hafta göstergesi, altta saniyeler ve 60 ayrı ilerleme çizgisi bulunur. Grayscale-dithering kullanmaz. Bu saat diğer saatlerle aynı `DrawLayer::Clock` güncellemesi ve `Screensaver when idle` temizliği içindedir.
5. `gxoled_app/src/settings_window.rs` Windows'un yerel Win32 penceresini sunar: kategori sekmeleri, saat/animasyon/radyo tercihleri, idle/media seçimleri ve 128×64 büyütülmüş önizleme. Harici tarayıcı/web sunucusu kullanılmaz. Pencere ve seçimler yapılandırmaya kaydedilir.

## Pencere / tray sınırlaması (önemli)

Mevcut SDL3 tray icon API'si simgeye **sol tıklamayı ayrı olay olarak bildirmediği için**, bu sürümde tray ikonuna tıklandığında açılan native menü henüz tamamen kaldırılamadı. `Open Settings...` menüsü pencereyi açar. `Settings UI mode` seçeneklerinden `Window` pencereyi başlangıçta otomatik açar; `Hybrid` seçeneğinde pencere manuel açılır, hızlı tray menü kalır; `Tray` eski varsayılan davranıştır. **Sol tıkla doğrudan pencere açılma özelliği henüz tamamlanmış değildir.** Bunun için tray simgesinin Win32 bildirim sistemi üzerinden sahiplenildiği ayrı bir revizyon gerekir.

Şimdilik pencerenin `General`, `Clock`, `Animations`, `Display`, `Media`, `About` başlıkları işlevseldir; brightness/burn-in ayarları eski tray menüsünde kalır. Bu bir ilk pencere altyapısı sürümüdür, eksiksiz bütün ayarların taşındığı son tasarım değildir.

## Derleme (Windows / PowerShell)

Rust toolchain, CMake, Visual Studio C++ build tools kurulu varsayılır. Go kurulu **olmak zorunda değildir**, çünkü kaynak paketi Windows x64 için Go renderer `.exe`'sini zaten içerir.

```powershell
cargo check --locked -p gxoled_app --features sdl3-static
cargo test --locked -p gxoled_draw -p gxoled_app --features gxoled_app/sdl3-static
cargo build --locked --release -p gxoled_app --features sdl3-static
```

Hedef: `target\release\gxoled.exe`. Gömülü Go executable `%LOCALAPPDATA%\GxOled\renderer\gxoled-animations.exe` klasörüne yalnızca animasyon başlatıldığında çıkar.

Go kaynaklarını kendin değiştirmek istersen (isteğe bağlı, Go gerektirir):

```powershell
cd go_animations
go test ./...
go build -trimpath -ldflags="-s -w" -o gxoled-animations.exe ./cmd/renderer
cd ..
cargo build --locked --release -p gxoled_app --features sdl3-static
```

Eski GxOled ayarları `%APPDATA%\GxOled\config.toml` altında kalır. Önce ayar dosyanın kopyasını al.

## Doğrulama sınırları

Bu geliştirme ortamında **Rust/Cargo yok** ve Windows/SteelSeries aygıtı bulunmuyor. Dolayısıyla bu sürümdeki Win32 Rust penceresi ve GxOled ana uygulaması için gerçek `cargo check`, `cargo test`, `cargo build` veya donanım testi **çalıştırılmış değildir**. `go test ./...` (upstream animasyon testleri dahil), 128×64 kaynak profillerinden kare üretimi, Windows amd64 Go renderer çapraz derlemesi, yapılandırma/ZIP ve değişiklik kontrolleri yapılmıştır. Bu kaynak paketi **deneysel** olarak değerlendirilmelidir; release yayımlamadan önce yukarıdaki Cargo kontrolleri ve OLED karşılaştırması zorunludur.

## Tasarım sınırları

- OLED'in 128×64 tek bitlik aktarımı değişmedi. Go renderer orijinal SteelClock `ImageToBytes` ve Floyd–Steinberg'i kullanır; Rust yalnızca gelen bitleri gösterir.
- Casio F-91W tam ekran yerleşimi nedeniyle burn-in için tüm ekranın taşınması mümkün değildir. Idle/sleep kapanışı mevcut saat davranışıyla aynıdır.
- Üst dört nokta 1–4. haftayı gösterir; ayın 29–31. günleri dördüncü nokta altında gruplandırılır.

## Lisans ve kaynak

GxOled önceki `JerwuQu/ggoled` kaynaklarına dayalı GPL-3.0 çalışmadır. Bu entegrasyondaki dört Go widget, glif ve ilgili çizim kodları: `https://github.com/pozitronik/steelclock-go` (GPL-3.0). Kaynak dosyalardaki kodların kaynağı ve GPL-3.0 yükümlülükleri korunur. Kullanıcının sağladığı SteelClock v1.7.6.2 `profiles/128x64/` JSON'ları kullanılır.
