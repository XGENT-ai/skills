@echo off
setlocal
node "%~dp0phoenix-launcher.cjs" %*
exit /b %errorlevel%
