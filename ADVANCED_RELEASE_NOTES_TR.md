# GxOled Advanced v0.5.0 — kurtarma temeli

- Esas kaynak: kullanıcının 25 Ekim 2026 tarihli Mavi arayüz ZIP dosyası.
- Stable sürümde yeni GIF Studio/editör arayüzü yoktur.
- `gxoled_app/src/settings_window.rs` içindeki `E0382` (taşınan `Rect`) hatası, yalnızca ilgili dikdörtgenin değerleri ayrı geçirilerek giderildi. Arayüz görünümü değiştirilmedi.
- `Validate-Windows.ps1`, başarılı derlemeden sonra `release/GxOled-Advanced-v0.5.0/` ve ZIP paketi oluşturur.
- `.github/workflows/advanced-release.yml`, kaynak derlenirse `v0.5.0` başlıklı kararlı GitHub Release oluşturur.
- `.github/workflows/advanced-prerelease.yml`, `development/advanced-*` üzerindeki her push için test/derleme başarılı olursa prerelease EXE yayımlar.
- Ana dal / mevcut yayımlanmış sürümler değiştirilmez.
