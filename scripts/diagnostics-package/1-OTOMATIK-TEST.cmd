@echo off
rem GxOled ses-OSD dogrulama dongusu (GXOLED_OSD_LAB). Dosya bilerek ASCII (Turkce karakter yok).
setlocal
cd /d "%~dp0"
echo.
echo ================================================================
echo  OTOMATIK DOGRULAMA (6 dugme cevirisi, yaklasik 3 dakika)
echo ================================================================
echo  1) Telefon kamerasini OLED'e sabitle ve KAYDI BASLAT (60 fps).
echo  2) OLED'in sag alt kosesindeki 4 haneli sayinin ILK hanesi siradaki
echo     deney numarasidir (0..5). Elle bir sey secmen gerekmez.
echo  3) Saat ekrana geldikten sonra 5 saniye bekle, sonra dugmeyi TEK
echo     SEFERDE 3-5 tik cevir, elini cek. Saat geri gelsin, en az 10
echo     saniye bekle ve bunu TOPLAM 6 KEZ tekrarla.
echo  4) Bitince sistem tepsisinden GxOled simgesine sag tikla, "Cikis/Quit" de.
echo.
echo  Bu test DAC'in ana ekranini "ayrintili" moda geri alir (onceki deney
echo  onu "basit" yapmisti) ve Sonar bayragini acik birakir.
echo  Baslamadan once calisan GxOled/ggoled varsa tepsiden kapat.
echo.
powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-ChildItem -LiteralPath '%~dp0' -Recurse | Unblock-File" 2>nul
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Test-VolumeOSD.ps1" -ExePath "%~dp0gxoled.exe" -Mode default -Lab
echo.
echo Test bitti. Kayitlari acmak icin 2-Loglari-ac.cmd dosyasina cift tikla.
pause
