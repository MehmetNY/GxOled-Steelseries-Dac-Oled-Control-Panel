@echo off
rem GxOled ses-OSD otomatik deney dongusu (GXOLED_OSD_LAB). Dosya bilerek ASCII (Turkce karakter yok).
setlocal
cd /d "%~dp0"
echo.
echo ================================================================
echo  OTOMATIK DENEY DONGUSU
echo ================================================================
echo  1) Telefon kamerasini OLED'e sabitle ve KAYDI BASLAT (60 fps).
echo  2) OLED'in sag alt kosesindeki 4 haneli sayinin ILK hanesi,
echo     siradaki deney numarasidir (0..9). Elle bir sey secmen gerekmez.
echo  3) Saat ekrana geri geldikten sonra 5 saniye bekle, sonra dugmeyi
echo     TEK SEFERDE birkac tik cevir. Bunu en az 10 kez tekrarla.
echo  4) Bitince sistem tepsisinden GxOled simgesine sag tikla, "Cikis/Quit" de.
echo     (Cikarken cihaz ayarlari otomatik eski haline getirilir.)
echo.
echo  Baslamadan once calisan GxOled/ggoled varsa tepsiden kapat.
echo.
powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-ChildItem -LiteralPath '%~dp0' -Recurse | Unblock-File" 2>nul
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Test-VolumeOSD.ps1" -ExePath "%~dp0gxoled.exe" -Mode default -Lab
echo.
echo Test bitti. Kayitlari acmak icin 2-Loglari-ac.cmd dosyasina cift tikla.
pause
