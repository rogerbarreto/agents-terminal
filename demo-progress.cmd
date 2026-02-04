@echo off
REM demo-progress.cmd
REM Demonstrates PTUI protocol with a progress bar
REM Run this INSIDE the Pixel Terminal

echo PTUI Progress Bar Demo
echo ======================
echo.

REM Note: We use a helper to send the escape sequences
REM In cmd.exe, we can use powershell to output the escape codes

echo Starting download simulation...
echo.

REM Create progress bar
powershell -Command "Write-Host -NoNewline ([char]0x1B + ']1337;PTUI={\"type\":\"progress\",\"id\":\"dl\",\"value\":0,\"max\":100,\"label\":\"Downloading\"}' + [char]0x07)"

REM Update progress in a loop
for /L %%i in (10,10,100) do (
    timeout /t 1 /nobreak >nul
    powershell -Command "Write-Host -NoNewline ([char]0x1B + ']1337;PTUI={\"update\":\"dl\",\"props\":{\"value\":%%i}}' + [char]0x07)"
)

echo.
echo Download complete!

REM Clear UI
powershell -Command "Write-Host -NoNewline ([char]0x1B + ']1337;PTUI={\"clear\":true}' + [char]0x07)"

echo.
echo Demo finished.
