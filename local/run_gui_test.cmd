@ECHO OFF
SETLOCAL
PUSHD "%~dp0.." || EXIT /B 1
cargo.exe run --manifest-path "%~dp0..\test\gui_framework\Cargo.toml"
SET "result=%ERRORLEVEL%"
POPD
EXIT /B %result%
