---
name: wild-goose-testing
description: Run a "wild goose" pass over a change to consisTerm - simulate a chaotic user, a hostile settings.toml, an old theme file, a second monitor, a dying session, against a new feature grafted into the terminal, and find the states it leaves behind that no gesture in the app can undo. Use when the user asks for a wild goose test or pass, to "guard against bugs" or stress a change, or before handing a branch over. This repository's version of the skill; the ObjectScript one does not apply here.
---

# Wild goose testing - consisTerm

The app is a room furnished over many sessions: a frame loop that must cost
nothing when idle, a terminal core that knows nothing of the window, Win32 code
running outside egui's frames, settings and themes written to disk by every
version that ever ran. The new feature is a chessboard set down in it. Let a
goose loose: what breaks, and how must the board be set so the game still plays?
**The user is the goose** - and so are the files an older build left behind,
the second monitor, and the IRIS session that drops in the middle of a gesture.

Not unit testing, not code review: an adversarial reading of a **new feature
inside old code**, asking what can happen that nobody planned for, and what is
left behind.

The symptom to hunt: **state the user cannot get out of through the
interface.** A window stuck where nothing can reach it, a setting written to
disk that the next start cannot read back, a dialog that cannot be answered, a
theme that cannot be edited, a password in `history.txt`, a frame loop that
never goes idle. An error in the status bar is a good outcome; a silent success
over a half-done write is the bug.

Guard the new thing against the room. Never redesign the room - a pass that
becomes a refactor of `app/` or `term/` has lost the plot.

## 1. Running a pass

**Never spawn subagents** unless the user explicitly asks for them. Every pass
is read and reasoned in the main session; a delegate returns a conclusion
without the code path, and section 2 needs the path. Passes run one after the
other.

1. **Verify the tree holds the change.** Grep two or three markers of the
   feature (a new type, field, `tr` string) before reading anything. Uncommitted
   work vanishes to a stash or a branch switch; a pass over replaced code is
   worse than none. Markers missing - say so, stop.
2. **Name the chessboard.** What exactly was added: a setting, a page, a painter,
   a Win32 hook, a dialog behaviour?
3. **List the seams.** Every point where the new thing reads, writes or branches
   on something older: `Settings` and its serde defaults, theme files, `PanelState`,
   egui memory and ids, the `UiRequest` path, the frame loop's repaint pacing,
   Win32 statics, the reader threads, `tr`.
4. **Walk the goose through each seam** with the catalogue in section 3. Read the
   code path end to end; never reason from a function's name or its doc comment.
   Code > notes > the commit message.
5. **Confirm before reporting** - section 2.
6. **Fix defensively**, then report per section 4.

## 2. Confirming a finding

A finding needs the concrete sequence - what the user does, or what file is on
disk - and the resulting state. "Could be a problem" is not a finding.

**Name the single premise it rests on and verify that directly.** Most premises
here are facts about **egui 0.28** or **Win32**, not about our code: whether a
widget registered later wins the hit test, whether an `Area` remembers its size,
whether `WM_EXITSIZEMOVE` arrives after a programmatic move, whether serde fills
a missing field. Read the dependency's source in `~/.cargo/registry/src/*/` - it
is on disk - rather than remembering how it works. One pass here built a drag
handle on the premise "a widget claimed first loses the hit test to everything
drawn over it"; egui 0.28 starts a drag immediately on a drag-only widget, and
the premise only fell when the test was run.

**Carry each fact to every half of the finding.** When a fact kills one half,
apply it to the others before writing them up.

**Prove it headless when you can.** The egui layer is drivable without a window:
`egui::Context::default()` and `ctx.run(RawInput { screen_rect, events, time, .. })`
for a few frames, then read `ctx.memory(|m| m.area_rect(id))`, the returned
`FullOutput` (its `viewport_output[..].commands` carry `StartDrag`, `Close`...),
or a `Response`. Press, move and release are `Event::PointerButton` and
`Event::PointerMoved`; hit testing uses the **previous** frame's widgets, so lay
out two frames first. A finding that a headless test reproduces is confirmed,
and the test stays as the guard.

**When the premise is how it looks, render it.** Tessellate a frame
(`ctx.tessellate(out.shapes, 1.0)`) and rasterise the colored triangles into a
PNG in the scratchpad - a sixty-line throwaway test, ignoring textures, so text
is blank but every shape is true. Read the PNG. Delete the throwaway afterwards.
This is how the XP and 98 buttons and the title-bar preview were checked.

**When only the real window can show it, ask.** Win32 placement, window regions,
the drop-down's roll, DWM, the tray, focus: none of it runs headless. Report the
finding `UNVERIFIED (live)`, name the fact that would settle it, and ask before
launching anything - the user works on this desktop, and a launched window or
`SetForegroundWindow` steals their focus. When they agree: a release build,
`CONSISTERM_CONFIG_DIR` pointed at a scratch folder with `open_on_start = false`,
and a local shell tab, never their IRIS session. Pointing `%APPDATA%` elsewhere
does nothing on Windows.

**Live IRIS tests never log in and never write.** `tests/integration/live_*` are
`#[ignore]`d and read-only; every `RDB*` database is shared with the team.

Run **more than one pass**: a later pass finds the worse defect, because the
earlier fixes changed what is reachable. Pick a different theme each time:

- **the round trip through disk** - follow the new value: default -> settings
  page -> `Settings` -> `settings.toml` -> next start -> page. Then the same with
  a file written by the **previous release**, and by a **future** one.
- **the old file** - a `settings.toml` or theme `.toml` from before the change:
  a removed field, a renamed variant, a value outside today's range, a built-in
  theme's name now taken by a user copy.
- **the idle frame** - does the change ask for frames while nothing moves? While
  minimized? While hidden in the tray, where eframe runs no frames at all?
- **the second pane and the second window** - splits, the detached Settings
  window, the drop-down, two monitors with different scales, a monitor unplugged.
- **the framework seam** - how the change is registered with egui or Win32 and
  what they do with it: ids, layer order, `move_to_top`, `Area` memory, the hit
  test, subclass procedures, timers, hotkeys. Read their source.
- **the identity seam** - egui ids: auto ids that shift when the layout above
  changes, `Id::new` collisions between two widgets or two dialogs, state keyed
  per widget that should be per tab.
- **blast radius of your own fixes** - follow each changed function out to its
  last caller, including the ones behind `cfg(windows)`, `cfg(test)` and the
  `plugins` feature.
- **the guard seam** - early returns and caches the new code runs under:
  `if self.quake == Some(wanted) { return }`, a `OnceLock` built once per
  process, a value cached in `PanelState` at open.
- **the undo seam** - every new state needs a way back through the UI: a switch
  off, a reset button, Esc, a page reachable to change it. A new setting with no
  row, a mode with no exit, a value the page clamps but the file does not.
- **the second run** - the gesture twice in one session, then after changing the
  setting, then after a restart. Session state - egui memory, Win32 statics,
  `PanelState` - is invisible on the first run.
- **the two languages** - every string through `tr`; a `tr` key that is built at
  run time never translates; a Portuguese string longer than the English one in
  a fixed-width control; search keywords in both.
- **the reachability seam** - the code path exists and runs: a function nothing
  calls, a page nothing links to, a `match` arm for a variant no constructor
  produces, a `cfg(windows)` body on a platform the user is on.

## 3. The catalogue

Every entry is a real finding or a near miss on this codebase. Walk them.

**The invariants in CLAUDE.md** - check each against the change, every pass:

- **A password never reaches disk or history.** Anything new that persists or
  *displays* what the user typed answers to `Tab::pump` muting the log across the
  autologon password step and `App::record_command` refusing outside a prompt.
  Displaying counts: a preview that echoes a parameter named "Senha" puts it on
  screen, and a macro with `hide_command` must show nothing anywhere - its menu
  tooltip, its dialog, its editor preview.
- **Nothing is polled.** A new `request_repaint` without an `_after` and a reason
  to stop costs a core forever. Measure with the `TotalProcessorTime` sample in
  CLAUDE.md, idle and with Settings open: under 1%.
- **Both panes of a split are drained every frame**, or IRIS blocks.
- **Glyphs sit on the lattice** - any change to drawing runs
  `a_row_drawn_as_one_shape...` and the paint benchmark against a stashed
  baseline in the same run.
- **Menu items and shortcuts raise a `UiRequest`.** A new gesture that acts
  directly is one path too many; the next fix will land on only one of them.
- **Every user-facing string goes through `tr`/`tr1`/`tr2`**, and **every tooltip
  through `ui::tip::Tip`**.

**Settings on disk**

- `#[serde(default)]` on `Settings` fills a **missing** field from
  `Settings::default()` - so changing a default changes every install whose file
  never wrote the field, and none whose file did. Ask which you meant. Changing
  the default language reached only new installs, which was the intent; a default
  that *should* reach everyone needs a migration, not a new default.
- A **removed** field is ignored on read (no `deny_unknown_fields`) and dropped
  on the next write. Fine - unless something else still reads it from the raw
  file (the usage report's "differs from a fresh install").
- An **enum** read from a file with an unknown variant fails the whole file
  unless the enum is tolerant. Look at how `QuakeEdge`, `BarPosition` and the new
  ones deserialize, and what a file written by a newer build does to an older one.
- A range the page enforces (`DragValue::range`) is not a range the file obeys.
  Every reader clamps, or a hand-edited `quake_height = 500` reaches Win32.
- Writing `settings.toml` happens on `UiRequest::SettingsChanged`; a control that
  edits `Settings` without setting `c.changed` is lost at exit.

**Themes on disk**

- A built-in is the binary's; a file with a built-in's name is dropped by
  `load_themes` if it says `builtin = true`, kept as a user theme otherwise.
  Adding a built-in whose name a user copy already has (`Windows 98`) gives two
  themes of one name, and `settings.theme` picks by name.
- A theme field read with `from_name` falls back silently (`"classic"` read by an
  older build is `Stroke`). Fine for files; check every `match` on the enum has
  the new arm, including the Themes page's segmented labels and any `matches!`
  that lists the filled styles.
- Every new colour slot needs: the file field, `from_file`, `to_file`, the editor
  row, the round-trip test.

**egui state that outlives the frame**

- **An `Area` lays out inside the size it had last time and never grows back.**
  Tooltips, popups and dialogs inherit last frame's width; content that was once
  short stays squeezed. Give a floor computed from the content (`ui::tip`).
- **Auto ids shift.** `allocate_exact_size` and friends take the next auto id;
  insert a widget above and every id below moves, and so does the state keyed on
  it - a tooltip area, a scroll offset, a "pressed" memory.
- **Hit testing uses the previous frame's widgets, in layer order, and the last
  registered wins within a layer** - except that a **drag-only** widget under the
  press starts dragging immediately, and is then the only thing hovered. A
  background that senses drag behind a button still receives the drag when the
  press starts on the button and moves. Decide from where the press *landed*.
- `ctx.data` temp values survive until replaced. A flag set on press must be set
  on every press, not only the ones you thought of, or it carries over.
- `ViewportCommand::StartDrag` hands the pointer to the OS move loop; the release
  never reaches egui. Anything that starts a drag must not do it on a press that
  could still have been a click.

**Win32 outside the frame**

- Code in `on_message`, a subclass procedure or a timer runs while eframe is
  between frames, on the main thread. It cannot read egui; what it needs lives in
  statics. Every static set on one path must be reset on every path that ends
  that state: a `DOWN` flag cleared on configure, on hide, on detach.
- A window that is hidden runs **no frames**. Anything that waits for a frame to
  finish what Win32 began - a region to clear, a size to restore - never runs.
  Finish in Win32.
- `SetWindowRgn` hands the region to the system; a region left set clips the
  window for good. Every path out of an animation clears it, including the press
  that interrupts the animation, and the window brought back by the tray.
- `WM_EXITSIZEMOVE` ends **moves and resizes alike**, from the user or from
  `ViewportCommand::BeginResize`. Tell them apart by what changed.
- Monitors: work area, not screen; negative coordinates for a monitor left of the
  primary; the monitor under the pointer, not the one the window was on.

**State carried in from last time**

- A `PanelState` field set when a page opens and read after it closed
  (`capture_quake_shortcut` left listening keeps the app from answering its own
  shortcuts - cleared on leaving the page for that reason).
- A memo keyed narrower than what it recognises: `App::quake` compares the whole
  `Quake` struct, so a new field must be in it or changing that setting never
  reconfigures Win32.

**Errors that don't propagate**

- `let _ =` on a Win32 call, an `.ok()` on a write, a `Result` turned into a
  status line nobody sees because the window is hidden. Ask where the user
  learns it failed.
- A refusal that leaves the half-done state: a theme saved, its file write
  failed, the in-memory copy says saved.

**Boundaries and blanks**

- Empty, whitespace-only, and absent are three states: an empty macro body, a
  parameter with no name, a shortcut cleared to `""` (written as blank because
  TOML has no null).
- Long text in a fixed place: a Portuguese label in a segmented control, a
  macro line of 400 characters in a tooltip, a theme name in a tab.
- Scale: every pixel constant multiplied by `ui_scale` and `title_bar_scale` and
  the monitor's DPI; a 1 px stroke at 150% lands between pixels.
- Text inside a structure: a `{{name}}` inside a value, a `}` in a parameter name,
  a `'` or `"` the command quotes around.

**Paths that do not exist**

- A `match` arm, page or link that nothing constructs or reaches. The settings
  tests check every link leads to a page and every sub-page is linked; a new page
  outside `build_pages` passes nothing.
- A `cfg(windows)` body never compiled on the CI runner for Linux or macOS, or
  the reverse: run clippy with the target in mind and read the `not(windows)`
  stub.

**The record that isn't there yet**

- The first frame: `area_rect` is `None`, `prev_frame` widgets are empty, a
  cached font list not loaded. A feature that needs last frame's geometry must do
  something sensible without it.

## 4. Reporting and fixing

Report in English. Lead with the worst finding stated operationally - *"a dialog
whose buttons are off screen leaves the window impossible to move"* - not with
the function name. For each finding: the sequence, the state left behind, the
fix, and whether anything already on disk (settings, themes, history) may be
damaged.

**Report the verified negatives too**, a line each: "I checked X; it is safe
because Y, `file:line`". On most passes this is most of the work.

**A fix needs its own pass.** A guard that refuses a state is a refusal waiting
to fire on a legitimate flow: run the headless test for the legitimate flow too
(the button that must still click, the resize that must not detach).

**Withdraw plainly** when a finding does not survive, revert whatever guard you
wrote for it, and record it with the premise that failed.

Before claiming anything works, run what CI runs:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features plugins -- -D warnings
cargo test --lib --tests
cargo test --features plugins --lib --tests
cargo doc --no-deps   # no warnings
```

with the `PATH` export from CLAUDE.md in the same command. End the report with
what was run, and what was not (live Win32, IRIS).
