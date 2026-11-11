//! The engine attacked: hostile scripts, hostile files, and a run raced by
//! the page. Each test is something a checks file or a machine can do to the
//! Doctor, and what must hold when it does - above all that a run always ends
//! and a broken check is never green.
//!
//! The script tests need Python, as the checks do; without it they say so
//! and pass.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::run::{self, Context, Outcome};
use super::spec::{self, Os, Spec, Status};
use super::Doctor;

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nit-doctor-adv-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn python_here() -> bool {
    let python = if cfg!(windows) { "python" } else { "python3" };
    let found = std::process::Command::new(python)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !found {
        eprintln!("no {python}; skipping");
    }
    found
}

/// Runs one Python check whose script is `body`, with `timeout` seconds.
fn script(tag: &str, body: &str, timeout: u32) -> Option<Outcome> {
    if !python_here() {
        return None;
    }
    let dir = tempdir(tag);
    std::fs::write(dir.join("c.py"), body).unwrap();
    let path = dir.join("doctor.xml");
    std::fs::write(
        &path,
        format!(r#"<doctor><check id="c" timeout="{timeout}"><run kind="python">c.py</run></check></doctor>"#),
    )
    .unwrap();
    let spec = spec::load(&path).unwrap();
    let ctx = Context::new(&spec, None);
    let outcome = run::run(&spec, spec.check("c").unwrap(), &ctx);
    let _ = std::fs::remove_dir_all(&dir);
    Some(outcome)
}

fn parse(xml: &str) -> anyhow::Result<Spec> {
    spec::parse(xml, Path::new("doctor.xml"), Os::current())
}

// ---------------------------------------------------------------------------
// Hostile scripts
// ---------------------------------------------------------------------------

#[test]
fn megabytes_of_output_neither_stall_the_run_nor_hide_the_answer() {
    let Some(o) = script(
        "huge",
        "import sys\nfor _ in range(60000): print('x' * 80)\nprint('{\"status\":\"ok\",\"summary\":\"done\"}')\n",
        30,
    ) else {
        return;
    };
    assert_eq!((o.status, o.summary.as_str()), (Status::Ok, "done"));
}

#[test]
fn a_background_process_holding_the_output_open_does_not_hang_the_run() {
    // `docker compose up -d`, a daemon started by a check: the check is done,
    // but its child still holds the pipe, and a reader waiting for the pipe
    // to close would wait as long as the child lives.
    let started = Instant::now();
    let Some(o) = script(
        "grandchild",
        "import subprocess, sys\nsubprocess.Popen([sys.executable, '-c', 'import time; time.sleep(25)'])\nprint('{\"status\":\"ok\"}', flush=True)\n",
        30,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Ok, "{o:?}");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_check_that_never_ends_is_stopped_at_its_timeout_and_is_unknown() {
    let started = Instant::now();
    let Some(o) = script("busy", "while True: pass\n", 1) else {
        return;
    };
    assert_eq!(o.status, Status::Unknown);
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn bytes_that_are_not_utf8_do_not_lose_the_answer() {
    let Some(o) = script(
        "bytes",
        "import sys\nsys.stdout.buffer.write(b'\\xf5\\xff\\xfe garbage\\n')\nsys.stdout.buffer.write(b'{\"status\":\"warn\",\"summary\":\"n\\xc3\\xa3o\"}\\n')\n",
        10,
    ) else {
        return;
    };
    assert_eq!((o.status, o.summary.as_str()), (Status::Warn, "não"));
}

#[test]
fn a_byte_order_mark_in_front_of_the_answer_is_not_an_unknown() {
    // What Windows PowerShell writes ahead of UTF-8 output.
    let o = run::answer("\u{feff}{\"status\":\"ok\"}\r\n").expect("an answer");
    assert_eq!(o.status, Status::Ok);
}

#[test]
fn an_answer_on_standard_error_only_is_unknown_and_shown() {
    let Some(o) = script(
        "stderr",
        "import sys\nprint('{\"status\":\"ok\"}', file=sys.stderr)\n",
        10,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Unknown);
    assert!(o.detail.contains("status"), "{o:?}");
}

#[test]
fn a_script_that_answers_and_then_crashes_keeps_its_answer() {
    let Some(o) = script(
        "crash-after",
        "print('{\"status\":\"error\",\"summary\":\"bad\"}')\nraise SystemExit(3)\n",
        10,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Error);
}

#[test]
fn a_script_that_is_not_there_is_never_green() {
    if !python_here() {
        return;
    }
    let spec = parse(
        r#"<doctor><check id="c"><run kind="python">nowhere/missing.py</run></check></doctor>"#,
    )
    .unwrap();
    let o = run::run(&spec, spec.check("c").unwrap(), &Context::new(&spec, None));
    assert_eq!(o.status, Status::Unknown, "{o:?}");
}

#[test]
fn a_script_that_waits_for_input_gets_none_and_goes_on() {
    let Some(o) = script(
        "stdin",
        "try:\n    input()\nexcept EOFError:\n    pass\nprint('{\"status\":\"ok\"}')\n",
        10,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Ok);
}

#[test]
fn a_status_written_loosely_is_still_read() {
    for said in ["OK", " ok ", "Warn"] {
        let o = run::answer(&format!("{{\"status\":\"{said}\"}}")).unwrap();
        assert_ne!(o.status, Status::Unknown, "{said:?}");
    }
}

#[test]
fn a_last_line_of_broken_json_falls_back_to_the_last_good_one() {
    let o = run::answer("{\"status\":\"warn\"}\n{\"status\": \"ok\", \"sum").unwrap();
    assert_eq!(o.status, Status::Warn);
}

#[test]
fn fields_of_the_wrong_type_do_not_panic_and_status_must_be_text() {
    let o = run::answer(r#"{"status":"ok","summary":42,"detail":{"a":[1,null]}}"#).unwrap();
    assert_eq!((o.status, o.summary.as_str()), (Status::Ok, "42"));
    assert!(run::answer(r#"{"status":1}"#).is_none());
    assert!(run::answer(r#"{"status":null}"#).is_none());
}

// ---------------------------------------------------------------------------
// Hostile files
// ---------------------------------------------------------------------------

#[test]
fn a_check_that_requires_itself_is_a_loop() {
    let e = parse(r#"<doctor><check id="a"><requires check="a"/><run kind="python">a.py</run></check></doctor>"#)
        .unwrap_err()
        .to_string();
    assert!(e.contains("loop"), "{e}");
}

#[test]
fn a_timeout_of_nothing_or_nonsense_is_refused() {
    for timeout in ["0", "-1", "ten", "99999999999999999999"] {
        let xml = format!(
            r#"<doctor><check id="a" timeout="{timeout}"><run kind="python">a.py</run></check></doctor>"#
        );
        assert!(parse(&xml).is_err(), "accepted timeout {timeout:?}");
    }
}

#[test]
fn an_empty_id_is_refused() {
    for id in ["", "   "] {
        let xml =
            format!(r#"<doctor><check id="{id}"><run kind="python">a.py</run></check></doctor>"#);
        assert!(parse(&xml).is_err(), "accepted id {id:?}");
    }
}

#[test]
fn a_long_chain_of_requirements_loads_quickly_and_in_order() {
    let mut xml = String::from("<doctor>");
    // Declared last-first, so every one is out of order.
    for i in (0..2000).rev() {
        let requires = if i > 0 {
            format!(r#"<requires check="c{}"/>"#, i - 1)
        } else {
            String::new()
        };
        xml.push_str(&format!(
            r#"<check id="c{i}">{requires}<run kind="python">x.py</run></check>"#
        ));
    }
    xml.push_str("</doctor>");
    let started = Instant::now();
    let spec = parse(&xml).unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "took {:?}",
        started.elapsed()
    );
    let at = |id: &str| spec.checks.iter().position(|c| c.id == id).unwrap();
    assert!(at("c0") < at("c1") && at("c1998") < at("c1999"));
}

#[test]
fn a_document_type_with_entities_is_not_expanded() {
    let xml = r#"<?xml version="1.0"?>
        <!DOCTYPE doctor [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;">]>
        <doctor><check id="a" title="&b;"><run kind="python">a.py</run></check></doctor>"#;
    let started = Instant::now();
    let parsed = parse(xml);
    assert!(started.elapsed() < Duration::from_secs(2));
    if let Ok(spec) = parsed {
        assert!(spec.check("a").unwrap().title.len() < 100);
    }
}

// ---------------------------------------------------------------------------
// Races with the page
// ---------------------------------------------------------------------------

fn wait(doctor: &mut Doctor) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while doctor.is_running() && Instant::now() < deadline {
        doctor.poll();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!doctor.is_running(), "the run never ended");
}

fn file(dir: &Path, name: &str, xml: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, xml).unwrap();
    path
}

fn no_iris() -> Context {
    Context {
        os: Os::current(),
        python: vec!["python".into()],
        instance: None,
        namespace: "USER".into(),
    }
}

#[test]
fn a_file_changed_during_a_run_never_gets_the_old_files_results() {
    if !python_here() {
        return;
    }
    let dir = tempdir("switch");
    std::fs::write(
        dir.join("slow.py"),
        "import time\ntime.sleep(2)\nprint('{\"status\":\"error\"}')\n",
    )
    .unwrap();
    let old = file(
        &dir,
        "old.xml",
        r#"<doctor><check id="shared"><run kind="python">slow.py</run></check></doctor>"#,
    );
    let new = file(
        &dir,
        "new.xml",
        r#"<doctor><check id="shared"><run kind="python">slow.py</run></check></doctor>"#,
    );
    let mut doctor = Doctor::default();
    doctor.load(Some(&old));
    let ctx = Context::new(doctor.spec().unwrap(), None);
    doctor.run_all(ctx);
    doctor.load(Some(&new));
    assert!(!doctor.is_running(), "still running the old file");
    std::thread::sleep(Duration::from_secs(4));
    doctor.poll();
    assert!(
        doctor.result("shared").is_none(),
        "the old file's answer landed on the new one"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_second_run_asked_for_during_the_first_is_ignored_and_the_first_completes() {
    let dir = tempdir("twice");
    let path = file(
        &dir,
        "d.xml",
        r#"<doctor><check id="a"><run kind="objectscript">1</run></check><check id="b"><run kind="objectscript">1</run></check></doctor>"#,
    );
    let mut doctor = Doctor::default();
    doctor.load(Some(&path));
    doctor.run_all(no_iris());
    doctor.run_all(no_iris());
    doctor.run_one("a", no_iris());
    wait(&mut doctor);
    for id in ["runtime.iris", "a", "b"] {
        assert!(doctor.result(id).is_some(), "{id} has no result");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn verifying_a_fix_whose_check_has_gone_from_the_file_does_nothing_harmful() {
    let dir = tempdir("gone");
    let path = file(
        &dir,
        "d.xml",
        r#"<doctor><check id="a"><run kind="objectscript">1</run><fix kind="objectscript">X^Y</fix></check></doctor>"#,
    );
    let mut doctor = Doctor::default();
    doctor.load(Some(&path));
    doctor.fix_started("a");
    file(
        &dir,
        "d.xml",
        r#"<doctor><check id="b"><run kind="objectscript">1</run></check></doctor>"#,
    );
    doctor.reload(Some(&path));
    doctor.verify("a", no_iris());
    wait(&mut doctor);
    assert!(doctor.verdict.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Second round
// ---------------------------------------------------------------------------

/// Runs `xml`'s check `c` from a pack in `dir`, with `files` written into it.
fn pack(dir: &Path, xml: &str, files: &[(&str, &str)]) -> Option<Outcome> {
    if !python_here() {
        return None;
    }
    for (name, body) in files {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
    let path = file(dir, "doctor.xml", xml);
    let spec = spec::load(&path).unwrap();
    let outcome = run::run(&spec, spec.check("c").unwrap(), &Context::new(&spec, None));
    Some(outcome)
}

const ECHO_ARGS: &str =
    "import sys, json\nprint(json.dumps({'status': 'ok', 'summary': '|'.join(sys.argv[1:])}))\n";

#[test]
fn a_script_whose_path_has_spaces_runs_when_quoted() {
    let dir = tempdir("spaces");
    let Some(o) = pack(
        &dir,
        r#"<doctor><check id="c"><run kind="python">"my scripts/a check.py" one "two words"</run></check></doctor>"#,
        &[("my scripts/a check.py", ECHO_ARGS)],
    ) else {
        return;
    };
    assert_eq!(
        (o.status, o.summary.as_str()),
        (Status::Ok, "one|two words"),
        "{o:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pack_in_a_folder_with_accents_and_spaces_runs() {
    let dir = tempdir("acentos").join("Configuração do time");
    std::fs::create_dir_all(&dir).unwrap();
    let Some(o) = pack(
        &dir,
        r#"<doctor><check id="c"><run kind="python">c.py ação</run></check></doctor>"#,
        &[("c.py", ECHO_ARGS)],
    ) else {
        return;
    };
    assert_eq!(
        (o.status, o.summary.as_str()),
        (Status::Ok, "ação"),
        "{o:?}"
    );
}

#[test]
fn a_variable_naming_another_is_put_in_as_written_not_expanded_forever() {
    let spec = parse(
        r#"<doctor><var name="a" value="{{b}}"/><var name="b" value="{{a}}"/>
             <check id="c" title="x {{a}}"><run kind="python">c.py</run></check></doctor>"#,
    )
    .unwrap();
    assert_eq!(spec.check("c").unwrap().title, "x {{b}}");
}

#[test]
fn an_unclosed_quote_in_a_command_does_not_panic() {
    assert_eq!(
        run::split_args(r#"a.py "never closed"#),
        ["a.py", "never closed"]
    );
    assert_eq!(run::split_args(r#"'"#), [""]);
    assert!(run::split_args("   ").is_empty());
}

#[test]
fn the_marker_inside_an_objectscript_answer_does_not_cut_it() {
    let mark = run::MARK;
    let out = format!(
        "USER>write !,\"{mark}\",x,!\n{mark}{{\"status\":\"ok\",\"summary\":\"has {mark} in it\"}}\nUSER>halt\n"
    );
    let outcome = run::marked(&out).and_then(run::answer).expect("an answer");
    assert_eq!(outcome.status, Status::Ok);
    assert!(outcome.summary.contains(mark), "{outcome:?}");
}

#[test]
fn the_report_hides_the_home_folder_whatever_its_case() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let home = home.display().to_string();
    let slash = |s: &str| s.replace('\\', "/");
    // Windows paths are not case-sensitive: a script may write the folder in
    // any case, and it is still the home folder.
    if !cfg!(windows) {
        return;
    }
    for written in [
        home.to_lowercase(),
        home.to_uppercase(),
        slash(&home).to_lowercase(),
    ] {
        let out = super::redact(&format!("at {written}\\x")).to_lowercase();
        assert!(
            !out.contains(&home.to_lowercase()) && !out.contains(&slash(&home).to_lowercase()),
            "{out}"
        );
    }
}

#[test]
fn a_large_input_to_a_program_that_never_reads_it_does_not_outlive_the_timeout() {
    if !python_here() {
        return;
    }
    let mut process = std::process::Command::new(if cfg!(windows) { "python" } else { "python3" });
    process.args(["-c", "import time; time.sleep(30)"]);
    let started = Instant::now();
    let result = run::execute(process, Some("x".repeat(4 << 20)), Duration::from_secs(2));
    assert!(matches!(result, Err(run::Failure::Timeout(_))));
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn unsetting_the_file_during_a_run_stops_it_and_forgets_everything() {
    if !python_here() {
        return;
    }
    let dir = tempdir("unset");
    std::fs::write(
        dir.join("slow.py"),
        "import time\ntime.sleep(2)\nprint('{\"status\":\"ok\"}')\n",
    )
    .unwrap();
    let path = file(
        &dir,
        "d.xml",
        r#"<doctor><check id="c"><run kind="python">slow.py</run></check></doctor>"#,
    );
    let mut doctor = Doctor::default();
    doctor.load(Some(&path));
    doctor.run_all(Context::new(doctor.spec().unwrap(), None));
    doctor.load(None);
    assert!(!doctor.is_running());
    std::thread::sleep(Duration::from_secs(3));
    doctor.poll();
    assert!(doctor.result("c").is_none() && doctor.spec().is_none() && doctor.overall().is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn checking_again_a_check_that_is_not_there_ends_cleanly() {
    let dir = tempdir("nothere");
    let path = file(
        &dir,
        "d.xml",
        r#"<doctor><check id="a"><run kind="objectscript">1</run></check></doctor>"#,
    );
    let mut doctor = Doctor::default();
    doctor.load(Some(&path));
    doctor.run_one("no-such-check", no_iris());
    wait(&mut doctor);
    assert!(doctor.result("a").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nul_and_control_bytes_in_the_output_do_not_lose_the_answer() {
    let Some(o) = script(
        "nul",
        "import sys\nsys.stdout.write('\\x00\\x1b[31mred\\x07\\r\\n')\nsys.stdout.write('{\"status\":\"warn\",\"summary\":\"a\\\\u0000b\"}\\n')\n",
        10,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Warn, "{o:?}");
}

/// The self-test pack, `examples/doctor-selftest`: every runtime doing what a
/// real check or fix does - files, folders, a line edited, git pull,
/// privileges. Ignored: it starts some forty processes. Without
/// `CONSISTERM_SELFTEST_IRIS=1` the ObjectScript half stays blocked rather
/// than open a session, as a test must not log in to an instance unasked.
#[test]
#[ignore]
fn selftest_pack() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/doctor-selftest/doctor.xml");
    let mut doctor = Doctor::default();
    doctor.load(Some(&path));
    assert!(doctor.load_error().is_none(), "{:?}", doctor.load_error());
    let instance = std::env::var("CONSISTERM_SELFTEST_IRIS").is_ok().then(|| {
        let launcher = crate::pty::launcher::launcher();
        crate::pty::launcher::instances(&*launcher)
            .first()
            .map(|i| i.name.clone())
    });
    let ctx = Context::new(doctor.spec().unwrap(), instance.flatten());
    let started = Instant::now();
    doctor.run_all(ctx);
    wait(&mut doctor);
    eprintln!(
        "--- {:.1} s\n{}",
        started.elapsed().as_secs_f32(),
        doctor.report()
    );
    let spec = doctor.spec().unwrap().clone();
    for check in spec.checks.iter().filter(|c| !c.builtin) {
        let o = doctor.result(&check.id).unwrap();
        // Blocked is a runtime that is not here (no pwsh, no IRIS): the
        // runtime's own row says why. Anything else must be healthy - bar the
        // check whose point is to warn when not elevated.
        let fine = check.healthy(o.status)
            || o.status == Status::Blocked
            || (check.id == "ps.elevation" && o.status == Status::Warn);
        assert!(fine, "{}: {:?} {}", check.id, o.status, o.summary);
    }
}

#[test]
fn a_console_code_page_line_is_read_as_one_and_a_utf8_line_as_utf8() {
    // "operações" as Windows PowerShell writes it to a pipe (CP850), then a
    // line that is UTF-8 already.
    let bytes = b"opera\x87\xe4es\n{\"status\":\"ok\",\"summary\":\"n\xc3\xa3o\"}\n";
    let text = run::decode(bytes);
    if cfg!(windows) {
        assert!(text.starts_with("operações\n"), "{text:?}");
    }
    assert!(text.ends_with("\"não\"}\n"), "{text:?}");
}
