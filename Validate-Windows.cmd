@echo off
setlocal
cd /d "%~dp0"
echo GxOled - Windows source validation
echo.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Validate-Windows.ps1"
set "STATUS=%ERRORLEVEL%"
echo.
if not "%STATUS%"=="0" (
    echo VALIDATION FAILED - exit code %STATUS%
) else (
    echo VALIDATION FINISHED SUCCESSFULLY
)
echo.
echo Press any key to close this window.
pause >nul
exit /b %STATUS%
