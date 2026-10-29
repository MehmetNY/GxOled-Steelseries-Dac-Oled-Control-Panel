@echo off
rem GxOled'in normal kullanimi: aday duzeltme acik, sadece dugme zamanlari kaydedilir. Dosya bilerek ASCII.
setlocal
cd /d "%~dp0"
echo.
echo ================================================================
echo  GERCEK KULLANIM TESTI (istege bagli, yaklasik 3 dakika)
echo ================================================================
echo  Once DAC'in USB kablosunu 10 saniye cek-tak (cihaz yeniden baslar),
echo  sonra bu dosyayi calistir. Ekranda sayac YOKTUR; GxOled normal calisir.
echo  1) Kaydi baslat (60 fps). 2) Dugmeyi 10 kez cevir: bazen 1 tik, bazen
echo  uzun cevirme; her cevirmeden sonra saat gelene kadar bekle ve +5 sn daha.
echo  3) Bitince tepsiden GxOled simgesine sag tikla, "Cikis/Quit" de.
echo.
powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-ChildItem -LiteralPath '%~dp0' -Recurse | Unblock-File" 2>nul
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Test-VolumeOSD.ps1" -ExePath "%~dp0gxoled.exe" -Mode default
echo.
echo Test bitti. Kayitlari acmak icin 2-Loglari-ac.cmd dosyasina cift tikla.
pause
