//! The checks file: what each check is, how it runs on which system, what it
//! needs before it can run, and how it is fixed.
//!
//! Read from XML, like the organisation's macros, because the same people
//! write both and the file travels the same way - see `docs/doctor.md` for the
//! format as a script author sees it.
//!
//! Loading also adds the runtimes the checks need: a Python check cannot say
//! anything about Docker on a machine without Python, so every check is made
//! to require the runtime of the command it will run, and that runtime is a
//! check of its own at the top of the list. A file that declares a check with
//! a runtime's id replaces the built-in one.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

/// What a check found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    Ok,
    Warn,
    Error,
    /// The check does not apply here - no Docker on this machine, say.
    Skip,
    /// The check could not say: it crashed, timed out, or printed nothing the
    /// doctor could read. Never green, so a broken check is never mistaken
    /// for a healthy environment.
    Unknown,
    /// Not run, because a check it requires is not where it has to be.
    Blocked,
}

impl Status {
    pub const ALL: [Status; 6] = [
        Status::Ok,
        Status::Warn,
        Status::Error,
        Status::Skip,
        Status::Unknown,
        Status::Blocked,
    ];

    /// The word in the file and in a script's answer.
    pub fn key(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Warn => "warn",
            Status::Error => "error",
            Status::Skip => "skip",
            Status::Unknown => "unknown",
            Status::Blocked => "blocked",
        }
    }

    pub fn parse(text: &str) -> Option<Status> {
        let text = text.trim().to_ascii_lowercase();
        Status::ALL.into_iter().find(|s| s.key() == text)
    }
}

/// An operating system a command is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
    Macos,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Macos
        } else {
            Os::Linux
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Os::Windows => "windows",
            Os::Linux => "linux",
            Os::Macos => "macos",
        }
    }

    fn parse(text: &str) -> Option<Os> {
        match text.trim().to_ascii_lowercase().as_str() {
            "windows" | "win" => Some(Os::Windows),
            "linux" => Some(Os::Linux),
            "macos" | "mac" | "darwin" => Some(Os::Macos),
            _ => None,
        }
    }
}

/// What runs a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A Python script - the default for anything that runs on every system.
    Python,
    PowerShell,
    /// A shell script: Git Bash on Windows, `bash` elsewhere.
    Shell,
    /// An ObjectScript expression for a check (`$$Check^ROUTINE()`,
    /// `##class(X).Check()`), an entry reference for a fix.
    ObjectScript,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Python => "python",
            Kind::PowerShell => "powershell",
            Kind::Shell => "shell",
            Kind::ObjectScript => "objectscript",
        }
    }

    fn parse(text: &str) -> Option<Kind> {
        match text.trim().to_ascii_lowercase().as_str() {
            "python" | "py" => Some(Kind::Python),
            "powershell" | "ps1" | "pwsh" => Some(Kind::PowerShell),
            "shell" | "sh" | "bash" => Some(Kind::Shell),
            "objectscript" | "cos" | "mac" | "cls" => Some(Kind::ObjectScript),
            _ => None,
        }
    }

    /// The id of the built-in check that says whether this runtime is here.
    pub fn runtime(self) -> &'static str {
        match self {
            Kind::Python => "runtime.python",
            Kind::PowerShell => "runtime.powershell",
            Kind::Shell => "runtime.shell",
            Kind::ObjectScript => "runtime.iris",
        }
    }
}

/// One way of running something, on the systems it names.
#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub kind: Kind,
    /// `None` for every system.
    pub os: Option<Vec<Os>>,
    /// The script and its arguments, or the ObjectScript to run. `{{name}}`
    /// stands for a variable of the file.
    pub target: String,
    /// The IRIS instance an ObjectScript command runs in, when not the
    /// default one.
    pub instance: Option<String>,
    pub namespace: Option<String>,
}

impl Command {
    pub fn runs_on(&self, os: Os) -> bool {
        self.os.as_ref().is_none_or(|list| list.contains(&os))
    }
}

/// A check that has to be in a given state before another one runs, or
/// after a fix has.
#[derive(Clone, Debug, PartialEq)]
pub struct Requirement {
    pub check: String,
    /// `None` for whatever that check itself counts as healthy.
    pub status: Option<Vec<Status>>,
}

/// How a check is put right: run in a tab, in view, and then verified.
#[derive(Clone, Debug, PartialEq)]
pub struct Fix {
    pub title: String,
    /// Ask before running it.
    pub confirm: bool,
    /// One per system at most is used: the first that runs here.
    pub commands: Vec<Command>,
    /// What has to hold once it has run. Empty means the check itself is
    /// healthy again.
    pub after: Vec<Requirement>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub id: String,
    pub title: String,
    pub objective: String,
    pub group: String,
    /// A runtime the doctor added, rather than one the file declared.
    pub builtin: bool,
    pub requires: Vec<Requirement>,
    /// The first that runs here is used; none at all makes the check `skip`.
    pub runs: Vec<Command>,
    /// The statuses that count as healthy. `ok` unless the file says.
    pub expect: Vec<Status>,
    pub fix: Option<Fix>,
    pub timeout: Duration,
    /// Whether a problem here counts against the page's overall status. An
    /// optional check (a cleanup the user may or may not want) still shows its
    /// own light and offers its fix, but leaves the summary green.
    pub mandatory: bool,
}

impl Check {
    /// The command this check runs on `os`.
    pub fn command(&self, os: Os) -> Option<&Command> {
        self.runs.iter().find(|c| c.runs_on(os))
    }

    /// The fix's command on `os`.
    pub fn fix_command(&self, os: Os) -> Option<&Command> {
        self.fix.as_ref()?.commands.iter().find(|c| c.runs_on(os))
    }

    pub fn healthy(&self, status: Status) -> bool {
        self.expect.contains(&status)
    }
}

/// A whole checks file, with its runtimes added and in an order it can run in.
#[derive(Clone, Debug)]
pub struct Spec {
    pub path: PathBuf,
    /// Scripts are named relative to this, and run in it.
    pub dir: PathBuf,
    pub vars: Vec<(String, String)>,
    /// How to start Python, when not `python` (Windows) or `python3`.
    pub python: Option<String>,
    /// Every check, runtimes first, each after everything it requires.
    pub checks: Vec<Check>,
}

impl Spec {
    pub fn check(&self, id: &str) -> Option<&Check> {
        self.checks.iter().find(|c| c.id == id)
    }

    /// Group names in the order they first appear.
    pub fn groups(&self) -> Vec<&str> {
        let mut groups: Vec<&str> = Vec::new();
        for check in &self.checks {
            if !groups.contains(&check.group.as_str()) {
                groups.push(&check.group);
            }
        }
        groups
    }

    pub fn var(&self, name: &str) -> Option<&str> {
        self.vars
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Whether `requirement` holds, given the status its check came to.
    pub fn satisfied(&self, requirement: &Requirement, status: Status) -> bool {
        match &requirement.status {
            Some(wanted) => wanted.contains(&status),
            None => self
                .check(&requirement.check)
                .is_some_and(|check| check.healthy(status)),
        }
    }
}

/// The group the runtimes are listed under. A key, translated when shown.
pub const RUNTIMES_GROUP: &str = "Runtimes";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

pub fn load(path: &Path) -> Result<Spec> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse(&text, path, Os::current())
}

/// Reads `text` as the checks file at `path`, for a machine running `os`.
pub fn parse(text: &str, path: &Path, os: Os) -> Result<Spec> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut vars = Vec::new();
    let mut python = None;
    let mut checks: Vec<Check> = Vec::new();
    let mut group = String::new();
    let mut check: Option<Check> = None;
    // The element whose text is being read: a `<run>` or a `<fix>`.
    let mut open: Option<(Open, BytesStart<'static>)> = None;
    let mut text_buf = String::new();
    let mut root = false;

    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| anyhow!("malformed XML at byte {}: {e}", reader.buffer_position()))?;
        match event {
            Event::Eof => break,
            Event::Start(ref e) | Event::Empty(ref e) => {
                let empty = matches!(event, Event::Empty(_));
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                match tag.as_str() {
                    "doctor" => {
                        root = true;
                        python = attr(e, "python").filter(|p| !p.trim().is_empty());
                    }
                    "var" => vars.push((
                        attr(e, "name").context("<var> without a name")?,
                        attr(e, "value").unwrap_or_default(),
                    )),
                    "group" => group = attr(e, "name").unwrap_or_default(),
                    "check" => {
                        let id = attr(e, "id")
                            .map(|id| id.trim().to_string())
                            .filter(|id| !id.is_empty())
                            .context("<check> without an id")?;
                        let timeout = match attr(e, "timeout") {
                            // A check killed before it can start says nothing.
                            Some(t) => match t.trim().parse::<u64>() {
                                Ok(secs) if secs > 0 => Duration::from_secs(secs),
                                _ => bail!("check {id}: timeout {t:?} is not a number of seconds"),
                            },
                            None => DEFAULT_TIMEOUT,
                        };
                        let mandatory = match attr(e, "mandatory").as_deref().map(str::trim) {
                            None | Some("true" | "1" | "yes") => true,
                            Some("false" | "0" | "no") => false,
                            Some(other) => {
                                bail!("check {id}: mandatory {other:?} is neither true nor false")
                            }
                        };
                        check = Some(Check {
                            mandatory,
                            title: attr(e, "title").unwrap_or_else(|| id.clone()),
                            objective: attr(e, "objective").unwrap_or_default(),
                            group: group.clone(),
                            builtin: false,
                            requires: Vec::new(),
                            runs: Vec::new(),
                            expect: vec![Status::Ok],
                            fix: None,
                            timeout,
                            id,
                        });
                    }
                    "requires" | "after" | "expect" => {
                        let c = check
                            .as_mut()
                            .with_context(|| format!("<{tag}> outside a <check>"))?;
                        let statuses = statuses(e, &c.id)?;
                        match tag.as_str() {
                            "expect" => {
                                c.expect = statuses.with_context(|| {
                                    format!("check {}: <expect> without a status", c.id)
                                })?
                            }
                            _ => {
                                let requirement = Requirement {
                                    check: attr(e, "check").with_context(|| {
                                        format!("check {}: <{tag}> without a check", c.id)
                                    })?,
                                    status: statuses,
                                };
                                if tag == "requires" {
                                    c.requires.push(requirement);
                                } else {
                                    fix_of(c, None).after.push(requirement);
                                }
                            }
                        }
                    }
                    "run" | "fix" => {
                        let what = if tag == "run" { Open::Run } else { Open::Fix };
                        if check.is_none() {
                            bail!("<{tag}> outside a <check>");
                        }
                        if empty {
                            bail!("<{tag}> with nothing to run");
                        }
                        open = Some((what, e.to_owned()));
                        text_buf.clear();
                    }
                    other => bail!("unknown element <{other}>"),
                }
            }
            Event::Text(e) if open.is_some() => {
                text_buf.push_str(&e.unescape().unwrap_or_default());
            }
            Event::CData(e) if open.is_some() => {
                text_buf.push_str(&String::from_utf8_lossy(&e.into_inner()));
            }
            Event::End(ref e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                match tag.as_str() {
                    "run" | "fix" => {
                        if let (Some((what, start)), Some(c)) = (open.take(), check.as_mut()) {
                            let command = command(&start, text_buf.trim(), &c.id)?;
                            match what {
                                Open::Run => c.runs.push(command),
                                Open::Fix => {
                                    let title = attr(&start, "title");
                                    let confirm = attr(&start, "confirm")
                                        .map(|v| matches!(v.as_str(), "true" | "1" | "yes"));
                                    let fix = fix_of(c, title);
                                    if let Some(confirm) = confirm {
                                        fix.confirm = confirm;
                                    }
                                    fix.commands.push(command);
                                }
                            }
                        }
                    }
                    "check" => {
                        if let Some(c) = check.take() {
                            if c.runs.is_empty() {
                                bail!("check {} has no <run>", c.id);
                            }
                            checks.push(c);
                        }
                    }
                    "group" => group.clear(),
                    _ => {}
                }
            }
            _ => {}
        }
        buf.clear();
    }
    if !root {
        bail!("not a checks file: no <doctor> element");
    }

    let dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    // Titles say which namespace, which service: "Namespace {{comp}}".
    for check in &mut checks {
        check.title = fill(&check.title, &vars);
        check.objective = fill(&check.objective, &vars);
        if let Some(fix) = &mut check.fix {
            fix.title = fill(&fix.title, &vars);
        }
    }
    let checks = finish(checks, os)?;
    Ok(Spec {
        path: path.to_path_buf(),
        dir,
        vars,
        python,
        checks,
    })
}

/// `{{name}}` replaced by the variable of that name. One that is not there
/// is left as written, so whoever reads it sees what was not given.
pub fn fill(text: &str, vars: &[(String, String)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        match after.find("}}") {
            Some(close) => {
                let name = after[..close].trim();
                match vars.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
                    Some((_, value)) => out.push_str(value),
                    None => out.push_str(&rest[open..open + 2 + close + 2]),
                }
                rest = &after[close + 2..];
            }
            None => {
                out.push_str(&rest[open..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

#[derive(Clone, Copy)]
enum Open {
    Run,
    Fix,
}

/// The check's fix, made on first mention: `<after>` can come before `<fix>`.
fn fix_of(check: &mut Check, title: Option<String>) -> &mut Fix {
    let fix = check.fix.get_or_insert_with(|| Fix {
        title: String::new(),
        // Asks unless told not to: a fix changes the machine.
        confirm: true,
        commands: Vec::new(),
        after: Vec::new(),
    });
    if let Some(title) = title {
        fix.title = title;
    }
    fix
}

fn command(e: &BytesStart, target: &str, check: &str) -> Result<Command> {
    let kind_text = attr(e, "kind").with_context(|| format!("check {check}: no kind"))?;
    let kind = Kind::parse(&kind_text)
        .with_context(|| format!("check {check}: unknown kind {kind_text:?}"))?;
    let os = match attr(e, "os") {
        None => None,
        Some(list) if list.trim().eq_ignore_ascii_case("any") => None,
        Some(list) => Some(
            words(&list)
                .map(|w| Os::parse(w).with_context(|| format!("check {check}: unknown os {w:?}")))
                .collect::<Result<Vec<_>>>()?,
        ),
    };
    if target.is_empty() {
        bail!(
            "check {check}: an empty <{}>",
            String::from_utf8_lossy(e.name().as_ref())
        );
    }
    Ok(Command {
        kind,
        os,
        target: target.to_string(),
        instance: attr(e, "instance").filter(|s| !s.is_empty()),
        namespace: attr(e, "namespace").filter(|s| !s.is_empty()),
    })
}

/// The `status` attribute: a list, as `ok warn` or `ok,warn`.
fn statuses(e: &BytesStart, check: &str) -> Result<Option<Vec<Status>>> {
    let Some(list) = attr(e, "status") else {
        return Ok(None);
    };
    let parsed = words(&list)
        .map(|w| Status::parse(w).with_context(|| format!("check {check}: unknown status {w:?}")))
        .collect::<Result<Vec<_>>>()?;
    Ok((!parsed.is_empty()).then_some(parsed))
}

fn words(list: &str) -> impl Iterator<Item = &str> {
    list.split([' ', ',', ';', '|'])
        .map(str::trim)
        .filter(|w| !w.is_empty())
}

/// Adds the runtimes, makes each check require its own, checks every
/// requirement names a check, and puts them in an order that can run.
fn finish(mut checks: Vec<Check>, os: Os) -> Result<Vec<Check>> {
    let mut seen = HashSet::new();
    for check in &checks {
        if !seen.insert(check.id.clone()) {
            bail!("two checks are called {}", check.id);
        }
    }

    let mut runtimes: Vec<Check> = Vec::new();
    for check in &mut checks {
        let mut kinds: Vec<Kind> = check.command(os).map(|c| c.kind).into_iter().collect();
        if let Some(fix) = check.fix_command(os) {
            kinds.push(fix.kind);
        }
        for kind in kinds {
            let id = kind.runtime();
            if check.id == id {
                continue;
            }
            // Only for the command the check runs: a check whose fix needs
            // Git Bash can still say what is wrong without it.
            if check.command(os).is_some_and(|c| c.kind == kind)
                && !check.requires.iter().any(|r| r.check == id)
            {
                check.requires.insert(
                    0,
                    Requirement {
                        check: id.to_string(),
                        status: None,
                    },
                );
            }
            if !seen.contains(id) && !runtimes.iter().any(|r| r.id == id) {
                runtimes.push(runtime(kind));
            }
        }
    }
    runtimes.sort_by_key(|r| {
        [
            Kind::Python,
            Kind::Shell,
            Kind::PowerShell,
            Kind::ObjectScript,
        ]
        .iter()
        .position(|k| k.runtime() == r.id)
    });
    runtimes.extend(checks);
    let checks = runtimes;

    let ids: HashSet<&str> = checks.iter().map(|c| c.id.as_str()).collect();
    for check in &checks {
        let fix_after = check.fix.iter().flat_map(|f| f.after.iter());
        for requirement in check.requires.iter().chain(fix_after) {
            if !ids.contains(requirement.check.as_str()) {
                bail!(
                    "check {} refers to {}, which is not in the file",
                    check.id,
                    requirement.check
                );
            }
        }
    }
    order(checks)
}

/// The built-in check for `kind`'s runtime.
fn runtime(kind: Kind) -> Check {
    let title = match kind {
        Kind::Python => "Python",
        Kind::PowerShell => "PowerShell",
        Kind::Shell if cfg!(windows) => "Git Bash",
        Kind::Shell => "Bash",
        Kind::ObjectScript => "IRIS",
    };
    Check {
        id: kind.runtime().to_string(),
        title: title.to_string(),
        objective: runtime_objective(kind.runtime())
            .unwrap_or_default()
            .to_string(),
        group: RUNTIMES_GROUP.to_string(),
        builtin: true,
        requires: Vec::new(),
        runs: vec![Command {
            kind,
            os: None,
            target: String::new(),
            instance: None,
            namespace: None,
        }],
        expect: vec![Status::Ok],
        fix: None,
        timeout: Duration::from_secs(20),
        mandatory: true,
    }
}

/// What a built-in runtime check is for, by its id: a literal, so the page
/// can translate it.
pub fn runtime_objective(id: &str) -> Option<&'static str> {
    Some(match id {
        "runtime.python" => "Python 3 runs, for the checks written in it",
        "runtime.powershell" => "PowerShell runs, for the checks written in it",
        "runtime.shell" => "A bash shell runs, for the shell-script checks",
        "runtime.iris" => "The IRIS instance answers without a login, for the ObjectScript checks",
        _ => return None,
    })
}

/// Every check after the ones it requires, otherwise in file order; an error
/// naming the checks when they require each other round in a loop.
fn order(checks: Vec<Check>) -> Result<Vec<Check>> {
    let index: HashMap<String, usize> = checks
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.clone(), i))
        .collect();
    let mut placed = vec![false; checks.len()];
    let mut out: Vec<usize> = Vec::with_capacity(checks.len());
    while out.len() < checks.len() {
        let next = (0..checks.len()).find(|&i| {
            !placed[i]
                && checks[i]
                    .requires
                    .iter()
                    .all(|r| index.get(&r.check).is_some_and(|&j| placed[j]))
        });
        match next {
            Some(i) => {
                placed[i] = true;
                out.push(i);
            }
            None => {
                let stuck: Vec<&str> = (0..checks.len())
                    .filter(|&i| !placed[i])
                    .map(|i| checks[i].id.as_str())
                    .collect();
                bail!(
                    "these checks require each other in a loop: {}",
                    stuck.join(", ")
                );
            }
        }
    }
    let mut slots: Vec<Option<Check>> = checks.into_iter().map(Some).collect();
    Ok(out.into_iter().filter_map(|i| slots[i].take()).collect())
}

fn attr(e: &BytesStart, key: &str) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        if a.key.as_ref() != key.as_bytes() {
            return None;
        }
        Some(
            a.unescape_value()
                .map(|v| v.into_owned())
                .unwrap_or_else(|_| String::from_utf8_lossy(&a.value).into_owned()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(xml: &str) -> Result<Spec> {
        parse(xml, Path::new("/packs/doctor.xml"), Os::Windows)
    }

    const SAMPLE: &str = r#"
        <doctor python="py -3">
          <var name="comp" value="COMP"/>
          <group name="IRIS">
            <check id="iris.comp" title="Namespace COMP" objective="COMP exists">
              <requires check="docker.running"/>
              <run kind="objectscript" namespace="%SYS">##class(%SYS.Namespace).Exists("{{comp}}")</run>
            </check>
          </group>
          <group name="Docker">
            <check id="docker.running" title="Docker" timeout="5">
              <run kind="powershell" os="windows">docker/running.ps1</run>
              <run kind="shell" os="linux macos">docker/running.sh</run>
              <expect status="ok skip"/>
              <after check="docker.running" status="ok"/>
              <fix title="Start" kind="shell" confirm="false">fixes/consistem.sh start</fix>
            </check>
          </group>
        </doctor>"#;

    #[test]
    fn a_file_reads_into_checks_with_their_runtimes_ahead_of_them() {
        let spec = read(SAMPLE).expect("parses");
        let ids: Vec<&str> = spec.checks.iter().map(|c| c.id.as_str()).collect();
        // The runtimes first, then each check after what it requires - the
        // file lists COMP before the Docker check it depends on.
        // Git Bash is listed for the fix, though nothing runs in it to check.
        assert_eq!(
            ids,
            [
                "runtime.shell",
                "runtime.powershell",
                "runtime.iris",
                "docker.running",
                "iris.comp"
            ]
        );
        assert_eq!(spec.python.as_deref(), Some("py -3"));
        assert_eq!(spec.var("COMP"), Some("COMP"));
        assert_eq!(spec.dir, Path::new("/packs"));

        let docker = spec.check("docker.running").unwrap();
        assert_eq!(docker.timeout, Duration::from_secs(5));
        assert_eq!(docker.expect, [Status::Ok, Status::Skip]);
        assert_eq!(docker.command(Os::Windows).unwrap().kind, Kind::PowerShell);
        assert_eq!(docker.command(Os::Linux).unwrap().kind, Kind::Shell);
        let fix = docker.fix.as_ref().unwrap();
        assert_eq!(fix.title, "Start");
        assert!(!fix.confirm);
        assert_eq!(fix.after.len(), 1);

        let comp = spec.check("iris.comp").unwrap();
        assert_eq!(comp.requires[0].check, "runtime.iris");
        assert_eq!(
            comp.command(Os::Windows).unwrap().namespace.as_deref(),
            Some("%SYS")
        );
    }

    #[test]
    fn a_runtime_is_added_only_for_what_runs_on_this_system() {
        let spec = parse(SAMPLE, Path::new("doctor.xml"), Os::Linux).expect("parses");
        assert!(spec.check("runtime.shell").is_some());
        assert!(spec.check("runtime.powershell").is_none());
    }

    #[test]
    fn a_file_can_replace_a_runtime_with_its_own_check() {
        let spec = read(
            r#"<doctor><check id="runtime.python" title="Our Python">
                 <run kind="powershell">py.ps1</run></check>
               <check id="x"><run kind="python">x.py</run></check></doctor>"#,
        )
        .expect("parses");
        let python = spec.check("runtime.python").unwrap();
        assert_eq!(python.title, "Our Python");
        assert!(!python.builtin);
    }

    #[test]
    fn checks_that_require_each_other_are_refused_by_name() {
        let error = read(
            r#"<doctor>
                 <check id="a"><requires check="b"/><run kind="python">a.py</run></check>
                 <check id="b"><requires check="a"/><run kind="python">b.py</run></check>
               </doctor>"#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("loop"), "{error}");
        assert!(error.contains('a') && error.contains('b'), "{error}");
    }

    #[test]
    fn a_requirement_on_a_check_that_is_not_there_is_refused() {
        let error = read(
            r#"<doctor><check id="a"><requires check="nope"/><run kind="python">a.py</run></check></doctor>"#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("nope"), "{error}");
    }

    #[test]
    fn mistakes_in_the_file_are_errors_not_guesses() {
        for xml in [
            r#"<doctor><check id="a"><run kind="perl">a.pl</run></check></doctor>"#,
            r#"<doctor><check id="a"><run kind="python" os="beos">a.py</run></check></doctor>"#,
            r#"<doctor><check id="a"><expect status="fine"/><run kind="python">a.py</run></check></doctor>"#,
            r#"<doctor><check id="a"></check></doctor>"#,
            r#"<doctor><check id="a"><run kind="python">a.py</run></check><check id="a"><run kind="python">b.py</run></check></doctor>"#,
            r#"<macros/>"#,
        ] {
            assert!(read(xml).is_err(), "accepted {xml}");
        }
    }

    #[test]
    fn a_requirement_with_no_status_asks_for_what_its_check_calls_healthy() {
        let spec = read(SAMPLE).unwrap();
        let on_docker = Requirement {
            check: "docker.running".into(),
            status: None,
        };
        assert!(spec.satisfied(&on_docker, Status::Skip));
        assert!(!spec.satisfied(&on_docker, Status::Warn));
        let strict = Requirement {
            status: Some(vec![Status::Ok]),
            ..on_docker
        };
        assert!(!spec.satisfied(&strict, Status::Skip));
    }
}
