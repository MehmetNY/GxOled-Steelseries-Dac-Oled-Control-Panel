# GxOled · Koyu Tema UI Yeniden Tasarım — 25 Ekim 2026

## Tasarım kaynağı

- Kullanıcının paylaştığı **GxOled UI/UX Design System** görselinden koyu kartlar, turkuaz/mor vurgu renkleri ve canlı OLED önizlemesi ilham alındı.
- D4rkTT/GGIF-Studio (https://github.com/D4rkTT/GGIF-Studio) incelendi. Katmanlar, timeline, gelişmiş GIF dönüşümleri sonraki GIF Studio işlevsel aşaması için referans olarak not edildi; bu kaynaklar bu revizyonda kopyalanmadı.
- Görseldeki örnek **batarya yüzdesi, firmware sürümü, bağlantı gücü** gibi değerler gerçek uygulama kaynaklarında doğrulanmadığından eklenmedi.

## Yapılan somut değişiklikler

1. Windows'un klasik açık renk düğmeleri yerine **yerel Win32/GDI owner-draw** tekniğiyle koyu, ince kenarlıklı, yuvarlatılmış gezinme ve eylem düğmeleri. Mevcut radio/checkbox davranışı korunur.
2. Arayüz kartları: solda kategori gezintisi, ortada ayar paneli, sağda **canlı 128×64 OLED önizleme**. Tema renkleri koyu lacivert/siyah, turkuaz ve mor. Light tercihi tutulur.
3. Sekme sırası: **General → Clock → Animations → GIF Studio → Display → Media → About**. Mevcut sayfa/komut kimlikleri değişmedi.
4. Clock Gallery: Dijital, analog ve Casio seçimleri iki sütunlu mini kartlarda stil örnekleriyle sunulur. Kartlar açıklayıcıdır; sağdaki ekran gerçek önizlemedir.
5. JSON Advanced Animation Editor: `Ctrl+A` eskisi gibi çalışır. `WM_GETMINMAXINFO` ile **850×470 minimum pencere boyutu** uygulanır. Alt beş eylem düğmesine eşit genişlik ve sabit aralık verilir; yazının sıkışması engellenir.
6. Pencere stilleri mevcut kararlı temel korunarak uygulanmıştır. **Ana ayar penceresi 950×670 sabit** tutulur; geçmişteki pencere yeniden boyutlandırma hataları geri getirilmez.
7. Kaydırma ve profiller, GIF oynatma, Go render yardımcısı, OLED hid bağlantısı, Casio saat, parlaklık ve burn-in koruması değiştirilmedi.

## Bilinen sınırlar / test yöntemi

- Rust/Windows derleyicisi bu çalışmanın yapıldığı ortamda mevcut değildi. **Cargo check/test/build ve gerçek Win32 görünüm testi yapılmadı.** Kaynak paketini GitHub release olarak yayımlamadan önce kendi Windows cihazında çalıştır.
- Öncelikle `Validate-Windows.cmd` ile check → test → release işlemlerini çalıştır, sonra `target/release/gxoled.exe`yi aç.
- GUI'de yedi sekmeyi, soldaki seçili sekme vurgusunu, sağdaki OLED önizlemesini, profil seçimlerini, Dark/Light geçişini, 850 piksel altına indirilemeyen JSON editörü penceresini ve GIF oynatmayı dene.
- Kaynak yapısı ve ZIP/patch karşılaştırması yerel olarak test edilir, ancak Windows GUI/cihaz testi yerine geçmez.
- GGIF Studio'daki **timeline, filtre, katman ve video düzenleme** araçları bu sürüme dahil değildir. Bu sürüm **GxOled ayar arayüzünün tasarım yenilemesidir**.
