# The Doctor: writing checks

The Doctor is the first page of Settings. It runs a list of checks on the environment around
the terminal (the IRIS namespaces, the Docker containers and their image versions, whatever an
organisation's developers depend on) and shows each one with a status light. A check that
knows how to put things right offers a **Fix**, which runs in a terminal tab, in view.

consisTerm ships no checks of its own. They come from a **checks file**: an XML file the
organisation writes, distributed the same way as the organisation's macros. The app looks for it
in one of two places:

- the path set in **Settings › Doctor › Checks file**, or
- `org-doctor.xml` beside the program, when that path is empty.

Scripts are named relative to the checks file, so the file and its scripts travel together as one
folder. A working example lives in [`examples/doctor/`](../examples/doctor/).

## The file

```xml
<doctor python="py -3">                      <!-- python: optional, how to start Python -->
  <var name="comp" value="COMP"/>            <!-- {{comp}} in commands, titles; DOCTOR_COMP in a script's environment -->

  <group name="Docker">
    <check id="docker.containers" title="Containers"
           objective="Every service of docker-compose.yml is running"
           timeout="60">                     <!-- seconds; 30 when left out -->
      <requires check="docker.network"/>     <!-- run only once that one is healthy -->
      <run kind="python">docker/containers.py</run>
      <expect status="ok"/>                  <!-- what counts as healthy; "ok" when left out -->
      <fix title="Start the containers" kind="shell" confirm="true">fixes/consistem.sh start</fix>
      <after check="docker.containers"/>     <!-- what must hold once the fix has run -->
      <after check="docker.network" status="ok"/>
    </check>
  </group>
</doctor>
```

| element | what it does |
| --- | --- |
| `<var name value>` | A variable. Written `{{name}}` in a command, a title or an objective; given to every check's script as `DOCTOR_<NAME>`. `instance` and `namespace` are also where ObjectScript runs by default. |
| `<group name>` | A card of checks on the page, in the order the file gives. |
| `<check id title objective timeout mandatory>` | One check. `id` is unique and is what `<requires>` and `<after>` name. `mandatory="false"` makes it optional: a warning or an error still shows on its own light and offers its fix, but leaves the page's overall status and issue count alone - for cleanups a user may or may not want. `true` when left out. |
| `<run kind os instance namespace>` | How the check runs. Several are allowed, one per system: the first whose `os` matches runs. None matching makes the check `skip`. |
| `<requires check status>` | Another check that has to be in `status` (by default: whatever that check calls healthy) before this one runs. When it is not, this one is `blocked`, saying what it waits on. |
| `<expect status>` | The statuses that count as healthy, e.g. `ok skip`. |
| `<fix title kind os confirm>` | How to put the check right. Like `<run>`, one per system. `confirm` defaults to `true`. |
| `<after check status>` | What has to hold once the fix has run. Left out, the check itself has to be healthy again. |

`kind` is one of:

| kind | runs | on Windows | elsewhere |
| --- | --- | --- | --- |
| `python` | a `.py` script, with its arguments | `python` | `python3` |
| `powershell` | a `.ps1` script | `powershell.exe` | `pwsh` |
| `shell` | a bash script | Git Bash | `bash` |
| `objectscript` | an expression (check) or an entry reference (fix) | `irissession` | `iris session` |

`os` is a list: `windows`, `linux`, `macos`, or `any` (the default).

Checks run in the order their requirements allow, then in file order. The Doctor refuses a file
with two checks of the same id, a requirement on a check that is not there, or checks that require
each other in a loop, and says which.

### The runtimes

Every check needs the program its `kind` runs, so the Doctor adds a check for each runtime the
file uses (Python, Git Bash, PowerShell, IRIS) at the top of the page, and makes every check
require its own. With no Python on the machine, the Python checks say `blocked` behind a red
**Python**, rather than every one of them failing on its own. A file can replace a runtime check by
declaring one with the same id: `runtime.python`, `runtime.shell`, `runtime.powershell`,
`runtime.iris`.

## The answer

A check answers with **one line of JSON on standard output**: the last line that parses as an
object with a `status`. Anything printed before it is ignored, so a script may log as it goes.

```json
{"status": "warn", "summary": "2 of 3 images behind", "detail": ["csw-desenv: 8.0.46b2, 8.0.46b3 released"]}
```

| field | |
| --- | --- |
| `status` | `ok`, `warn`, `error`, `skip` (does not apply here) or `unknown` (could not tell). |
| `summary` | One line, shown under the check's title. |
| `detail` | Text, or a list of lines. Shown when the check is opened, and in the copied report. |

A check that crashes, times out, prints nothing readable or names a status the Doctor does not
know comes back `unknown`, with the end of what it printed as the detail. **A broken check is never
green.**

A script is started in the checks file's folder, with:

| variable | |
| --- | --- |
| `CONSISTERM_DOCTOR` | `1` |
| `CONSISTERM_DOCTOR_DIR` | the checks file's folder |
| `CONSISTERM_CHECK` | the check's id |
| `CONSISTERM_OS` | `windows`, `linux` or `macos` |
| `CONSISTERM_IRIS_INSTANCE`, `CONSISTERM_IRIS_NAMESPACE` | where ObjectScript runs |
| `DOCTOR_<NAME>` | every `<var>` |

`examples/doctor/lib/doctor.py` has a `result(status, summary, detail)` that prints the line and
exits, and a `run([...])` that runs a command without a console window.

### ObjectScript

An ObjectScript check is an **expression whose value is the JSON line**, evaluated in a session of
the instance with nothing typed at a login prompt. It works only on an instance that lets the
operating system's user in. The built-in **IRIS** check finds that out first, and every ObjectScript
check waits on it.

```xml
<run kind="objectscript" namespace="%SYS">$$Check^CSWDOCTOR("{{comp}}")</run>
```

The instance is the `<run>`'s `instance`, else the file's `instance` variable, else the default
session's, else the first one installed. The namespace is the `<run>`'s, else the file's
`namespace` variable, else `USER`.

## Fixes

**Fix** opens a tab running the fix, so everything it prints is seen and anything it asks (a
password to `consistem_install`, a confirmation) can be answered there. A fix never runs hidden. When
it has finished, **Verify** runs the `<after>` checks again (and whatever depends on them) and says
whether the fix did what it promised.

- A `python`, `powershell` or `shell` fix runs its script in the checks file's folder. It gets the
  file's variables through `{{name}}` on its command line only: a tab inherits the app's
  environment, not the `DOCTOR_*` variables.
- An `objectscript` fix is an **entry reference**, `Label^ROUTINE` or `##class(Pkg.Class).Method()`,
  run instead of the prompt in a tab on the instance.
- A fix never takes a password as an argument: it would be in the process list and in the tab's
  log. It asks for it in the tab.

Installing an environment from scratch is the same thing on a bare machine: every check red, and
the fixes run from the top of the list down, each verified before the next.

## The page

- The checks run when the page is first opened, and when **Re-run checks** is pressed, which also
  reads the file again. Never on a timer: an idle terminal costs nothing.
- **Check again**, on an open check, runs it and everything that depends on it.
- **Copy report** puts every result on the clipboard as text, without the user's home folder or
  user name.

## Self-test

[`examples/doctor-selftest/`](../examples/doctor-selftest/) is a checks file that exercises every
runtime doing what real checks and fixes do: folders and files created, copied, moved, renamed and
deleted; one line of a `consistem.env`-like file edited; a `git pull` bringing a clone up to date;
and whether the check runs elevated. One PowerShell check warns when it is not, so its fix -
`elevated_fix.ps1`, which asks Windows for an administrator through the UAC prompt - can be tried.
Everything happens in a temporary folder that is removed afterwards; the ObjectScript checks use
only temporary files and a process-private global (`^||`), and write to no database.

Point **Checks file** at it, or run it without the app:

```bash
cargo test --lib selftest_pack -- --ignored --nocapture
CONSISTERM_SELFTEST_IRIS=1 cargo test --lib selftest_pack -- --ignored --nocapture   # with the ObjectScript half
```

The second form opens a session on the first local instance, so it is opt-in.

The engine's own attacks on each runtime (`src/features/doctor/adversarial_kinds.rs`) start real
interpreters too, so they are ignored in the everyday suite and run on request, as the release
workflow does on Windows:

```bash
cargo test --lib adversarial_kinds -- --ignored
```

A check runs as the user who opened the app: the Doctor never elevates one. A fix that needs an
administrator asks for one itself, as `elevated_fix.ps1` shows.
