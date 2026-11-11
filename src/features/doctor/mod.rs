//! The Doctor: the health of the environment around the terminal, as a list
//! of checks the organisation writes - the namespaces, the product version,
//! the Docker images - each with a status, and a fix where one is known.
//!
//! Nothing here knows what is being checked. A checks file names scripts
//! (Python, PowerShell, shell, ObjectScript), says which needs which, and the
//! engine runs them in that order, off the UI thread, reading one line of JSON
//! from each. See [`spec`] for the file, [`run`] for the contract with a
//! script, and `docs/doctor.md` for both as a check's author sees them.
//!
//! A run is started by hand or by opening the page, never on a timer: an idle
//! terminal must cost nothing, and a check may take seconds.

pub mod run;
pub mod spec;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

use run::{Context, Outcome};
use spec::{Check, Spec, Status};

/// What the engine says back while it runs.
enum Event {
    Started(String),
    Finished(String, Outcome),
    Done,
}

/// What became of a fix, once the checks it promised were run again.
#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    pub check: String,
    pub worked: bool,
    /// The requirements that do not hold, as "title: status".
    pub failing: Vec<String>,
}

/// The Doctor's state: the file, the latest outcome of every check, and the
/// run in progress if there is one.
#[derive(Default)]
pub struct Doctor {
    /// The file loaded, or why it could not be.
    loaded: Option<(PathBuf, Result<Spec, String>)>,
    results: HashMap<String, Outcome>,
    running: HashSet<String>,
    events: Option<Receiver<Event>>,
    pub checked_at: Option<chrono::DateTime<chrono::Local>>,
    /// A fix opened in a tab, waiting to be verified.
    pub fixing: Option<String>,
    /// The fix being verified by the run in progress.
    verifying: Option<String>,
    pub verdict: Option<Verdict>,
}

impl Doctor {
    /// Reads `path` unless it is the file already loaded. Forgets the last
    /// results when the file changes: they were answers to other checks.
    pub fn load(&mut self, path: Option<&Path>) {
        let same = match (&self.loaded, path) {
            (Some((loaded, _)), Some(path)) => loaded == path,
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        self.reload(path);
    }

    /// Reads `path` again, whatever was loaded.
    pub fn reload(&mut self, path: Option<&Path>) {
        let previous = self.loaded.take();
        self.loaded = path.map(|p| (p.to_path_buf(), spec::load(p).map_err(|e| format!("{e:#}"))));
        let same_file = previous.map(|(p, _)| p) == self.loaded.as_ref().map(|(p, _)| p.clone());
        if !same_file {
            // A run of the old file is let go: its answers are about checks
            // that may share an id with the new file's and nothing else. The
            // thread stops at its next answer, finding no one to send it to.
            self.events = None;
            self.running.clear();
            self.verifying = None;
            self.results.clear();
            self.checked_at = None;
            self.fixing = None;
            self.verdict = None;
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.loaded.as_ref().map(|(p, _)| p.as_path())
    }

    pub fn spec(&self) -> Option<&Spec> {
        self.loaded.as_ref().and_then(|(_, s)| s.as_ref().ok())
    }

    pub fn load_error(&self) -> Option<&str> {
        self.loaded
            .as_ref()
            .and_then(|(_, s)| s.as_ref().err())
            .map(String::as_str)
    }

    pub fn is_running(&self) -> bool {
        self.events.is_some()
    }

    pub fn is_checking(&self, id: &str) -> bool {
        self.running.contains(id)
    }

    pub fn result(&self, id: &str) -> Option<&Outcome> {
        self.results.get(id)
    }

    /// Whether every check has been run at least once since the file loaded.
    pub fn has_run(&self) -> bool {
        self.checked_at.is_some() || self.is_running()
    }

    /// Runs every check, in order, on a thread of its own.
    pub fn run_all(&mut self, ctx: Context) {
        let Some(spec) = self.spec().cloned() else {
            return;
        };
        let ids = spec.checks.iter().map(|c| c.id.clone()).collect();
        self.start(spec, ids, ctx);
    }

    /// Runs one check again, and whatever requires it, after it.
    pub fn run_one(&mut self, id: &str, ctx: Context) {
        let Some(spec) = self.spec().cloned() else {
            return;
        };
        let ids = with_dependents(&spec, [id.to_string()].into());
        self.start(spec, ids, ctx);
    }

    /// Notes that `check`'s fix has been opened in a tab.
    pub fn fix_started(&mut self, check: &str) {
        self.fixing = Some(check.to_string());
        self.verdict = None;
    }

    /// Runs the checks `check`'s fix promised to put right, and then says
    /// whether they are.
    pub fn verify(&mut self, check: &str, ctx: Context) {
        let Some(spec) = self.spec().cloned() else {
            return;
        };
        let Some(c) = spec.check(check) else {
            return;
        };
        let promised: HashSet<String> = after(c).into_iter().map(|r| r.check).collect();
        let ids = with_dependents(&spec, promised);
        self.fixing = None;
        self.verifying = Some(check.to_string());
        self.start(spec, ids, ctx);
    }

    fn start(&mut self, spec: Spec, ids: HashSet<String>, ctx: Context) {
        if self.is_running() {
            return;
        }
        self.verdict = None;
        let (send, events) = channel();
        self.events = Some(events);
        // What the checks not being run came to last time: a requirement on
        // one of them is judged by that.
        let mut known = self.results.clone();
        for id in &ids {
            known.remove(id);
        }
        std::thread::Builder::new()
            .name("doctor".into())
            .spawn(move || {
                for check in spec.checks.iter().filter(|c| ids.contains(&c.id)) {
                    let outcome = match unmet(&spec, check, &known) {
                        Some(waiting) => Outcome::new(Status::Blocked, waiting),
                        None => {
                            let _ = send.send(Event::Started(check.id.clone()));
                            crate::pty::wake();
                            run::run(&spec, check, &ctx)
                        }
                    };
                    known.insert(check.id.clone(), outcome.clone());
                    if send
                        .send(Event::Finished(check.id.clone(), outcome))
                        .is_err()
                    {
                        return;
                    }
                    crate::pty::wake();
                }
                let _ = send.send(Event::Done);
                crate::pty::wake();
            })
            .ok();
    }

    /// A Doctor showing `results` - id, status, summary, detail - for the
    /// checks at `path`, as though it had just run them: the snapshot of the
    /// page, which must not run anything.
    #[cfg(test)]
    pub fn showing(path: &Path, results: &[(&str, Status, &str, &str)]) -> Doctor {
        let mut doctor = Doctor::default();
        doctor.load(Some(path));
        for (id, status, summary, detail) in results {
            let mut outcome = Outcome::new(*status, *summary);
            outcome.detail = detail.to_string();
            outcome.took = std::time::Duration::from_millis(840);
            doctor.results.insert(id.to_string(), outcome);
        }
        doctor.checked_at = Some(chrono::Local::now());
        doctor
    }

    /// Takes in whatever the run has said since the last frame. Returns true
    /// when anything changed.
    pub fn poll(&mut self) -> bool {
        let Some(events) = &self.events else {
            return false;
        };
        let mut changed = false;
        let mut done = false;
        while let Ok(event) = events.try_recv() {
            changed = true;
            match event {
                Event::Started(id) => {
                    self.running.insert(id);
                }
                Event::Finished(id, outcome) => {
                    self.running.remove(&id);
                    self.results.insert(id, outcome);
                }
                Event::Done => done = true,
            }
        }
        // A thread that died without saying so is done too.
        if done
            || matches!(
                events.try_recv(),
                Err(std::sync::mpsc::TryRecvError::Disconnected)
            )
        {
            self.events = None;
            self.running.clear();
            self.checked_at = Some(chrono::Local::now());
            if let Some(check) = self.verifying.take() {
                self.verdict = self.judge(&check);
            }
            changed = true;
        }
        changed
    }

    /// Whether the fix of `check` did what it promised, by the results now.
    fn judge(&self, check: &str) -> Option<Verdict> {
        let spec = self.spec()?;
        let c = spec.check(check)?;
        let failing: Vec<String> = after(c)
            .iter()
            .filter(|r| {
                !self
                    .results
                    .get(&r.check)
                    .is_some_and(|o| spec.satisfied(r, o.status))
            })
            .map(|r| {
                let title = spec
                    .check(&r.check)
                    .map_or(r.check.as_str(), |c| c.title.as_str());
                let status = self.results.get(&r.check).map_or("?", |o| o.status.key());
                format!("{title}: {status}")
            })
            .collect();
        Some(Verdict {
            check: check.to_string(),
            worked: failing.is_empty(),
            failing,
        })
    }

    /// The checks that are not healthy, not counting ones that do not apply,
    /// could not run for want of another, or are not mandatory.
    pub fn issues(&self) -> usize {
        let Some(spec) = self.spec() else {
            return 0;
        };
        spec.checks
            .iter()
            .filter(|c| c.mandatory)
            .filter(|c| {
                self.results
                    .get(&c.id)
                    .is_some_and(|o| is_issue(c, o.status))
            })
            .count()
    }

    /// The worst status among the results, for the summary's light.
    pub fn overall(&self) -> Option<Status> {
        let spec = self.spec()?;
        let rank = |s: Status| match s {
            Status::Error => 4,
            Status::Unknown => 3,
            Status::Warn => 2,
            Status::Blocked => 1,
            Status::Ok | Status::Skip => 0,
        };
        spec.checks
            .iter()
            .filter_map(|c| self.results.get(&c.id).map(|o| (c, o.status)))
            .map(|(c, s)| {
                if c.healthy(s) || !c.mandatory {
                    Status::Ok
                } else {
                    s
                }
            })
            .max_by_key(|s| rank(*s))
    }

    /// Every result as plain text, for pasting into a ticket or a chat.
    /// The user's home folder and name are taken out: a report is passed on.
    pub fn report(&self) -> String {
        let Some(spec) = self.spec() else {
            return String::new();
        };
        let mut out = format!(
            "consisTerm Doctor {}\nFile: {}\nSystem: {}\n",
            env!("CARGO_PKG_VERSION"),
            spec.path.display(),
            spec::Os::current().key()
        );
        if let Some(at) = self.checked_at {
            out.push_str(&format!("Checked: {}\n", at.format("%Y-%m-%d %H:%M")));
        }
        out.push_str(&format!("Issues: {}\n", self.issues()));
        for group in spec.groups() {
            out.push_str(&format!("\n[{group}]\n"));
            for check in spec.checks.iter().filter(|c| c.group == group) {
                match self.results.get(&check.id) {
                    None => out.push_str(&format!("  {:<8} {}\n", "-", check.title)),
                    Some(o) => {
                        out.push_str(&format!(
                            "  {:<8} {} - {}\n",
                            o.status.key().to_uppercase(),
                            check.title,
                            o.summary
                        ));
                        for line in o.detail.lines().filter(|l| !l.trim().is_empty()) {
                            out.push_str(&format!("           {line}\n"));
                        }
                    }
                }
            }
        }
        redact(&out)
    }
}

/// Whether `status` is a problem for `check`: not what it expects, and not a
/// check that simply does not apply or could not run.
pub fn is_issue(check: &Check, status: Status) -> bool {
    !check.healthy(status) && !matches!(status, Status::Skip | Status::Blocked)
}

/// What `check`'s fix promised: its `<after>`s, or the check itself.
fn after(check: &Check) -> Vec<spec::Requirement> {
    match &check.fix {
        Some(fix) if !fix.after.is_empty() => fix.after.clone(),
        _ => vec![spec::Requirement {
            check: check.id.clone(),
            status: None,
        }],
    }
}

/// The first requirement of `check` that does not hold, said as what it is
/// waiting on; `None` when it can run.
fn unmet(spec: &Spec, check: &Check, known: &HashMap<String, Outcome>) -> Option<String> {
    check.requires.iter().find_map(|r| {
        let holds = known
            .get(&r.check)
            .is_some_and(|o| spec.satisfied(r, o.status));
        (!holds).then(|| {
            let title = spec
                .check(&r.check)
                .map_or(r.check.as_str(), |c| c.title.as_str());
            crate::i18n::tr1("Waiting on {}", title)
        })
    })
}

/// `ids`, and every check that requires one of them, however indirectly.
fn with_dependents(spec: &Spec, mut ids: HashSet<String>) -> HashSet<String> {
    // The checks are in dependency order, so one pass reaches every dependent.
    for check in &spec.checks {
        if check.requires.iter().any(|r| ids.contains(&r.check)) {
            ids.insert(check.id.clone());
        }
    }
    ids
}

/// `text` without the user's home folder or user name in it.
fn redact(text: &str) -> String {
    let mut out = text.to_string();
    if let Some(home) = dirs::home_dir() {
        let home = home.display().to_string();
        if home.len() > 3 {
            for form in [home.clone(), home.replace('\\', "/")] {
                out = replace_any_case(&out, &form, "~");
            }
        }
    }
    for var in ["USERNAME", "USER"] {
        if let Ok(name) = std::env::var(var) {
            if name.len() > 2 {
                out = replace_any_case(&out, &name, "<user>");
            }
        }
    }
    out
}

/// `text` with every `what` replaced, whatever its case: Windows reads
/// `c:\users\x` and `C:\Users\X` as one folder, and a script may write
/// either. ASCII case only, so the lowered copy's byte offsets are the
/// original's.
fn replace_any_case(text: &str, what: &str, with: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let wanted = what.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(found) = lower[at..].find(&wanted) {
        out.push_str(&text[at..at + found]);
        out.push_str(with);
        at += found + wanted.len();
    }
    out.push_str(&text[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use spec::Os;

    fn spec(xml: &str) -> Spec {
        spec::parse(xml, Path::new("doctor.xml"), Os::current()).unwrap()
    }

    fn loaded(xml: &str) -> Doctor {
        Doctor {
            loaded: Some((PathBuf::from("doctor.xml"), Ok(spec(xml)))),
            ..Doctor::default()
        }
    }

    fn outcome(status: Status) -> Outcome {
        Outcome::new(status, "")
    }

    const CHAIN: &str = r#"<doctor>
        <check id="docker"><run kind="objectscript">1</run></check>
        <check id="images"><requires check="docker"/><run kind="objectscript">1</run>
          <fix kind="objectscript">Fix^X</fix>
          <after check="images"/><after check="docker" status="ok"/></check>
        <check id="other"><run kind="objectscript">1</run></check>
      </doctor>"#;

    #[test]
    fn a_check_waits_on_what_it_requires_and_says_which() {
        let spec = spec(CHAIN);
        let images = spec.check("images").unwrap();
        let mut known = HashMap::new();
        known.insert("runtime.iris".into(), outcome(Status::Ok));
        known.insert("docker".into(), outcome(Status::Error));
        assert_eq!(
            unmet(&spec, images, &known).as_deref(),
            Some("Waiting on docker")
        );
        known.insert("docker".into(), outcome(Status::Ok));
        assert_eq!(unmet(&spec, images, &known), None);
    }

    #[test]
    fn running_one_check_again_runs_what_depends_on_it_too() {
        let spec = spec(CHAIN);
        let ids = with_dependents(&spec, ["docker".to_string()].into());
        assert_eq!(ids, ["docker".to_string(), "images".to_string()].into());
        let all = with_dependents(&spec, ["runtime.iris".to_string()].into());
        assert_eq!(all.len(), spec.checks.len());
    }

    #[test]
    fn a_fix_worked_only_when_everything_it_promised_holds() {
        let mut doctor = loaded(CHAIN);
        doctor.results.insert("images".into(), outcome(Status::Ok));
        doctor
            .results
            .insert("docker".into(), outcome(Status::Warn));
        let verdict = doctor.judge("images").unwrap();
        assert!(!verdict.worked);
        assert_eq!(verdict.failing, ["docker: warn"]);
        doctor.results.insert("docker".into(), outcome(Status::Ok));
        assert!(doctor.judge("images").unwrap().worked);
    }

    #[test]
    fn issues_leave_out_what_does_not_apply_or_could_not_run() {
        let mut doctor = loaded(CHAIN);
        doctor
            .results
            .insert("runtime.iris".into(), outcome(Status::Ok));
        doctor
            .results
            .insert("docker".into(), outcome(Status::Error));
        doctor
            .results
            .insert("images".into(), outcome(Status::Blocked));
        doctor.results.insert("other".into(), outcome(Status::Skip));
        assert_eq!(doctor.issues(), 1);
        assert_eq!(doctor.overall(), Some(Status::Error));
        let report = doctor.report();
        assert!(report.contains("ERROR    docker"), "{report}");
        assert!(report.contains("BLOCKED  images"), "{report}");
    }

    #[test]
    fn an_optional_check_in_trouble_leaves_the_summary_green_but_keeps_its_fix() {
        let mut doctor = loaded(
            r#"<doctor>
              <check id="req"><run kind="objectscript">1</run></check>
              <check id="opt" mandatory="false"><run kind="objectscript">1</run>
                <fix kind="objectscript">Fix^X</fix></check>
            </doctor>"#,
        );
        doctor
            .results
            .insert("runtime.iris".into(), outcome(Status::Ok));
        doctor.results.insert("req".into(), outcome(Status::Ok));
        for status in [Status::Warn, Status::Error] {
            doctor.results.insert("opt".into(), outcome(status));
            assert_eq!(doctor.overall(), Some(Status::Ok));
            assert_eq!(doctor.issues(), 0);
            let opt = doctor.spec().unwrap().check("opt").unwrap();
            // The page offers a fix to whatever is_issue calls a problem.
            assert!(is_issue(opt, status));
        }
        doctor.results.insert("req".into(), outcome(Status::Warn));
        assert_eq!(doctor.overall(), Some(Status::Warn));
    }

    #[test]
    fn a_mandatory_that_is_neither_true_nor_false_is_refused() {
        let xml = r#"<doctor><check id="c" mandatory="maybe"><run kind="objectscript">1</run></check></doctor>"#;
        let error = spec::parse(xml, Path::new("doctor.xml"), Os::current()).unwrap_err();
        assert!(error.to_string().contains("mandatory"), "{error}");
    }

    #[test]
    fn a_report_does_not_carry_the_home_folder() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let text = format!("at {}\\x", home.display());
        assert!(!redact(&text).contains(&home.display().to_string()));
    }

    #[test]
    fn a_whole_run_comes_back_through_poll_in_order() {
        // A runtime of no IRIS at all: every ObjectScript check is then
        // blocked behind it, which exercises the run without touching one.
        let mut doctor = loaded(CHAIN);
        let ctx = Context {
            os: Os::current(),
            python: vec!["python".into()],
            instance: None,
            namespace: "USER".into(),
        };
        doctor.run_all(ctx);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while doctor.is_running() && std::time::Instant::now() < deadline {
            doctor.poll();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!doctor.is_running());
        assert_eq!(doctor.result("runtime.iris").unwrap().status, Status::Error);
        for id in ["docker", "images", "other"] {
            assert_eq!(doctor.result(id).unwrap().status, Status::Blocked, "{id}");
        }
        assert!(doctor.checked_at.is_some());
    }
}

#[cfg(test)]
mod adversarial;

#[cfg(test)]
mod adversarial_kinds;
