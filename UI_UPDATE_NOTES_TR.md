# GxOled arayüz revizyonu — deneysel

Bu paketin temeli, çalışan **Go animasyonlu Reset paketi**dir. Matrix, Hyperspace, Star Wars Intro ve Game of Life'ın özgün Go algoritma dosyaları değiştirilmemiştir. Go yardımcı işlemindeki ekleme yalnızca kullanıcı tarafından düzenlenen profil JSON dosyasını yükleme ve doğrulama işlevleridir.

## Açılış biçimi

- **Tray:** Simgeye sol/sağ tıklanınca eski menü açılır.
- **Window:** Simgeye sol/sağ tıklanınca ayar penceresi açılır.
- **Hybrid:** Sol tık pencere, sağ tık eski menü.

Bu özellik yalnızca Windows için ve SDL3 3.4'ün `SDL_TRAY` / `WM_USER+1` davranışına göre yapılmış deneysel Win32 entegrasyonudur. SDL3 güncellenirse tekrar doğrulanmalıdır.

## Saat

- `Digital Big` yazı boyutu arttırıldı; mevcut PixelOperator tabanı korundu.
- Casio ekranında daha boşluklu rakamlar, yeni `1`, her saniye yanıp sönen iki nokta ve 60 saniye çizgisi bulunur.
- Casio için `Clock burn-in protection` rastgele pozisyon uygulanmaz; `OLED screen shift` bağımsızdır.
- `English` varsayılan, `Türkçe` isteğe bağlı. Casio gün adları ve dijital saatte 12 saatlik `AM/PM` / `ÖÖ/ÖS` biçimi buna bağlıdır.

## Medya

`Media screen (clock off)` yalnızca `Time → Off` seçiliyken ve animasyon çalışmıyorken parça bilgilerini gösterir. Analog/dijital saatlerle artık birbirinin üzerine çizilmez. Bu henüz ayrı gelişmiş medya widget'ı değildir.

## Animasyon profil düzenleyicisi

Ayar penceresinde Animations bölümünden istenen animasyonu seç, `Edit animation profile...` ile özgün 128×64 JSON profilini aç. Değişiklikleri `Save and apply` ile uygula; `Restore original` ile sıfırla. Kullanıcının profil dosyaları `%APPDATA%\GxOled\profiles` altında tutulur. Düzenleyici tarayıcı kullanmaz. Özgün SteelClock web düzenleyicisinin görsel slider/form özelliklerinin **tam karşılığı henüz değildir**; şimdilik tüm profil alanlarını düzenleyebilen yerel JSON editörüdür.

## About ve bağış QR

GitHub: https://github.com/MehmetNY/GxOled-v0.4.0

Yayıncının sağladığı gerçek USDT/TRON bağış görseli artık `gxoled_app/assets/donation-usdt-tron.png` dosyasında uygulamaya gömülüdür. **About** bölümünde doğrudan görüntülenir; kullanıcı QR görselini değiştiremez. Eski `%APPDATA%\GxOled\donation-qr.png` dosyası kullanımdan çıkarılmıştır.

## Kontrol

Windows'ta kaynak klasöründe `powershell -ExecutionPolicy Bypass -File .\Validate-Windows.ps1` komutunu çalıştır. Rust derleme ve donanım testi bu hazırlama ortamında yapılamadı; GitHub için kararlı sürüm kabul etmeden önce bu kontrol gereklidir.
