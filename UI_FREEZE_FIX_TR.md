# GxOled — Windows ayar penceresi kilitlenme düzeltmesi

Bu paket, **GxOled-UI-Revision-Profile-Note-source.zip** sürümünün üzerine kurulmuştur.

## Sorunun kaynağı

Win32 `ShowWindow` / `SetWindowPos` / `SetScrollInfo` çağrıları pencere mesajlarını eşzamanlı tetikleyebilir. Önceki kod, `STATE` adlı `Mutex` kilidini tutarken `ShowWindow` çağırıyordu. Bunun sonucunda `WM_SIZE` -> `layout_window` aynı kilide tekrar erişmeye çalışıyor ve UI mesaj döngüsü kilitlenebiliyordu. Görünür semptomlar: Ayarlar penceresinde "Yanıt Vermiyor", tıklanamayan kontroller ve boyanmamış alanlar.

## Düzeltmeler

1. `open()` artık pencere tutamacını (HWND) kilit altındayken kopyalayıp kilidi **önce bırakıyor**, `ShowWindow` işlemini sonra yapıyor.
2. `layout_window()` bütün koordinatları kilitliyken hesaplıyor, ancak Win32 yerleşim işlemlerini kilit bırakıldıktan sonra yapıyor.
3. İç içe `WM_SIZE`/yeniden yerleşim çağrılarına karşı `LAYOUT_ACTIVE` koruması eklendi.
4. Sekme değiştirme, tema, dil ve seçim kontrollerinin güncellenmesi aynı kurala uyarlandı.
5. Animasyon JSON editörü açma ve modal hata pencereleri sırasında benzer kilit riskleri azaltıldı.

## Değişmeden korunanlar

- SteelClock-Go Go animasyon motoru ve gömülü Windows render yardımcısı
- Mevcut saatler / Casio Digital Dial
- GIF, medya ve OLED sürücü/iletişimi
- Dil/tema/animasyon profilleri ve bağış QR'si
- `%APPDATA%\\GxOled` ayar dosyaları (silinmez veya sıfırlanmaz)

## Derleme ve kontrol

Kaynak klasöründe `Validate-Windows.cmd` dosyasını çalıştırın. Bu komut sırasıyla `cargo check`, `cargo test` ve release build yapar. Başarılı sonuç **GUI'nin çalışma zamanında yanıt verdiğini tek başına kanıtlamaz**.

Ardından:

1. Görev Yöneticisi'nden önceki kilitli `gxoled.exe` işlemini sonlandırın.
2. `target\\release\\gxoled.exe` dosyasını çalıştırın.
3. Tray ikonundan Settings penceresini açın ve sekmeler arasında birkaç kez geçiş yapın.
4. Pencereyi küçültüp büyütün, Light/Dark ve English/Türkçe arasında değiştirin.
5. Animasyon profili editörünü açıp kapatın ve yeniden açın.
6. Birkaç dakika OLED animasyonu açıkken ayar penceresinin yanıt verdiğini kontrol edin.

ÖNEMLİ: Bu ortamda Cargo/Windows GUI mevcut olmadığından bu sürüm Windows üzerinde doğrudan derlenip test edilmedi. Kaynak düzeyinde kontrol, Cargo manifest doğrulaması ve arşiv bütünlüğü testleri yapıldı.
