# aiclip // actually now clyphord

Assemble files and review instructions in a named buffer, publish the whole prompt to the Windows clipboard, and capture the reply. Both modes of `clyphord.exe` share the same local SQLite database.

## Build and run on Windows

From the project root, run `.\local\build.cmd` to build `clyphord.exe`. Run `.\var\build\debug\clyphord.exe --gui` or use `.\local\run_desktop.cmd` to open the desktop workspace. It opens session `default`, buffer `review`. Refresh after staging files in another process. Choose **Compare buffers**, enter the second session and buffer, then click **Load both** to read the current contents side by side.

`.\local\clean_build.cmd` clears the shared Cargo output under `./var/build/`, rebuilds aiclip and the standalone window probe, then runs the CLI integration test. Close running project executables before cleaning. Use `.\local\build_release.cmd` to build `./var/build/release/clyphord.exe`.

## Arrange the desktop

**New tab** opens another buffer view. **Split right** opens a response view beside the focused pane. Each tab has its own session, buffer and unsubmitted instructions. Drag a tab onto another tab bar to group them, or onto a pane edge to split. Drag the divider to resize.

**Detach panel** opens the whole editor in a frameless native window. Its original tab keeps a return point. Use **Strip** for a compact Publish/Capture bar, **Expand** to resume editing, and **Pin** to keep the window on top. Drag the window by its title. On Windows, releasing within the **Snap** distance aligns it with the usable edge of that monitor, including corners. The default is 14 physical pixels. Set it to zero to disable snapping. **Reattach**, the original tab's **Reattach panel**, or closing the pop-out returns the editor to its tab. **Fullscreen** toggles the main workspace.

## Assemble a review

Run these PowerShell commands from the repository root with your own input files. Once assigned, the executable path remains valid when you change directories.

```powershell
$clip = (Resolve-Path ./var/build/debug/aiclip.exe).Path
& $clip add review "draft*.md" "glossary.md"
& $clip text review "Compare the drafts and flag missing requirements."
& $clip show review --stats
& $clip put review
# Copy the model's reply before running the next command.
& $clip get response
& $clip save response response.md
```

Add and show leave the Windows clipboard alone. Each wildcard expands again at preview or publication. Quote patterns, including `"docs\*.md"`, to preserve argument groups. Each group sorts by path. Repeating an input includes it again. Use `--no-headings` and `--separator` to control formatting. File bodies must be UTF-8. A leading byte-order mark is removed. A failed required read cancels publication.

## Reuse and preserve work

- `template save critique review` saves the ordered entries from review. `template run critique` renders that global recipe using current file contents.
- `snapshot save review` freezes the rendered text and source paths and prints an ID. Use `snapshot list`, `snapshot show ID`, `snapshot sources ID`, or `snapshot restore ID new-buffer`. Publish the restored buffer when you need exactly the frozen text.
- `--session NAME` selects an independent group of buffers and snapshots for that invocation. `list` shows its buffers. `clear NAME` clears the named buffer.
- `get --file reply.md` captures directly to a new file. `append "notes*.md"` appends files directly to the current clipboard. `--append`, `--add` and `--clear` are alternate spellings of their subcommands.
- Use `--db PATH` in either mode or set `AICLIP_DB` to use a separate database. The Windows default is `%LOCALAPPDATA%/aiclip/aiclip.sqlite`. Captures and snapshots contain plaintext. Capturing a reply is a manual action.

The standalone frameless-window probe runs with `.\local\run_gui_test.cmd`. The remaining docking, platform and optional AI milestones are in [the development plan](doc/plan.md).
