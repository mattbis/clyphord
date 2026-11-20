# aiclip development plan

Build the wildcard-to-clipboard CLI first. A typical job is assembling a draft, glossary and review instructions for another model, then capturing its response. The same operations should work later from a session panel, a detached window or a hotkey. This is a roadmap. Command spellings below distinguish the recovered design from proposed details that still need a CLI test.

## Current Windows implementation

One aiclip executable provides the CLI and the desktop workspace (`--gui`). It and the standalone GUI probe build from `local/clean_build.cmd`. CLI commands and examples are in [the README](../README.md). The desktop shares the CLI database and supports source staging, literal instructions, clipboard capture, publication, a detachable frameless preview and side-by-side comparison across named sessions. Templates and versioned snapshots are available through the CLI. Capture and snapshot history are explicit user actions.

The Windows build and cross-process CLI tests cover wildcard ordering, persistence, session isolation, immutable snapshots, live templates and publication failure handling. Interactive window behavior and the system clipboard round trip require a desktop check. Advanced window management and suggestion features below remain planned.

## 1. CLI and text assembly

| Operation | Direction and intended effect |
| --- | --- |
| `put review` | Render the review buffer and replace the system clipboard. Repeating it with identical inputs produces identical text. Proposed aliases include `cp`, `copy`, `pub` and `publish`. |
| `get` | Capture the current clipboard into the selected buffer. The earlier file-oriented example was `get gemini-response.md`. Make a file destination unambiguous before freezing the parser. |
| `add review "draft*.md"` | Expand the pattern and append the sources to the named buffer in deterministic order. Support adding to the current buffer as shorthand. |
| `show`, `save`, `--stats` | Preview exactly what put would write, save rendered text to a file, or report file count, size and estimated tokens. Label the estimate. |
| `--append`, `--clear`, `--add` | Earlier convenience forms. Define which target each affects and map each to a single underlying operation. Appending to the current clipboard is an explicit operation. |

Preserve argument group order. Sort wildcard matches within each group, including quoted Windows patterns such as `"docs\*.md"`. Keep filename headings and separators configurable. Provide plain text on stdout for pipelines and diagnostics on stderr. Specify stdin, empty input, duplicate matches, directories, recursion and file overwrite behavior in the CLI help. Proposed first encoding is UTF-8 with optional BOM. Report an unreadable file, unsupported encoding or unmatched required pattern before changing the clipboard.

The first milestone needs persistent named buffers because each line below starts a new process. Stage instructions and files in the buffer. Only the final put publishes to Windows, using one clipboard write for the whole document so clipboard history stays useful. Report a size warning for unusually large output; the threshold is open.

```powershell
aiclip add review "draft-6.md"
aiclip add review "glossary*.md"
aiclip add review "review-instructions.md"
aiclip show review
aiclip put review
```

The original direct-append example was `aiclip --append "draft-6.md" "glossary*.md"`. Test this separately with existing clipboard text. For named buffers, verify that add and show leave that text intact until put.

**First acceptance run.** Create draft-5.md, draft-6.md and glossary.md with distinct text, including a non-ASCII character. Add the quoted draft wildcard followed by the glossary. Preview and put twice. Check all three names and bodies appear in the same order exactly once in each result and that the preview matches the pasted text. Add a missing required source and verify the existing clipboard survives the error.

## 2. Buffers, templates and sessions

- Store named buffers with ordered entries. Distinguish captured clipboard text, literal instructions, file references and wildcard references so the UI can show what a later render will read.
- Save reusable recipes with `template save` and render them with `template run`. A live recipe reads current files and expands its patterns at render time. A snapshot retains the rendered text and source list. Editing a file after capture must change the next live render while preserving the snapshot.
- A session groups buffers, templates, captures and history for one task. Switch or reopen a session and recover its working state. Compare two sessions or snapshots side by side so a missing draft or changed instruction is visible.
- Use SQLite via rusqlite for persistent state and history. Commit a capture together with its session association. Define history retention, exclusion and deletion controls before enabling background collection.

Buffers belong to sessions. Keep globally reusable templates available across sessions. Preserve the exact published prompt and captured response as versioned snapshots so a user can restore an earlier draft or compare ChatGPT and Gemini responses against the prompt each received. The session view should expose its buffers, templates and the current clipboard together.

## 3. Architecture and portability

Use Rust throughout as the working choice. Put wildcard expansion, ordering, formatting, rendering and session operations in a shared core. Give clap responsibility for CLI parsing, arboard for the clipboard boundary and rusqlite for storage. Exercise the core with temporary files and a memory clipboard, then run the same workflow against the Windows clipboard. Keep Windows work-area, tray and hotkey code behind desktop-specific interfaces. The CLI should run without a desktop window. Report clipboard access errors to the caller.

The earlier Windows-first alternative was C#/WPF. Rust with egui/eframe and egui_dock is the cross-platform candidate. Electron/React supplies the interaction reference. Independent windows can share one application model in one process. If a later design splits processes, carry the same commands and state-change notifications over IPC. Keep one writer for each state change so opening two views cannot double-append a file.

Organize the Rust workspace around the shared core, CLI and later desktop frontend. Give window management responsibility for monitor selection, snapping, visibility and saved geometry. Keep native calls in platform adapters. Evaluate winit through eframe for window events and Win32 `SHAppBarMessage` for optional reserved desktop space. The Electron sampler used IPC and publish/subscribe to update React views. In the Rust probe, a change in either window should update the other from the same buffer.

## 4. Desktop workspace and smoke test

Start with the standalone crate under `test/gui_framework/`. Run it with `cargo run --manifest-path test/gui_framework/Cargo.toml`. It exercises a real second viewport, a shared editable buffer, pin/unpin and reattachment. Close the pop-out using its title bar and verify the text reappears in the main window. This first probe uses eframe directly. Extend it in the following order before committing the production UI to the stack.

| Probe | Pass condition on Windows |
| --- | --- |
| Panel docking | Use egui_dock to move a tab between panels in the fullscreen session workspace. Detach it into a native window and reattach it with its identity and edits intact. |
| Anchoring | Attach a pop-out to the parent, move the parent, then detach and move each independently. Keep parent attachment distinct from snapping to a display. |
| Magnetic snap | Compare window bounds to the current monitor’s usable work area. Within the configured threshold, align to the top or bottom edge as in the Electron example. Outside the threshold, leave the position alone. Skip maximized windows. Repeat on a secondary display with different scaling and a taskbar. |
| Pinning and focus | Toggle always-on-top per window, switch to another app and verify both states. Closing or hiding one panel should leave the others usable. |
| Global hotkey and tray | With another app focused, show/hide aiclip by hotkey and restore it from the tray. Check conflicts and clean exit. |
| Top-screen dock | Give the dock configurable action icons and auto-hide. Test overlay and reserved-space modes independently, including releasing reserved space on exit. |

Provide three ways into a session: a quick overlay, a docked action strip and a fullscreen workspace with resizable comparison panels. A detached session on a second monitor has its own visibility. Test hiding and restoring the main workspace while that session stays visible.

Use the reference application's 14 px magnetic threshold as the starting value. First reproduce its top/bottom behavior, then probe left, right and corner docking. Try a configurable 52–100 px strip. Save the session layout, monitor, window bounds and pin state. Reopen it on the same displays, then disconnect one and verify every window can be reached on a remaining display.

Record OS, display scale, monitor layout and pass/fail per probe. Select the framework after the window, focus and docking behaviors work on the target desktop.

## 5. Learning and optional AI

Start with an opt-in local SQLite event history for actions such as selecting a template or adding a source. Rank frequent and recent actions first. Compare second-order Markov suggestions (what tends to follow the last two actions) and association rules (which items tend to be used together) against that baseline. On sparse history, fall back to frequency. Suggestions require a user action to apply. Keep event metadata and captured text independently controllable.

Later, offer local-model and API-assisted suggestions. Preview the exact material included in an API request and require explicit approval to send it. The ordinary clipboard, template and session workflows must work offline.

Additional learning candidates discussed were Clink-style frequency/recency scoring, Apriori or FP-Growth for recurring item sets, local embeddings for semantically similar material, and contextual bandits for ranking actions from feedback. Treat these as experiments. Evaluate whether each improves on the simple ranking before adding it.

## 6. Privacy and credentials

Start with deliberate aiclip actions. Make passive clipboard monitoring opt-in and provide a privacy mode and sensitive-app exclusions, including password managers. Test that excluded content never enters history or learning data. Store API credentials in the OS credential vault. Show suggested edits for approval before changing buffer text.

## Delivery order

1. Ship one end-to-end wildcard CLI workflow and settle the positional grammar with real shell examples.
2. Add durable buffers, live templates, snapshots and session comparison. Replay the draft-review workflow across two launches.
3. Run the GUI smoke test alongside CLI work. Validate remaining Windows probes, then build the session workspace and top dock around the shared core.
4. Evaluate local suggestions on actual usage. Add optional AI only after users can inspect and control the history and payload.
