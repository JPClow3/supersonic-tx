@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -no_logo
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
cd /d "H:\Code\Pessoais\SP - Solana\supersonic-tx"
cargo check -p supersonic-tx-sdk > "%~dp0task-2-red.log" 2>&1
echo EXIT:%ERRORLEVEL%
