@echo off
rem GxOled ses-OSD testi, tur 1: NORMAL davranis (2.5 sn bekleme) + kare numarasi damgasi.
rem Bu dosya ASCII tutuldu (Turkce karakter yok) ki cmd.exe kod sayfasi sorun cikarmasin.
setlocal
cd /d "%~dp0"
echo.
echo ================================================================
echo  TUR 1 - VARSAYILAN MOD + KARE DAMGASI
echo ================================================================
echo  Baslamadan once: sistem tepsisindeki GxOled simgesine sag tikla,
echo  "Cikis / Quit" de. Calisan baska GxOled/ggoled kalmamali.
echo.
rem Internetten indirilen dosyalardaki "engellendi" isaretini kaldir (SmartScreen sormasin).
powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-ChildItem -LiteralPath '%~dp0' -Recurse | Unblock-File" 2>nul
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Test-VolumeOSD.ps1" -ExePath "%~dp0gxoled.exe" -Mode default -Stamp
echo.
echo Test bitti. Log klasorunu acmak icin 3-Loglari-ac.cmd dosyasini cift tikla.
pause
