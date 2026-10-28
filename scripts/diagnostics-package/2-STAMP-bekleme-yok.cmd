@echo off
rem GxOled ses-OSD testi, tur 2: bekleme YOK (kareler ~200 ms sonra gonderilir) + kare numarasi damgasi.
setlocal
cd /d "%~dp0"
echo.
echo ================================================================
echo  TUR 2 - BEKLEME YOK (no-hold) + KARE DAMGASI
echo ================================================================
echo  Bu mod urun ozelligi DEGIL; sadece cihazin kareleri atip atmadigini
echo  gormek icin kullanilan bir olcum modudur.
echo  Baslamadan once: sistem tepsisindeki GxOled simgesine sag tikla,
echo  "Cikis / Quit" de. Calisan baska GxOled/ggoled kalmamali.
echo.
powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-ChildItem -LiteralPath '%~dp0' -Recurse | Unblock-File" 2>nul
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Test-VolumeOSD.ps1" -ExePath "%~dp0gxoled.exe" -Mode no-hold -Stamp
echo.
echo Test bitti. Log klasorunu acmak icin 3-Loglari-ac.cmd dosyasini cift tikla.
pause
