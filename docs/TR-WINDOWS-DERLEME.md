# Windows'ta kaynak koddan derleme (TR)

> **Önce şunu bil:** Sadece testi yapmak için derleme yapman GEREKMİYOR. Hazır test paketini
> [`TR-TEST-REHBERI.md`](TR-TEST-REHBERI.md) içindeki adımlarla indirip çalıştırabilirsin.
> Bu rehber, kaynak koddan kendi EXE'ni üretmek isteyenler içindir.

## 1. Gerekenler (bir kez kurulur)

| Araç | Nereden | Not |
| --- | --- | --- |
| Rust (MSVC) | <https://rustup.rs> | Varsayılan seçenekle kur. Kurduktan sonra YENİ bir pencere aç. |
| Visual Studio Build Tools | <https://visualstudio.microsoft.com/downloads/> → "Build Tools" | Kurulumda **"Desktop development with C++"** iş yükünü işaretle. |
| CMake | <https://cmake.org/download> | Kurulumda **"Add CMake to the system PATH"** seçeneğini işaretle. |
| Git (isteğe bağlı) | <https://git-scm.com> | Depoyu indirmek için; ZIP indirmek de olur. |

## 2. Depoyu KISA bir yola koy

SDL3, derleme sırasında CMake/MSBuild ile kaynaktan derlenir. MSBuild 260 karakterden uzun yollarda
şu hatayla durur:

```
error FTK1011: could not create the new file tracking log file ...
```

Bu hata kodunla ilgili değildir; **klasör yolu çok uzun olduğu için** çıkar (ör. Masaüstü içinde iç
içe `...-main\...-main\target\debug\build\sdl3-sys-...`). Önlem:

1. Depoyu `C:\gx` gibi KISA bir klasöre çıkar/klonla (ör. `C:\gx\GxOled`).
2. `Validate-Windows.cmd` zaten derleme çıktısını da kısa bir klasöre (`C:\gxb`) yönlendirir; yani
   depon biraz uzun bir yolda olsa bile bu hata engellenir. Yine de depoyu kısa yolda tutmak en güvenlisidir.

## 3. Derle

1. Depo klasöründe **`Validate-Windows.cmd`** dosyasına çift tıkla.
2. Betik sırayla şunları yapar (herhangi biri başarısız olursa DURUR):
   1. `cargo` ve `cmake` kurulu mu kontrol eder, değilse ne yapacağını söyler.
   2. `cargo check` — derlenebilir mi?
   3. `cargo test` — birim testleri geçiyor mu?
   4. `cargo build --release` — çalıştırılabilir dosyayı üretir.
3. Başarılı olursa EXE şuraya kopyalanır: **`<depo klasörü>\dist\gxoled.exe`**; SHA-256 değeri pencerede yazar.
4. İlk derleme SDL3 yüzünden uzun sürer (birkaç dakika). Pencereyi kapatma.

Derleme klasörünü değiştirmek istersen (ör. D: sürücüsü), komut isteminde:

```
set GXOLED_BUILD_DIR=D:\gxb
Validate-Windows.cmd
```

## 4. Bu çıktı neyi KANITLAR, neyi kanıtlamaz?

- Derleme + test başarısı: kodun derlendiğini ve birim testlerinin geçtiğini gösterir.
- DAC üzerinde doğru çalıştığını GÖSTERMEZ. Bunu yalnızca fiziksel test gösterir
  ([`TR-TEST-REHBERI.md`](TR-TEST-REHBERI.md)).

## 5. Sık görülen hatalar

| Belirti | Neden / Çözüm |
| --- | --- |
| `FTK1011 ... file tracking log` | Yol çok uzun. Depoyu `C:\gx` altına taşı; `Validate-Windows.cmd` kullan (çıktıyı `C:\gxb`ye alır). |
| `'cargo' was not found` | Rust kurulu değil veya pencere eski. rustup'ı kur, YENİ pencere aç. |
| `'cmake' was not found` | CMake kurulu değil / PATH'e eklenmemiş. Kurulumda "Add CMake to the system PATH" seç. |
| `link.exe not found` / `MSVC` hatası | Build Tools'ta "Desktop development with C++" kurulu değil. |
| `-Stamp` parametresi bulunamadı | ESKİ bir test klasöründeki betiği çalıştırıyorsun. Güncel paketi indir (`TR-TEST-REHBERI.md`). |
