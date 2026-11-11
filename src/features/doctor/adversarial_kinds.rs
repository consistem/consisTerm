//! Ten attacks on each runtime: what a Python, shell, PowerShell or
//! ObjectScript check can do wrong in its own language, and what the Doctor
//! must make of it - a run that ends, and nothing green that is not.
//!
//! A runtime that is not on the machine skips its ten. The ObjectScript ten
//! open a session on the first local instance, so they run only with
//! `CONSISTERM_SELFTEST_IRIS=1`; they touch nothing that persists.

use std::time::{Duration, Instant};

use super::run::{self, Context, Outcome};
use super::spec::{self, Kind, Status};

/// Runs a check of `kind` whose script is `body` (none for ObjectScript, whose
/// `target` is the expression), or `None` when the runtime is not here.
fn attack(
    tag: &str,
    kind: Kind,
    body: Option<&[u8]>,
    target: &str,
    timeout: u32,
) -> Option<Outcome> {
    let dir = std::env::temp_dir().join(format!("nit-doctor-kind-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    if let Some(body) = body {
        let name = target.split_whitespace().next().unwrap();
        std::fs::write(dir.join(name), body).unwrap();
    }
    let path = dir.join("doctor.xml");
    std::fs::write(
        &path,
        format!(
            r#"<doctor><check id="c" timeout="{timeout}"><run kind="{}"><![CDATA[{target}]]></run></check></doctor>"#,
            kind.key()
        ),
    )
    .unwrap();
    let spec = spec::load(&path).unwrap();
    let instance = (kind == Kind::ObjectScript)
        .then(|| std::env::var("CONSISTERM_SELFTEST_IRIS").ok())
        .flatten()
        .and_then(|_| {
            let launcher = crate::pty::launcher::launcher();
            crate::pty::launcher::instances(&*launcher)
                .first()
                .map(|i| i.name.clone())
        });
    if kind == Kind::ObjectScript && instance.is_none() {
        eprintln!("ObjectScript attacks need CONSISTERM_SELFTEST_IRIS=1; skipping");
        return None;
    }
    let ctx = Context::new(&spec, instance);
    let runtime = run::run(&spec, spec.check(kind.runtime()).unwrap(), &ctx);
    if runtime.status != Status::Ok {
        eprintln!("{} not here ({}); skipping", kind.key(), runtime.summary);
        return None;
    }
    let outcome = run::run(&spec, spec.check("c").unwrap(), &ctx);
    let _ = std::fs::remove_dir_all(&dir);
    Some(outcome)
}

fn py(tag: &str, body: &str, timeout: u32) -> Option<Outcome> {
    attack(tag, Kind::Python, Some(body.as_bytes()), "c.py", timeout)
}

fn sh(tag: &str, body: &str, timeout: u32) -> Option<Outcome> {
    attack(tag, Kind::Shell, Some(body.as_bytes()), "c.sh", timeout)
}

fn ps(tag: &str, body: &str, timeout: u32) -> Option<Outcome> {
    // With a BOM, as a .ps1 with accents has to be for Windows PowerShell.
    let mut bytes = b"\xef\xbb\xbf".to_vec();
    bytes.extend_from_slice(body.as_bytes());
    attack(tag, Kind::PowerShell, Some(&bytes), "c.ps1", timeout)
}

fn cos(tag: &str, expression: &str, timeout: u32) -> Option<Outcome> {
    attack(tag, Kind::ObjectScript, None, expression, timeout)
}

/// Unknown or worse - never a green for what could not say.
fn never_green(o: &Outcome) {
    assert!(
        matches!(o.status, Status::Unknown | Status::Error),
        "came back {:?}: {} / {}",
        o.status,
        o.summary,
        o.detail
    );
}

fn quick(started: Instant, limit: u64) {
    assert!(
        started.elapsed() < Duration::from_secs(limit),
        "took {:?}",
        started.elapsed()
    );
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------

#[test]
fn py_an_answer_with_no_newline_after_it_is_read() {
    let Some(o) = py(
        "py-nonl",
        "import sys\nsys.stdout.write('{\"status\":\"ok\"}')\n",
        10,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Ok);
}

#[test]
fn py_pretty_printed_json_is_not_an_answer() {
    let Some(o) = py(
        "py-pretty",
        "import json\nprint(json.dumps({'status': 'ok'}, indent=2))\n",
        10,
    ) else {
        return;
    };
    never_green(&o);
}

#[test]
fn py_a_silent_exit_is_unknown() {
    let Some(o) = py("py-silent", "import sys\nsys.exit(0)\n", 10) else {
        return;
    };
    never_green(&o);
}

#[test]
fn py_a_syntax_error_is_unknown_with_the_traceback_kept() {
    let Some(o) = py("py-syntax", "def broken(:\n    pass\n", 10) else {
        return;
    };
    never_green(&o);
    assert!(o.detail.contains("SyntaxError"), "{}", o.detail);
}

#[test]
fn py_an_answer_then_a_hang_is_stopped_and_not_green() {
    let started = Instant::now();
    let Some(o) = py(
        "py-hang",
        "print('{\"status\":\"ok\"}', flush=True)\nwhile True: pass\n",
        2,
    ) else {
        return;
    };
    never_green(&o);
    quick(started, 10);
}

#[test]
fn py_emoji_and_accents_in_the_answer_come_through() {
    let Some(o) = py(
        "py-emoji",
        "import json\nprint(json.dumps({'status': 'warn', 'summary': 'ação 🐳 ✓'}))\n",
        10,
    ) else {
        return;
    };
    assert_eq!((o.status, o.summary.as_str()), (Status::Warn, "ação 🐳 ✓"));
}

#[test]
fn py_fifty_child_processes_do_not_stall_the_answer() {
    let started = Instant::now();
    let Some(o) = py(
        "py-children",
        "import subprocess, sys\nfor _ in range(50): subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(5)'])\nprint('{\"status\":\"ok\"}', flush=True)\n",
        20,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Ok, "{o:?}");
    quick(started, 15);
}

#[test]
fn py_a_megabyte_on_stderr_does_not_block_the_answer_on_stdout() {
    let Some(o) = py(
        "py-stderr",
        "import sys\nsys.stderr.write('e' * (1 << 20))\nprint('{\"status\":\"ok\"}')\n",
        15,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Ok);
}

#[test]
fn py_the_check_is_told_where_it_is_and_what_it_is() {
    let Some(o) = py(
        "py-env",
        "import os, json\nprint(json.dumps({'status': 'ok', 'summary': os.environ.get('CONSISTERM_CHECK', '') + '|' + os.environ.get('CONSISTERM_OS', '')}))\n",
        10,
    ) else {
        return;
    };
    assert_eq!(o.summary, format!("c|{}", spec::Os::current().key()));
}

#[test]
fn py_characters_a_windows_shell_would_read_arrive_as_written() {
    let dir_args = r#"c.py "a&b|c<d>e^f" %PATH% "say ""hi"""#;
    let Some(o) = attack(
        "py-args",
        Kind::Python,
        Some(b"import sys, json\nprint(json.dumps({'status': 'ok', 'summary': '|'.join(sys.argv[1:])}))\n"),
        dir_args,
        10,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Ok, "{o:?}");
    assert!(
        o.summary.starts_with("a&b|c<d>e^f|%PATH%|"),
        "{}",
        o.summary
    );
}

// ---------------------------------------------------------------------------
// Shell
// ---------------------------------------------------------------------------

#[test]
fn sh_a_script_saved_with_crlf_is_read_not_mistaken() {
    let Some(o) = sh(
        "sh-crlf",
        "#!/usr/bin/env bash\r\necho '{\"status\":\"warn\",\"summary\":\"crlf\"}'\r\n",
        10,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Warn, "{o:?}");
}

#[test]
fn sh_a_silent_failure_is_unknown() {
    let Some(o) = sh("sh-exit", "exit 1\n", 10) else {
        return;
    };
    never_green(&o);
}

#[test]
fn sh_a_job_left_in_the_background_does_not_hold_the_run() {
    let started = Instant::now();
    let Some(o) = sh("sh-bg", "sleep 30 &\necho '{\"status\":\"ok\"}'\n", 40) else {
        return;
    };
    assert_eq!(o.status, Status::Ok);
    quick(started, 10);
}

#[test]
fn sh_an_unset_variable_under_set_u_is_unknown() {
    let Some(o) = sh(
        "sh-unset",
        "set -u\necho \"$NOT_THERE\"\necho '{\"status\":\"ok\"}'\n",
        10,
    ) else {
        return;
    };
    never_green(&o);
}

#[test]
fn sh_a_read_with_no_input_goes_on() {
    let Some(o) = sh("sh-read", "read -r line\necho '{\"status\":\"ok\"}'\n", 10) else {
        return;
    };
    assert_eq!(o.status, Status::Ok);
}

#[test]
fn sh_five_megabytes_from_yes_then_the_answer() {
    let Some(o) = sh(
        "sh-yes",
        "yes | head -c 5000000\necho\necho '{\"status\":\"ok\"}'\n",
        30,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Ok);
}

#[test]
fn sh_printf_with_no_newline_is_read() {
    let Some(o) = sh("sh-printf", "printf '%s' '{\"status\":\"ok\"}'\n", 10) else {
        return;
    };
    assert_eq!(o.status, Status::Ok);
}

#[test]
fn sh_an_answer_from_an_exit_trap_counts() {
    let Some(o) = sh(
        "sh-trap",
        "trap 'echo {\\\"status\\\":\\\"error\\\"}' EXIT\nfalse\n",
        10,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Error, "{o:?}");
}

#[test]
fn sh_a_star_in_the_arguments_is_not_expanded() {
    let Some(o) = attack(
        "sh-glob",
        Kind::Shell,
        Some(b"printf '{\"status\":\"ok\",\"summary\":\"%s\"}\\n' \"$1\"\n"),
        "c.sh *",
        10,
    ) else {
        return;
    };
    assert_eq!(o.summary, "*", "{o:?}");
}

#[test]
fn sh_a_script_that_kills_itself_is_unknown() {
    let Some(o) = sh("sh-kill", "kill -9 $$\necho '{\"status\":\"ok\"}'\n", 10) else {
        return;
    };
    never_green(&o);
}

// ---------------------------------------------------------------------------
// PowerShell
// ---------------------------------------------------------------------------

#[test]
fn ps_a_thrown_exception_is_unknown_with_its_message() {
    let Some(o) = ps("ps-throw", "throw 'disco cheio'\n", 20) else {
        return;
    };
    never_green(&o);
    assert!(o.detail.contains("disco cheio"), "{}", o.detail);
}

#[test]
fn ps_a_non_terminating_error_then_an_answer_keeps_the_answer() {
    let Some(o) = ps(
        "ps-error",
        "Write-Error 'aviso' \n'{\"status\":\"warn\"}'\n",
        20,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Warn, "{o:?}");
}

#[test]
fn ps_an_object_written_out_is_not_an_answer() {
    let Some(o) = ps("ps-object", "@{ status = 'ok' }\n", 20) else {
        return;
    };
    never_green(&o);
}

#[test]
fn ps_nested_json_converts_and_reads() {
    let Some(o) = ps(
        "ps-nested",
        "[ordered]@{ status = 'ok'; summary = 'n'; detail = @('a', 'b') } | ConvertTo-Json -Compress -Depth 5\n",
        20,
    ) else {
        return;
    };
    assert_eq!((o.status, o.detail.as_str()), (Status::Ok, "a\nb"), "{o:?}");
}

#[test]
fn ps_a_process_started_and_left_does_not_hold_the_run() {
    let started = Instant::now();
    let Some(o) = ps(
        "ps-bg",
        "$p = if ($IsWindows -or $env:OS -eq 'Windows_NT') { 'ping' } else { 'sleep' }\n$a = if ($p -eq 'ping') { '-n 30 127.0.0.1' } else { '30' }\nStart-Process $p -ArgumentList $a -WindowStyle Hidden -ErrorAction SilentlyContinue\n'{\"status\":\"ok\"}'\n",
        40,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Ok, "{o:?}");
    quick(started, 15);
}

#[test]
fn ps_asking_for_input_does_not_wait_for_it() {
    let started = Instant::now();
    let Some(o) = ps(
        "ps-readhost",
        "$x = Read-Host 'senha'
'{\"status\":\"warn\",\"summary\":\"lido: [' + $x + ']\"}'
",
        10,
    ) else {
        return;
    };
    // No input to be had: Read-Host gets nothing and the script goes on,
    // rather than the run waiting for an answer nobody can type.
    quick(started, 15);
    assert_eq!(
        (o.status, o.summary.as_str()),
        (Status::Warn, "lido: []"),
        "{o:?}"
    );
}

#[test]
fn ps_an_answer_written_with_write_host_is_read() {
    let Some(o) = ps("ps-host", "Write-Host '{\"status\":\"ok\"}'\n", 20) else {
        return;
    };
    assert_eq!(o.status, Status::Ok, "{o:?}");
}

#[test]
fn ps_an_exit_code_after_the_answer_keeps_the_answer() {
    let Some(o) = ps("ps-exit", "'{\"status\":\"error\"}'\nexit 3\n", 20) else {
        return;
    };
    assert_eq!(o.status, Status::Error);
}

#[test]
fn ps_output_forced_to_utf8_is_read_as_utf8() {
    let Some(o) = ps(
        "ps-utf8",
        "[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)\n'{\"status\":\"ok\",\"summary\":\"operações\"}'\n",
        20,
    ) else {
        return;
    };
    assert_eq!(o.summary, "operações", "{o:?}");
}

#[test]
fn ps_an_endless_loop_is_stopped_at_the_timeout() {
    let started = Instant::now();
    let Some(o) = ps("ps-loop", "while ($true) { }\n", 3) else {
        return;
    };
    never_green(&o);
    quick(started, 15);
}

// ---------------------------------------------------------------------------
// ObjectScript
// ---------------------------------------------------------------------------

#[test]
fn cos_an_undefined_variable_is_unknown_and_does_not_hang() {
    let started = Instant::now();
    let Some(o) = cos("cos-undef", "notDefinedAnywhere", 20) else {
        return;
    };
    never_green(&o);
    quick(started, 25);
}

#[test]
fn cos_a_syntax_error_is_unknown() {
    let Some(o) = cos("cos-syntax", "$$$(((", 20) else {
        return;
    };
    never_green(&o);
}

#[test]
fn cos_a_value_that_is_not_json_is_unknown() {
    let Some(o) = cos("cos-one", "1", 20) else {
        return;
    };
    never_green(&o);
}

#[test]
fn cos_quotes_in_the_answer_survive() {
    let Some(o) = cos(
        "cos-quotes",
        r#"$XECUTE("() set o=##class(%DynamicObject).%New() do o.%Set(""status"",""ok""),o.%Set(""summary"",""disse """"oi"""""") quit o.%ToJSON()")"#,
        20,
    ) else {
        return;
    };
    assert_eq!(
        (o.status, o.summary.as_str()),
        (Status::Ok, "disse \"oi\""),
        "{o:?}"
    );
}

#[test]
fn cos_an_endless_loop_is_stopped_at_the_timeout() {
    let started = Instant::now();
    let Some(o) = cos("cos-loop", r#"$XECUTE("() for { } quit 1")"#, 3) else {
        return;
    };
    never_green(&o);
    quick(started, 15);
}

#[test]
fn cos_a_read_that_swallows_the_halt_is_stopped_at_the_timeout() {
    let started = Instant::now();
    let Some(o) = cos("cos-read", r#"$XECUTE("() read x quit x")"#, 3) else {
        return;
    };
    never_green(&o);
    quick(started, 15);
}

#[test]
fn cos_noise_written_before_the_value_does_not_matter() {
    let Some(o) = cos(
        "cos-noise",
        r#"$XECUTE("() write ""ruido"",!,""{""""status"""":""""ok""""}"",! quit ""{""""status"""":""""warn""""}""")"#,
        20,
    ) else {
        return;
    };
    assert_eq!(o.status, Status::Warn, "{o:?}");
}

#[test]
fn cos_a_halt_inside_the_expression_is_unknown() {
    let Some(o) = cos("cos-halt", r#"$XECUTE("() halt")"#, 20) else {
        return;
    };
    never_green(&o);
}

#[test]
fn cos_changing_namespace_inside_still_answers() {
    let Some(o) = cos(
        "cos-zn",
        r#"$XECUTE("() new $namespace set $namespace=""%SYS"" quit ""{""""status"""":""""ok"""",""""summary"""":""""""_$namespace_""""""}""")"#,
        20,
    ) else {
        return;
    };
    assert_eq!(
        (o.status, o.summary.as_str()),
        (Status::Ok, "%SYS"),
        "{o:?}"
    );
}

#[test]
fn cos_a_value_of_a_hundred_kilobytes_is_read_whole() {
    let Some(o) = cos(
        "cos-long",
        r#"$XECUTE("() set o=##class(%DynamicObject).%New() do o.%Set(""status"",""ok""),o.%Set(""detail"",$translate($justify("""",100000),"" "",""x"")) quit o.%ToJSON()")"#,
        30,
    ) else {
        return;
    };
    assert_eq!(
        (o.status, o.detail.len()),
        (Status::Ok, 100000),
        "{:?}",
        o.status
    );
}
