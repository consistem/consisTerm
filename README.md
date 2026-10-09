<p align="center">
  <img src="assets/logo.png" alt="consisTerm logo: a glass ring holding a terminal prompt, teal fading to violet" width="160">
</p>

<h1 align="center">consisTerm</h1>

<p align="center">
  <b>The terminal for InterSystems IRIS, built for the people who live in it.</b><br>
  Read globals, look up data, ask Claude about the output, share commands across the team — without leaving the prompt.<br>
  Windows · Linux · macOS
</p>

<p align="center">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue">
  <img alt="Written in Rust" src="https://img.shields.io/badge/written%20in-Rust-orange">
  <img alt="Runs on Windows, Linux and macOS" src="https://img.shields.io/badge/runs%20on-Windows%20%7C%20Linux%20%7C%20macOS-555">
  <img alt="No account, no telemetry" src="https://img.shields.io/badge/no%20account-no%20telemetry-2ea44f">
</p>

<p align="center">
  <b>English</b> · <a href="README.pt-BR.md">Português</a>
</p>

<p align="center">
  <a href="#global-tooltip">Global tooltip</a> ·
  <a href="#autocomplete-and-data-lookup">Autocomplete</a> ·
  <a href="#send-it-to-claude">Claude</a> ·
  <a href="#macros-for-you-and-for-the-whole-team">Macros</a> ·
  <a href="#every-shell-in-one-window">Shells</a> ·
  <a href="#and-everything-else">Everything else</a> ·
  <a href="#downloads">Downloads</a>
</p>

---

## Global tooltip

**Point at a global and it tells you what it is.** Hover any piece of a
`zwrite` line, a `write` of one node or a `^%G` listing, and consisTerm shows
which property that subscript or `^`-piece holds, its type, its size and its
list of allowed values — read from the classes that map the global, in the
namespace the session is in.

The question goes down a session of its own, never the one you are typing in,
and it asks for structure only, never for a record's value. The first hover on
a global is the only one that ever waits.

<p align="center">
  <img src="docs/images/global-tooltip.png" alt="The pointer over the third piece of a zwrite line; a tooltip names the property, its type and its allowed values" width="100%">
  <br><sub>Hover a piece of a global and read what it means.</sub>
</p>

## Autocomplete and data lookup

**Suggestions as you type, from the namespace itself.** Commands, `$`
functions, `^` globals, routines, `##class(` names and, in SQL mode, keywords
and tables. Globals come straight from the namespace — mapped ones and
`^mtemp…` in IRISTEMP included — and a prefix with too many is folded into one
line per next letter (`^TG…`) that narrows as you type.

Inside `^GLOBAL(` it goes further: it names the subscript you are on from the
global's documentation and offers its constants, its listed values and the
subscripts that actually exist there right now. Up/Down choose, Tab accepts,
Esc closes.

<p align="center">
  <img src="docs/images/autocomplete.png" alt="Typing inside ^GLOBAL( shows a list of existing subscripts with the subscript's name above them" width="100%">
  <br><sub>Real subscripts from the database, named by the global's documentation.</sub>
</p>

## Send it to Claude

**Ask about what is on the screen.** *Analyze with Claude* opens a Claude Code
session in a tab of its own with the terminal's output already in its context,
and then waits for your question — nothing is asked on your behalf. Choose how
much to hand over: everything, the last 10 commands, the last 5, or just the
selection.

<p align="center">
  <img src="docs/images/claude.png" alt="A Claude Code tab beside an IRIS session, answering a question about an error in the output" width="100%">
  <br><sub>An error on screen, and a tab that already knows about it.</sub>
</p>

Needs [`claude`](https://claude.com/claude-code) on the `PATH`.

## Macros for you and for the whole team

**A collection of commands, one click or one shortcut away.** Macros live in
XML: `{{param}}` placeholders are asked for before sending, `confirm="true"`
guards anything that writes, each macro can have its own shortcut, and
`hide_command="true"` keeps a command carrying a password out of the
interface. The editor in Settings checks every parameter against the command.

**The organisation file** is a second, read-only collection that every copy of
consisTerm loads beside your own. Put `org-macros.xml` next to the executable —
or point Settings at a share — and the whole team has the same commands, kept
in one place. A template ships in
[`packaging/org-macros.xml`](packaging/org-macros.xml).

<p align="center">
  <img src="docs/images/macros.png" alt="The right-click menu of the terminal showing Organisation and Personal macro groups" width="100%">
  <br><sub>The team's macros and your own, side by side.</sub>
</p>

## Every shell in one window

**Not only IRIS.** Command Prompt, Windows PowerShell, PowerShell 7, Git Bash,
WSL, or whatever `/etc/shells` lists — each in a tab, beside your IRIS
sessions, split with them, themed with them. Every shell is a `.toml` in
`plugins/shells/`, written the first time the app finds it installed and yours
to edit after that. *Open consisTerm here* in Explorer's folder menu opens one
in that folder.

<p align="center">
  <img src="docs/images/shells.png" alt="Tabs for an IRIS session, PowerShell and Git Bash in one window" width="100%">
  <br><sub>IRIS, PowerShell and Git Bash in the same window.</sub>
</p>

---

## And everything else

* **Tabs and split panes** — one session per tab, each with its own scrollback
  and log; split right or down for a second session in the same tab. Named
  after the instance and, if you like, the namespace, following every `ZN`.
* **Title bar anywhere** — top, bottom, left or right, as browsers offer. On a
  side, the tabs become a column.
* **Drop-down terminal** *(Windows)* — a system-wide shortcut (F12 by default)
  brings the window down from the top or up from the bottom of the screen, the
  way Guake and Yakuake do, and puts it away again.
* **Lines far longer than the window** — the terminal reports a 16384-column
  margin, so a `zwrite` of a wide global arrives whole.
* **ObjectScript colouring** — globals, strings, macros, class and method
  references, routines and commands, including their abbreviations.
  Prompt-aware, so plain prose never lights up.
* **Editing at the prompt** — Home/End, word jumps, click to place the cursor,
  and typing `"`, `(` or `[` over a selection wraps it instead of replacing it.
* **Command history that outlives the session**, and **Ctrl+F** across the
  whole transcript.
* **SQL mode** — `/sql` or Ctrl+Shift+Q drops into the IRIS SQL shell, with SQL
  colouring while you are there.
* **IRIS utilities** — compile a package, compile a routine group, generate an
  interface, with the exact line shown before it is sent.
* **Zoom** — Ctrl + wheel, Ctrl+Plus/Minus, a trackpad pinch; Ctrl+0 resets.
* **Themes you can edit** — fourteen built in, including **High Contrast** dark
  and light and **Colour-blind Safe** dark and light, built on the Okabe–Ito
  palette. Duplicate any of them and every colour is yours, including the
  title bar's layout.
* **Settings with search**, in English or Portuguese, with an interface scale
  from 100% to 150% and a separate title bar scale.
* **Export and logging** — screen or scrollback as text or colour-preserving
  HTML; per-session transcripts with password redaction and rotation.
* **Screen savers** — Matrix, a bouncing logo, or your own text or picture.
* **Autologon** — credentials from the OS credential store, never from a file.
* **Close to the tray**, **always on top**, and **automatic updates** through
  the machine's own proxy — nothing is downloaded without being asked.

---

## Downloads

➡️ **[Latest release](https://github.com/consistem/consisTerm/releases/latest)**

| Platform | File | Notes |
| --- | --- | --- |
| Windows x64 | `consisterm-<version>-windows-x64.exe` | Portable — no installer. Also as a `.zip`. |
| Linux x86_64 | `consisterm-<version>-linux-x86_64.AppImage` | `chmod +x` and run. Needs glibc 2.35+ (Ubuntu 22.04, Debian 12, Fedora 36 or later). Also as a `.tar.gz`. |
| macOS (Apple silicon and Intel) | `consisterm-<version>-macos-universal.dmg` | Open it and drag consisTerm to Applications. |

Every release lists its `SHA256SUMS.txt`. An IRIS or Caché instance to connect
to is the only other requirement — local instances are found on their own.

---

## Where it stands

Honestly, as of 0.1:

* **Solid:** IRIS and shell sessions on Windows, and everything above that is
  not marked otherwise.
* **New:** the Linux and macOS builds. They build cleanly, but have seen far
  less use than Windows — reports welcome.
* **Windows only, for now:** the drop-down terminal, the tray, the Explorer
  menu and standing in for `Iristerm.exe` from the IRIS tray.
* **Unsigned:** Windows SmartScreen and macOS Gatekeeper will ask before the
  first run. On macOS, right-click the app and choose *Open*.

---

## Configuration

Everything lives in the platform's config folder — `%APPDATA%\consisTerm`,
`~/.config/consisTerm` or `~/Library/Application Support/consisTerm`. Set
`CONSISTERM_CONFIG_DIR` to keep it somewhere else, such as beside a portable
copy.

| File | Purpose |
| --- | --- |
| `settings.toml` | Language, theme, font, window, sessions, logging, profiles |
| `macros.xml` | Your personal macros |
| `history.txt` | Commands typed at an IRIS prompt, for recall |
| `themes/*.toml` | Your own themes |
| `plugins/shells/*.toml` | One per shell, found or declared |

Passwords never touch a file: they go to Windows Credential Manager, the macOS
Keychain or the Secret Service.

Coming from **newIrisTerminal**? The first start copies your settings, macros,
themes and history across; the old folder is left as it was.

## Build from source

```sh
cargo build --release
cargo test
```

The WASM plugin host is behind a feature flag:

```sh
cargo build --release --features plugins
```

On Windows without Visual Studio, the GNU toolchain needs a full MinGW-w64
beside it:

```powershell
winget install Rustlang.Rust.GNU
winget install BrechtSanders.WinLibs.POSIX.MSVCRT
```

Release packaging lives in [`packaging/`](packaging/) and is described in
[`docs/releasing.md`](docs/releasing.md).

## ⚠️ Safety note

Macros and the IRIS utilities type into a live session, and `RDB*` databases
are shared with the whole team. Give any macro that changes data
`confirm="true"`: consisTerm then shows the exact text it will send and waits
for an explicit yes.

## Versioning and license

[ZeroVer](https://0ver.org) — the major version stays at zero. MIT licensed.

<p align="center"><sub>Made at <a href="https://www.consistem.com.br">Consistem</a>.</sub></p>
