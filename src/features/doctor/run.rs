//! Running one check, and reading what it answered.
//!
//! The contract with a check is one line of JSON on standard output - the last
//! line that parses as an object with a `status` - so a script may print what
//! it likes before it. Anything else (a crash, a timeout, a program that is
//! not there, a line with no status) comes back as `unknown`, with what the
//! program did print kept as the detail: a check that cannot say is never
//! counted as a healthy one.
//!
//! An ObjectScript check is an expression, written into a session's standard
//! input between a marker the doctor finds again in the output. That needs an
//! instance that lets the operating system's user in without asking, which is
//! what the built-in IRIS check finds out first.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};

use super::spec::{Check, Command, Kind, Os, Spec, Status};
use crate::config::profile::{Profile, ShellCommand};
use crate::i18n::{tr, tr1, tr2};

/// What a check came to.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub status: Status,
    /// One line, shown beside the check's title.
    pub summary: String,
    /// Shown when the check is opened: what the script said at length, or
    /// what it printed when it said nothing the doctor could read.
    pub detail: String,
    pub took: Duration,
}

impl Outcome {
    pub fn new(status: Status, summary: impl Into<String>) -> Outcome {
        Outcome {
            status,
            summary: summary.into(),
            detail: String::new(),
            took: Duration::ZERO,
        }
    }

    fn with_detail(mut self, detail: impl Into<String>) -> Outcome {
        self.detail = detail.into();
        self
    }
}

/// Everything about the machine a check runs against that is not in the file.
#[derive(Clone, Debug)]
pub struct Context {
    pub os: Os,
    /// The program that starts Python, and any arguments it needs first.
    pub python: Vec<String>,
    /// The IRIS instance ObjectScript runs in, unless a command names one.
    pub instance: Option<String>,
    pub namespace: String,
}

impl Context {
    /// The file's own settings first - a `python` attribute, `instance` and
    /// `namespace` variables - then the app's default session.
    pub fn new(spec: &Spec, default_instance: Option<String>) -> Context {
        let python = spec
            .python
            .as_deref()
            .map(split_args)
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| vec![if cfg!(windows) { "python" } else { "python3" }.to_string()]);
        Context {
            os: Os::current(),
            python,
            instance: spec
                .var("instance")
                .map(str::to_string)
                .or(default_instance)
                .filter(|i| !i.is_empty()),
            namespace: spec.var("namespace").unwrap_or("USER").to_string(),
        }
    }
}

/// Runs `check` and reads its answer. Blocks for as long as the check takes,
/// up to its timeout - call it off the UI thread.
pub fn run(spec: &Spec, check: &Check, ctx: &Context) -> Outcome {
    let started = Instant::now();
    let mut outcome = if check.builtin {
        runtime(check, ctx)
    } else {
        match check.command(ctx.os) {
            None => Outcome::new(Status::Skip, tr1("Not for {}", ctx.os.key())),
            Some(command) => run_command(spec, check, command, ctx),
        }
    };
    outcome.took = started.elapsed();
    outcome
}

fn run_command(spec: &Spec, check: &Check, command: &Command, ctx: &Context) -> Outcome {
    let target = expand(&command.target, spec);
    if command.kind == Kind::ObjectScript {
        let instance = command.instance.clone().or_else(|| ctx.instance.clone());
        let namespace = command.namespace.as_deref().unwrap_or(&ctx.namespace);
        return objectscript(
            instance.as_deref(),
            namespace,
            &target,
            check.timeout,
            answer,
        );
    }
    let (program, args) = match program_and_args(spec, command.kind, &target, ctx) {
        Ok(found) => found,
        Err(e) => return Outcome::new(Status::Unknown, format!("{e:#}")),
    };
    let mut process = std::process::Command::new(&program);
    process.args(&args).current_dir(&spec.dir);
    environment(&mut process, spec, check, ctx);
    match execute(process, None, check.timeout) {
        Err(failure) => failure.outcome(&program),
        Ok(done) => match answer(&done.stdout) {
            Some(outcome) => outcome,
            None => Outcome::new(
                Status::Unknown,
                match done.code {
                    Some(code) => tr1("No answer (exit code {})", &code.to_string()),
                    None => tr("No answer").to_string(),
                },
            )
            .with_detail(done.transcript()),
        },
    }
}

/// The program that runs a script of `kind`, and its arguments: the script,
/// found relative to the checks file, then whatever followed it.
fn program_and_args(
    spec: &Spec,
    kind: Kind,
    target: &str,
    ctx: &Context,
) -> Result<(PathBuf, Vec<String>)> {
    let mut words = split_args(target);
    if words.is_empty() {
        return Err(anyhow!(tr("Nothing to run")));
    }
    let script = spec.dir.join(words.remove(0));
    let script_arg = |forward: bool| {
        let text = script.display().to_string();
        if forward {
            text.replace('\\', "/")
        } else {
            text
        }
    };
    let (program, mut args) = match kind {
        Kind::Python => {
            let mut python = ctx.python.clone();
            let program = PathBuf::from(python.remove(0));
            python.push(script_arg(false));
            (program, python)
        }
        Kind::PowerShell => (powershell(), powershell_args(script_arg(false))),
        // Git Bash reads a Windows path with forward slashes as one; with
        // backslashes it would read them as escapes.
        Kind::Shell => (
            bash().ok_or_else(|| anyhow!(tr("Git Bash not found")))?,
            vec![script_arg(true)],
        ),
        Kind::ObjectScript => return Err(anyhow!("ObjectScript is not a script")),
    };
    args.extend(words);
    Ok((program, args))
}

fn powershell() -> PathBuf {
    PathBuf::from(if cfg!(windows) {
        "powershell.exe"
    } else {
        "pwsh"
    })
}

fn powershell_args(script: String) -> Vec<String> {
    [
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
    ]
    .iter()
    .map(|s| s.to_string())
    .chain([script])
    .collect()
}

/// Git Bash on Windows, where `bash` on PATH may well be WSL's; `bash`
/// elsewhere.
pub fn bash() -> Option<PathBuf> {
    if cfg!(windows) {
        ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
            .iter()
            .filter_map(|var| std::env::var(var).ok())
            .flat_map(|base| {
                [
                    Path::new(&base).join(r"Git\bin\bash.exe"),
                    Path::new(&base).join(r"Programs\Git\bin\bash.exe"),
                ]
            })
            .find(|path| path.is_file())
    } else {
        Some(PathBuf::from("bash"))
    }
}

/// What every check is told, besides its arguments: where it is, what system
/// it is on, the instance, and every variable of the file as `DOCTOR_<NAME>`.
fn environment(process: &mut std::process::Command, spec: &Spec, check: &Check, ctx: &Context) {
    process
        .env("CONSISTERM_DOCTOR", "1")
        .env("CONSISTERM_DOCTOR_DIR", &spec.dir)
        .env("CONSISTERM_CHECK", &check.id)
        .env("CONSISTERM_OS", ctx.os.key())
        .env("CONSISTERM_IRIS_NAMESPACE", &ctx.namespace)
        // A script's accents arrive as UTF-8 whatever the console's codepage.
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8");
    // Git Bash expands wildcards in its own command line on Windows, which
    // bash anywhere else never does: a `*` passed to a check arrived as the
    // names of the files in its folder.
    if cfg!(windows) {
        let msys = std::env::var("MSYS").unwrap_or_default();
        process.env("MSYS", format!("{msys} noglob").trim());
    }
    if let Some(instance) = &ctx.instance {
        process.env("CONSISTERM_IRIS_INSTANCE", instance);
    }
    for (name, value) in &spec.vars {
        let key: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect();
        process.env(format!("DOCTOR_{key}"), value);
    }
}

/// `{{name}}` replaced by the file's variable of that name. One that is not
/// there is left as written, so the script shows what it was not given.
pub fn expand(text: &str, spec: &Spec) -> String {
    super::spec::fill(text, &spec.vars)
}

/// Words as a shell would split them: on spaces, with quotes holding a word
/// that has spaces in it together.
pub fn split_args(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    for ch in text.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '"' | '\'') => {
                quote = Some(ch);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started || !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            (None, c) => word.push(c),
        }
    }
    if started || !word.is_empty() {
        words.push(word);
    }
    words
}

/// The last line of `stdout` that is a JSON object with a `status`.
pub fn answer(stdout: &str) -> Option<Outcome> {
    stdout.lines().rev().find_map(|line| {
        // Windows PowerShell can put a byte order mark ahead of its output.
        let line = line.trim().trim_start_matches('\u{feff}');
        if !line.starts_with('{') {
            return None;
        }
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        let object = value.as_object()?;
        let said = object.get("status")?.as_str()?;
        let text = |key: &str| match object.get(key) {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Array(lines)) => lines
                .iter()
                .map(|l| {
                    l.as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| l.to_string())
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Some(other) if !other.is_null() => other.to_string(),
            _ => String::new(),
        };
        // `blocked` is the doctor's to give, not a script's.
        let outcome = match Status::parse(said).filter(|s| *s != Status::Blocked) {
            Some(status) => Outcome::new(status, text("summary")),
            None => Outcome::new(
                Status::Unknown,
                tr1("Unrecognised status {}", &format!("{said:?}")),
            ),
        };
        Some(outcome.with_detail(text("detail")))
    })
}

// ---------------------------------------------------------------------------
// The runtimes
// ---------------------------------------------------------------------------

fn runtime(check: &Check, ctx: &Context) -> Outcome {
    let kind = check.runs.first().map(|c| c.kind).unwrap_or(Kind::Python);
    match kind {
        Kind::Python => {
            let mut python = ctx.python.clone();
            let program = PathBuf::from(python.remove(0));
            let mut process = std::process::Command::new(&program);
            process.args(&python).arg("--version");
            version(process, &program, check.timeout, |text| {
                if text.starts_with("Python 2") {
                    Some(tr("Python 2 found; the checks need Python 3").to_string())
                } else {
                    None
                }
            })
        }
        Kind::Shell => match bash() {
            None => Outcome::new(Status::Error, tr("Git Bash not found")),
            Some(program) => {
                let mut process = std::process::Command::new(&program);
                process.arg("--version");
                version(process, &program, check.timeout, |_| None)
            }
        },
        Kind::PowerShell => {
            let program = powershell();
            let mut process = std::process::Command::new(&program);
            process.args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "'PowerShell ' + $PSVersionTable.PSVersion.ToString()",
            ]);
            version(process, &program, check.timeout, |_| None)
        }
        Kind::ObjectScript => objectscript(
            ctx.instance.as_deref(),
            &ctx.namespace,
            "$ZVERSION",
            check.timeout,
            // The version is the first line of what the session wrote.
            |written| {
                let line = written.lines().next().unwrap_or_default().trim();
                (!line.is_empty()).then(|| Outcome::new(Status::Ok, line))
            },
        ),
    }
}

/// Runs a `--version` and calls the runtime healthy when it answered, with
/// the first line it gave as the summary - unless `wrong` objects to it.
fn version(
    process: std::process::Command,
    program: &Path,
    timeout: Duration,
    wrong: impl Fn(&str) -> Option<String>,
) -> Outcome {
    match execute(process, None, timeout) {
        Err(failure) => {
            let mut outcome = failure.outcome(program);
            if matches!(failure, Failure::Start(_)) {
                outcome.status = Status::Error;
            }
            outcome
        }
        Ok(done) => {
            // Python 2 printed its version on standard error.
            let first = done
                .stdout
                .lines()
                .chain(done.stderr.lines())
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or_default();
            let first = first.split(" (").next().unwrap_or(first).to_string();
            match (done.code, wrong(&first)) {
                (Some(0), None) => Outcome::new(Status::Ok, first),
                (_, Some(problem)) => Outcome::new(Status::Error, problem),
                (code, None) => Outcome::new(
                    Status::Error,
                    tr2(
                        "{} failed (exit code {})",
                        &program.display().to_string(),
                        &code.unwrap_or(-1).to_string(),
                    ),
                )
                .with_detail(done.transcript()),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ObjectScript
// ---------------------------------------------------------------------------

/// Brackets the value an ObjectScript check writes, so it is found again in
/// whatever else the session prints - a banner, a prompt.
pub(super) const MARK: &str = "<<CONSISTERM-DOCTOR>>";

/// Evaluates `expression` in a session of `instance` and hands the text it
/// wrote to `read`.
fn objectscript(
    instance: Option<&str>,
    namespace: &str,
    expression: &str,
    timeout: Duration,
    read: impl Fn(&str) -> Option<Outcome>,
) -> Outcome {
    let Some(instance) = instance else {
        return Outcome::new(
            Status::Error,
            tr("No IRIS instance: set a default session, or <var name=\"instance\"> in the checks file"),
        );
    };
    let (program, args) = match session_command(instance, namespace) {
        Ok(found) => found,
        Err(e) => return Outcome::new(Status::Error, e.to_string()),
    };
    let mut process = std::process::Command::new(&program);
    process.args(&args);
    // A failed expression drops the session into its error prompt; the
    // `halt` after it ends that too.
    let input = format!("write !,\"{MARK}\",{expression},!\r\nhalt\r\n");
    match execute(process, Some(input), timeout) {
        Err(failure) => failure.outcome(&program),
        Ok(done) => {
            // The session echoes what it was given, so the marker is there
            // twice: in the command, and in front of what it wrote. The last
            // one that reads is the answer.
            let written = marked(&done.stdout);
            match written.and_then(&read) {
                Some(outcome) => outcome,
                None if done.stdout.to_ascii_lowercase().contains("username") => Outcome::new(
                    Status::Error,
                    tr("IRIS asked for a login; ObjectScript checks need OS authentication"),
                ),
                None => Outcome::new(
                    Status::Unknown,
                    tr(if written.is_some() {
                        "The expression wrote no status"
                    } else {
                        "IRIS did not answer"
                    }),
                )
                .with_detail(done.transcript()),
            }
        }
    }
}

/// Everything the expression wrote: what follows the last line that
/// *starts* with the marker. The session echoes the command it was given, so
/// the marker is also in the echo - but there in the middle of a line, after
/// the prompt, where the `write !` in front of it never leaves it. And an
/// expression may write lines of its own before its value, so the value is
/// not always on the marker's line: the whole rest is handed on, and the
/// reader picks the answer out of it.
pub(super) fn marked(stdout: &str) -> Option<&str> {
    let mut at = None;
    let mut offset = 0;
    for line in stdout.split_inclusive('\n') {
        if line.trim_start().starts_with(MARK) {
            at = Some(offset + line.find(MARK).unwrap_or(0) + MARK.len());
        }
        offset += line.len();
    }
    at.map(|at| &stdout[at..])
}

/// The session binary for `instance` and the arguments that open `namespace`.
fn session_command(instance: &str, namespace: &str) -> Result<(PathBuf, Vec<String>)> {
    let launcher = crate::pty::launcher::launcher();
    let bin = crate::pty::launcher::instances(&*launcher)
        .into_iter()
        .find(|i| i.name.eq_ignore_ascii_case(instance))
        .and_then(|i| i.bin_dir);
    let mut args = Vec::new();
    let program = if cfg!(windows) {
        bin.map(|b| b.join("irissession.exe"))
            .ok_or_else(|| anyhow!(tr1("Instance {} not found on this machine", instance)))?
    } else {
        args.push("session".to_string());
        bin.map(|b| b.join("iris"))
            .unwrap_or_else(|| PathBuf::from("iris"))
    };
    args.extend([
        instance.to_string(),
        "-U".to_string(),
        namespace.to_string(),
    ]);
    Ok((program, args))
}

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

/// What a program printed, as text, line by line: UTF-8 where a line is, and
/// on Windows the console's code page where it is not - Windows PowerShell
/// writes to a pipe in the console's code page, not in UTF-8, so its accents
/// arrived as replacement marks. CP850 is that page on a Western or Brazilian
/// Windows. Line by line, because one output can mix both: a script's own
/// UTF-8 and a tool it ran that wrote in the console's.
pub(super) fn decode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for line in bytes.split_inclusive(|b| *b == b'\n') {
        match std::str::from_utf8(line) {
            Ok(text) => out.push_str(text),
            Err(_) if cfg!(windows) => {
                let utf8 = crate::term::Encoding::Cp850.decode(line);
                out.push_str(&String::from_utf8_lossy(&utf8));
            }
            Err(_) => out.push_str(&String::from_utf8_lossy(line)),
        }
    }
    out
}

pub(super) struct Finished {
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) code: Option<i32>,
}

impl Finished {
    /// The end of what the program printed, for a check that said nothing
    /// readable: usually the traceback that explains why.
    fn transcript(&self) -> String {
        let tail = |text: &str| {
            let lines: Vec<&str> = text.lines().collect();
            lines[lines.len().saturating_sub(20)..].join("\n")
        };
        [tail(&self.stderr), tail(&self.stdout)]
            .into_iter()
            .filter(|t| !t.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(super) enum Failure {
    Start(std::io::Error),
    Timeout(Duration),
}

impl Failure {
    fn outcome(&self, program: &Path) -> Outcome {
        match self {
            Failure::Start(e) => Outcome::new(
                Status::Unknown,
                tr2(
                    "Could not start {}: {}",
                    &program.display().to_string(),
                    &e.to_string(),
                ),
            ),
            Failure::Timeout(after) => Outcome::new(
                Status::Unknown,
                tr1("Timed out after {} s", &after.as_secs().to_string()),
            ),
        }
    }
}

/// Runs `process` to the end or to `timeout`, whichever is first, feeding it
/// `input`. Both outputs are read on threads of their own, so a program that
/// fills one pipe while the other is waited on cannot stall.
pub(super) fn execute(
    mut process: std::process::Command,
    input: Option<String>,
    timeout: Duration,
) -> std::result::Result<Finished, Failure> {
    process
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window flashing up for every check.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        process.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = process.spawn().map_err(Failure::Start)?;
    // Written on a thread of its own: a program that does not read its input
    // fills the pipe, and a write waiting on it here would hold the run past
    // the timeout below, which is the only thing that can end it.
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        std::thread::spawn(move || {
            let _ = stdin.write_all(text.as_bytes());
        });
    }
    // Each output read into a buffer of its own as it comes, rather than
    // read to its end: a check that leaves a process behind - a daemon it
    // started - leaves that process holding the pipe, and the end would only
    // come when it exits. Once the check itself has, what was read is enough.
    type Buffer = std::sync::Arc<std::sync::Mutex<Vec<u8>>>;
    let reader = |pipe: Option<Box<dyn Read + Send>>| {
        let buffer: Buffer = Default::default();
        let into = buffer.clone();
        let thread = std::thread::spawn(move || {
            let Some(mut pipe) = pipe else {
                return;
            };
            let mut chunk = [0u8; 8192];
            while let Ok(n) = pipe.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                if let Ok(mut b) = into.lock() {
                    b.extend_from_slice(&chunk[..n]);
                }
            }
        });
        (buffer, thread)
    };
    let (stdout, out_thread) = reader(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let (stderr, err_thread) = reader(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );

    let deadline = Instant::now() + timeout;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Failure::Timeout(timeout));
            }
        }
    };
    // What is still in the pipes when the check exits arrives at once; a
    // second is for a slow machine, not for a process left behind.
    let grace = Instant::now() + Duration::from_secs(1);
    while !(out_thread.is_finished() && err_thread.is_finished()) && Instant::now() < grace {
        std::thread::sleep(Duration::from_millis(10));
    }
    let text = |buffer: &Buffer| buffer.lock().map(|b| decode(&b)).unwrap_or_default();
    Ok(Finished {
        stdout: text(&stdout),
        stderr: text(&stderr),
        code,
    })
}

// ---------------------------------------------------------------------------
// Fixes
// ---------------------------------------------------------------------------

/// The tab a fix runs in, named `name`: the script in a terminal, in view,
/// so whatever it prints is seen and whatever it asks can be answered.
///
/// A fix is told the file's variables through `{{name}}` in its command line
/// only: a tab's program inherits the app's environment and nothing more.
pub fn fix_profile(spec: &Spec, check: &Check, ctx: &Context, name: String) -> Result<Profile> {
    let command = check
        .fix_command(ctx.os)
        .ok_or_else(|| anyhow!(tr1("No fix for {}", ctx.os.key())))?;
    let target = expand(&command.target, spec);
    if command.kind == Kind::ObjectScript {
        // An entry reference, run instead of the prompt.
        return Ok(Profile {
            name: name.clone(),
            instance: command
                .instance
                .clone()
                .or_else(|| ctx.instance.clone())
                .ok_or_else(|| anyhow!(tr("No IRIS instance")))?,
            namespace: command
                .namespace
                .clone()
                .unwrap_or_else(|| ctx.namespace.clone()),
            routine: Some(target),
            ..Profile::default()
        });
    }
    let mut words = split_args(&target);
    if words.is_empty() {
        return Err(anyhow!(tr("Nothing to run")));
    }
    // Named as written, relative to the checks file the tab starts in: a
    // shell tab's command line on Windows cannot quote a space.
    let script = words.remove(0);
    let (program, mut args) = match command.kind {
        Kind::Python => {
            let mut python = ctx.python.clone();
            let program = PathBuf::from(python.remove(0));
            python.push(script);
            (program, python)
        }
        Kind::PowerShell => {
            let mut args = powershell_args(script);
            args.retain(|a| a != "-NonInteractive");
            (powershell(), args)
        }
        Kind::Shell => (
            bash().ok_or_else(|| anyhow!(tr("Git Bash not found")))?,
            vec![script.replace('\\', "/")],
        ),
        Kind::ObjectScript => unreachable!("handled above"),
    };
    args.extend(words);
    Ok(Profile {
        instance: name.clone(),
        name,
        namespace: String::new(),
        shell: Some(ShellCommand {
            program,
            args,
            cwd: Some(spec.dir.clone()),
        }),
        ..Profile::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::doctor::spec;

    fn spec_at(dir: &Path, xml: &str) -> Spec {
        let path = dir.join("doctor.xml");
        std::fs::write(&path, xml).unwrap();
        spec::load(&path).expect("parses")
    }

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nit-doctor-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Python is a prerequisite of the environments this is for, but not of
    /// the machine building it: without one, the tests that need it say so
    /// and pass.
    fn python(ctx: &Context) -> bool {
        let found = std::process::Command::new(&ctx.python[0])
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success());
        if !found {
            eprintln!("no {} here; skipping", ctx.python[0]);
        }
        found
    }

    #[test]
    fn the_last_json_line_with_a_status_is_the_answer() {
        let out = "starting\n{\"not\": \"this\"}\n{\"status\":\"warn\",\"summary\":\"2 behind\",\"detail\":[\"a\",\"b\"]}\nbye\n";
        let outcome = answer(out).expect("an answer");
        assert_eq!(outcome.status, Status::Warn);
        assert_eq!(outcome.summary, "2 behind");
        assert_eq!(outcome.detail, "a\nb");
    }

    #[test]
    fn a_status_the_doctor_does_not_know_is_unknown_never_ok() {
        for said in ["fine", "blocked"] {
            let outcome = answer(&format!("{{\"status\":\"{said}\"}}")).expect("an answer");
            assert_eq!(outcome.status, Status::Unknown, "{said}");
        }
        assert!(answer("all good!\n").is_none());
    }

    #[test]
    fn an_echoed_command_is_not_mistaken_for_the_answer() {
        // What irissession prints for a piped command: the command echoed
        // after the prompt, marker and all, then the value it wrote.
        let out = format!(
            "USER>write !,\"{MARK}\",$select(1:\"{{\"\"status\"\":1}}\"),!
{MARK}{{\"status\":\"ok\",\"summary\":\"yes\"}}
USER>halt
"
        );
        let outcome = marked(&out).and_then(answer).unwrap();
        assert_eq!(
            (outcome.status, outcome.summary.as_str()),
            (Status::Ok, "yes")
        );
    }

    #[test]
    fn variables_are_filled_in_and_unknown_ones_left_showing() {
        let spec = spec::parse(
            r#"<doctor><var name="comp" value="COMP"/>
                 <check id="a"><run kind="python">a.py</run></check></doctor>"#,
            Path::new("d.xml"),
            Os::current(),
        )
        .unwrap();
        assert_eq!(
            expand("x {{comp}} {{ COMP }} {{nope}} {{", &spec),
            "x COMP COMP {{nope}} {{"
        );
    }

    #[test]
    fn arguments_split_on_spaces_and_quotes_hold_them_together() {
        assert_eq!(
            split_args(r#"a.py check "two words" '' last"#),
            ["a.py", "check", "two words", "", "last"]
        );
    }

    #[test]
    fn a_python_check_is_run_and_read() {
        let dir = tempdir("python");
        std::fs::write(
            dir.join("ok.py"),
            "import os, json, sys\nprint('noise')\nprint(json.dumps({'status': 'ok', 'summary': os.environ['DOCTOR_COMP'] + ' ' + sys.argv[1]}))\n",
        )
        .unwrap();
        std::fs::write(dir.join("crash.py"), "raise SystemExit('broken here')\n").unwrap();
        std::fs::write(dir.join("slow.py"), "import time\ntime.sleep(30)\n").unwrap();
        let spec = spec_at(
            &dir,
            r#"<doctor><var name="comp" value="COMP"/>
                 <check id="ok"><run kind="python">ok.py {{comp}}</run></check>
                 <check id="crash"><run kind="python">crash.py</run></check>
                 <check id="slow" timeout="1"><run kind="python">slow.py</run></check>
                 <check id="elsewhere"><run kind="python" os="beos-never">x.py</run></check>
               </doctor>"#
                .replace(
                    r#" os="beos-never""#,
                    if cfg!(windows) {
                        r#" os="linux""#
                    } else {
                        r#" os="windows""#
                    },
                )
                .as_str(),
        );
        let ctx = Context::new(&spec, None);
        if !python(&ctx) {
            return;
        }
        let ok = run(&spec, spec.check("ok").unwrap(), &ctx);
        assert_eq!((ok.status, ok.summary.as_str()), (Status::Ok, "COMP COMP"));

        let crash = run(&spec, spec.check("crash").unwrap(), &ctx);
        assert_eq!(crash.status, Status::Unknown);
        assert!(crash.detail.contains("broken here"), "{crash:?}");

        let slow = run(&spec, spec.check("slow").unwrap(), &ctx);
        assert_eq!(slow.status, Status::Unknown);
        assert!(slow.summary.contains("Timed out"), "{slow:?}");
        assert!(slow.took < Duration::from_secs(10));

        let elsewhere = run(&spec, spec.check("elsewhere").unwrap(), &ctx);
        assert_eq!(elsewhere.status, Status::Skip);

        let runtime = run(&spec, spec.check("runtime.python").unwrap(), &ctx);
        assert_eq!(runtime.status, Status::Ok, "{runtime:?}");
        assert!(runtime.summary.starts_with("Python 3"), "{runtime:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_runtime_that_is_not_installed_is_an_error() {
        let spec = spec::parse(
            r#"<doctor python="no-such-python-here"><check id="a"><run kind="python">a.py</run></check></doctor>"#,
            Path::new("d.xml"),
            Os::current(),
        )
        .unwrap();
        let ctx = Context::new(&spec, None);
        let outcome = run(&spec, spec.check("runtime.python").unwrap(), &ctx);
        assert_eq!(outcome.status, Status::Error, "{outcome:?}");
    }

    #[test]
    fn a_fix_opens_a_tab_running_its_script_where_the_file_is() {
        let spec = spec::parse(
            r#"<doctor><var name="svc" value="desenv"/>
                 <check id="a"><run kind="python">a.py</run>
                   <fix kind="python">fixes/up.py {{svc}}</fix></check></doctor>"#,
            Path::new("/packs/doctor.xml"),
            Os::current(),
        )
        .unwrap();
        let ctx = Context::new(&spec, Some("IRIS".into()));
        let profile = fix_profile(&spec, spec.check("a").unwrap(), &ctx, "Fix: a".into()).unwrap();
        let shell = profile.shell.expect("a shell tab");
        assert_eq!(shell.cwd.as_deref(), Some(Path::new("/packs")));
        assert_eq!(
            shell.args[shell.args.len() - 2..],
            ["fixes/up.py", "desenv"]
        );
        assert_eq!(profile.name, "Fix: a");
    }
}
