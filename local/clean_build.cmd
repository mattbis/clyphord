@ECHO OFF
SETLOCAL
PUSHD "%~dp0.." || EXIT /B 1
cargo.exe clean --target-dir "./var/build"
IF ERRORLEVEL 1 GOTO finish
CALL "%~dp0build.cmd"
IF ERRORLEVEL 1 GOTO finish
CALL "%~dp0build_gui_test.cmd"
IF ERRORLEVEL 1 GOTO finish
cargo.exe test --workspace --locked
:finish
SET "result=%ERRORLEVEL%"
POPD
EXIT /B %result%
