# GxOled Studio / UI design development snapshot

Bu paket, son çalışan `GxOled-Dark-UI-Redesign-2026-10-25-source.zip`
paketi esas alınarak hazırlanmıştır. GitHub'daki v0.4.0 Light release'e dokunulmadı.

## Classic GIF korunuyor
- GIF Studio sekmesindeki `Classic GIF`, önceki `try_decode_gif_frames`
  çözümleme ve OLED gönderme sistemini hiç değiştirmeden kullanır.
- `Studio GIF` ayrıca 128×64 çözünürlüğe dönüştürülmüş karelerle çalışır.
- Kullanılmayan GIF kareleri bellekten çıkartılmaya devam edilir; GIF
  kapalıyken ayrı bir Studio arka plan süreci bulunmaz.

## Studio GIF / mevcut özellikler
- 128×64 GIF decode, animasyon kare sürelerinin korunması ve hız çarpanı.
- Kaynak yerleşimi: contain / cover / stretch.
- Rotation (0/90/180/270), yatay/dikey çevirme, zoom ve X/Y ofsetler.
- Siyah/beyaz arka plan, kaynak ve çıktı renklerini ters çevirme.
- Brightness, contrast, gamma, normalize ve threshold.
- Floyd–Steinberg, Atkinson, Bayer, Threshold ve None dönüşümleri.
- Kaynak GIF karelerinin ilk/son aralıkta kesilmesi ve minimum kare süresi.
- GUI'de Classic/Studio seçimi, 4 başlangıç profili, fit, dither, invert,
  threshold/brightness ve JSON ayar içe/dışa aktarımı.
- Gelişmiş parametreler `gxoled-studio.json` dosyasında değiştirilebilir ve
  GIF Studio > Import Studio settings ile içe alınabilir.

ÖRNEK (JSON):

```json
{
  "fit":"Contain", "dither":"FloydSteinberg",
  "rotate_quarters":0, "flip_h":false, "flip_v":false,
  "zoom":1.0,"offset_x":0,"offset_y":0,
  "white_background":false,"invert_source":false,
  "brightness":0.0,"contrast":0.0,"gamma":1.0,
  "normalize":false,"threshold":128,"invert_output":false,
  "playback_speed":1.0,"minimum_frame_ms":30,
  "trim_start":0,"trim_end":null
}
```

## Tasarım sistemi
- Yeni varsayılan tasarım: **Studio Orange**; ek seçenekler Studio Cyan ve
  Studio Violet. Bunlar kaydedilen vurgulu tasarım profilleridir. Light/Dark
  renk modu ayrıca korunur.
- OLED preview gerçek 128×64 bitmap verisini kullanmaya devam eder.
- `Animations` sayfası kaydırıldığında LISTBOX kontrolü açıkça yeniden çizilir.
- Aşağıdaki modların biçimleri ileride yeni görsel tasarımlarla genişletilebilir;
  şu an birincil fark renk/vurgu sistemidir, tüm ekranın ayrı UI uygulaması değildir.

## Henüz tamamlanmamış GGIF özellikleri
Bu pakete tam GGIF-Studio editörü taşınmış DEĞİLDİR. Özellikle video
(mp4/webm/mov/m4v), çok katmanlı tasarım sahnesi, sahne öğelerini fareyle
taşıma, zaman çizelgesi, kare bazında kes-yapıştır, kaynak medya koleksiyonu ve
cihaz üzerinde eşzamanlı scrubbing **yoktur**. Bunlar ileriki geliştirme
çalışmasıdır. Electron/Node, SteelSeries GameSense ve ayrı sunucu kurulumu
GxOled'e eklenmemiştir.

## Windows doğrulaması

`Validate-Windows.cmd` çalıştırılarak `cargo check`, testler ve release build
Windows ortamında doğrulanmalıdır. Bu çalışma ortamında Rust/Cargo bulunmuyor;
Windows EXE çalıştırma testi yapılmadı. Doğrudan kararlı yayın yapmayın.

## Kaynaklar ve lisans
- GGIF-Studio, D4rkTT: https://github.com/D4rkTT/GGIF-Studio (MIT)
- Kaynak bildirim dosyası: `docs/studio/THIRD_PARTY_NOTICE_GGIF_STUDIO.txt`
- GxOled'in asıl GPLv3 lisansı korunmuştur.
