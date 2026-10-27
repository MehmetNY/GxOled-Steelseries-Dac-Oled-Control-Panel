# GxOled — Arayüz düzeni kurtarma ve kullanım notları

Bu kaynak sürümü 24 Ekim'deki son deneysel `Settings Freeze Fix` kaynak kodundan türetildi. Önceki uygulama dosyanı sakla; yeni sürümü ayrı klasörde doğrula.

## Düzeltmeler

- **Pencere genişliği:** Önizleme artık sağda açılan ikinci bir sütuna bağlı değil. Tüm pencere genişliklerinde ayarların üstünde durur. Sekme içeriği dikey kaydırılabilir. Bu nedenle büyük monitöre veya ekranı genişletmeye gerek kalmamalı.
- **Clock + Media:** Hatalı birleştirme kaldırıldı. Müzik çalarken medya bilgisi tam ekran gösterilir, oynatma bitince saat geri döner. Casio ve Go animasyonlarıyla medya üst üste çizilmez. Önceden kaydedilen ClockAndMedia ayarı açılışta güvenli MediaOnly moduna geçirilir.
- **Animation Advanced Editor:** JSON editörde satır sonları Win32 çok satırlı metin biçiminde gösterilir; dosya okunurken ve dışa aktarılırken düzenli girintileme uygulanır. Özel profiller `%APPDATA%\GxOled\profiles\custom` klasöründe kalır.
- **Display:** Parlaklık 10–100% arası, on kademeli kaydırıcıdan değiştirilir. Cihaz varsayılanı bilinmiyorsa ilk hareketle kademeyi belirlemiş olursun.
- **About:** Yalnızca tek tıklanabilir GitHub bağlantısı vardır. Mevcut gömülü bağış görseli değiştirilmedi.

## Windows doğrulaması

1. Kaynak ZIP'ini yeni bir klasöre aç.
2. `Validate-Windows.cmd` çalıştır. Cargo check ve test geçmeden release build'e geçilmez.
3. `target\release\gxoled.exe` uygulamasını aç.
4. Pencereyi 640×480 ve normal masaüstü çözünürlüğünde yeniden boyutlandır; OLED önizlemesini ve dikey sayfa kaydırmasını kontrol et.
5. Display'de slider hareketiyle parlaklık değişimini; Animations > Edit selected profile ile çok satırlı JSON'u; Media ile tam ekran medya geçişini test et.
6. About'taki QR kodunu fiziksel cihazınla tarayarak tekrar doğrula.

**Sınır:** Bu çalışma ortamında Rust/Cargo/Windows bulunmuyor. Burada Windows release derlemesi veya canlı Win32 arayüz testi gerçekleştirilmedi. Animasyon Go testleri ve kaynak/yama bütünlük kontrolleri yapıldı.
