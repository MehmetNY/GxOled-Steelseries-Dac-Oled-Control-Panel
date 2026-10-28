@echo off
rem Test kayitlarinin (log) bulundugu klasoru Windows Gezgini'nde acar.
if not exist "%TEMP%\GxOled-OSD-Diagnostics" mkdir "%TEMP%\GxOled-OSD-Diagnostics"
start "" explorer.exe "%TEMP%\GxOled-OSD-Diagnostics"
