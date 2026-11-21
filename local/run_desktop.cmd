@ECHO OFF
SETLOCAL
PUSHD "%~dp0.." || EXIT /B 1
cargo.exe run --locked --bin clyphord -- --gui %*
SET "result=%ERRORLEVEL%"
POPD
EXIT /B %result%
