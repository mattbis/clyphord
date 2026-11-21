@ECHO OFF
SETLOCAL
PUSHD "%~dp0.." || EXIT /B 1
cargo.exe build --locked --bin clyphord
SET "result=%ERRORLEVEL%"
POPD
EXIT /B %result%
