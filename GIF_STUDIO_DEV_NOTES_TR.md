# GxOled Advanced — GIF Studio geliştirme dalı

- Kaynak: v0.5.0 kararlı Classic/Mavi arayüz temeli.
- Mevcut `settings_window.rs` arayüzü kesinlikle değiştirilmedi.
- Bildirim alanında Animasyon > `Open GIF Studio Editor...` düğmesi eklendi.
- Çok katmanlı Studio düzenleyicisi Chrome/Edge bağımsız uygulama penceresinde açılır. Medya içe alma, önizleme, kare zaman çizelgesi, katman sırası, kırpma, efektler, proje kaydetme ve animasyonlu GIF dışa aktarımı vardır.
- `Send to GxOled` yerel token korumalı loopback protokolünü kullanır, yalnızca 128×64 GIF aktarır; cihazla yazışmayı mevcut tek GxOled HID motoru sürdürür.
- Eski GIF çözümleyicisi ve ana OLED görünümü değiştirilmez.
- Bu dal bir prerelease geliştirme sürümüdür; fiziksel cihazda test yapılmadan kararlı olarak etiketlenmez.
