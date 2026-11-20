@ECHO OFF
SETLOCAL
PUSHD "%~dp0.." || EXIT /B 1
cargo.exe build --locked --release --bin aiclip
SET "result=%ERRORLEVEL%"
POPD
EXIT /B %result%
