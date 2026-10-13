# Keyframe

A macro recorder and player for Windows 10/11, built with Tauri v2 (Rust backend, React + TypeScript + Tailwind frontend).

## Build

You need Rust (MSVC toolchain), Node 18+ and the Visual Studio C++ build tools.

```
npm install
npm run tauri dev
npm run tauri build
```

## Using it

- Record with the Record button or the record hotkey (`Ctrl+Shift+R` by default, changeable in Settings). There is a 3 second countdown first.
- Right-click the action list to add actions, paste, undo or redo. Right-click an action for insert, copy, duplicate, wrap in loop and delete.
- Each macro has a trigger (key combo or mouse button), a play mode (once, N times, toggle, while held) and an optional app it is limited to.
- To hold a key or mouse button while other actions run, select those actions, right-click and pick "Hold a key during these" (or mouse button). Select actions and use "Wrap in loop" to repeat them.
- Macros can be put in folders and searched. Import and export live in the `...` menu.
- Themes are defined in `src/lib/themes.ts`. Add an entry and it shows up in Settings.
- The close button quits the app. Turn on "Close button hides to tray" in Settings to keep it running in the background instead.

## Playback input

Settings > Playback > Input has two options:

- **Standard** sends input with SendInput.
- **Driver (Interception)** sends keys and mouse through the Interception driver, so the input looks like real hardware.

The driver is bundled. Choose it, press Install driver, accept the UAC prompt and restart Windows.
Some anti-cheats refuse to run while it is installed.

If a game runs as administrator, run Keyframe as administrator too.

## License

MIT, see [LICENSE](LICENSE).

The files in `src-tauri/resources/interception` (`interception.dll` and its installer, from
https://github.com/oblitum/Interception) are not covered by it. They are LGPL 3.0 and free for
non-commercial use only; commercial use needs a license from their author.

## Layout

```
src-tauri/src
  input.rs       low-level keyboard and mouse hooks
  hook.rs        routes input to recording, triggers and key capture
  recorder.rs    recording state machine
  player.rs      playback threads
  inject.rs      Interception backend
  driver.rs      driver status and installer
  hotkeys.rs     global shortcut registration
  procs.rs       foreground app helpers
  commands.rs    commands used by the UI
src              React UI
```
