# GxOled GIF Studio — deneme sürümü

Bu dalda 27 Ekim 2026 tarihli **Classic UI Rollback** arayüzü korunmuştur. `main` ve eski Light release değiştirilmemiştir.

- **GxOled ayarları → GIF Studio → Open GIF Studio Editor** düğmesi yerel Microsoft Edge uygulama penceresinde (Edge yoksa varsayılan tarayıcıda) bağımsız bir düzenleme görünümü açar. Studio HTML/JS kaynağı derleme sırasında EXE içine gömülür; harici sunucu veya paket kurulumuna gerek duymaz.
- GIF, resim ve desteklenen videolar eklenebilir; metin/çoklu katman, görünürlük, sıralama, sürükleyerek konumlandırma, ölçek, döndürme, opaklık, katman zamanlaması, trim, oynatma, frame-step, dithering, eşik, gamma, kontrast, parlaklık ve invert kontrolü vardır.
- **Export GIF** 128×64 tek renkli GIF89a dosyası üretir. **Save/Open project** kaynak kareleriyle birlikte proje JSON'unu yerelde saklar.
- **Send to GxOled** GIF'i sadece `127.0.0.1` üzerinden geçici, kriptografik olarak rastgele oturum anahtarıyla GxOled uygulamasına aktarır. Gönderim sonrası önceden var olan **Classic GIF** motorunda oynatılır; ikincil HID/GameSense yazıcısı başlatılmaz. Eski klasik Add GIF ve Start/Stop davranışı korunur.
- GIF işleme ve dosya çözümleme Chromium/Edge WebCodecs API'sine bağlıdır; eski tarayıcılar animasyonlu GIF'in yalnızca ilk karesini açabilir. Video dosyaları 30 saniye / 600 kare ve 30 fps ile sınırlıdır. Proje ve kaynak boyutu için güvenlik sınırları bulunur.

## Bilinen kısıtlar

Bu kaynak **D4rkTT/GGIF-Studio Electron uygulamasının birebir portu değildir**. D4rkTT projesinin katman, timeline ve render yaklaşımından esinlenen özgün çevrimdışı editördür. Video kare örnekleme ve yerel tarayıcı erişimi bilgisayardaki Edge sürümüne bağlıdır. Windows EXE ve fiziksel DAC üzerinde bu ortamda deney yapılamamıştır; GitHub Actions MSVC denetimi ve gerçek Windows kullanıcı testi gereklidir.

## Test

1. `Validate-Windows.cmd` ile Cargo check/test/release build çalıştır.
2. Ayarlarda eski görünümün korunduğunu, GIF Studio düğmesinin Editör'ü açtığını doğrula.
3. Editor'de animasyonlu GIF ekle, text layer ekle, timeline'da ilerle, Export GIF yap ve üretilen dosyayı aç.
4. GxOled açıkken **Send to GxOled** seç; OLED'de hareketi ve tray menüsünde GIF seçiminin kaydedildiğini kontrol et.
5. Ayar penceresi/Studio kapandığında saat, medya ve Classic GIF seçeneklerini yeniden test et.

Editörün JavaScript sözdizimi: `node studio/verify-editor.cjs`; Go regresyonu: `cd go_animations && go test ./...`.
