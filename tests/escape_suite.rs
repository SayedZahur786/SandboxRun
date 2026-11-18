//! Adversarial capability/limit checks, not a claim of exhaustive exploit coverage.
use sandboxrun::{ExecutionRequest, Language, Limits, Sandbox};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::Command};

#[test]
fn escape_suite() {
    // A separate process seeds the host environment without changing the parallel test runner.
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "adversarial_child", "--nocapture"])
        .env(
            "SANDBOXRUN_HOST_CANARY",
            "HOST_SECRET_MUST_NOT_REACH_GUEST_7b260fb1",
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Case {
    name: &'static str,
    category: &'static str,
    language: Language,
    code: String,
    limits: Limits,
    expected: &'static str,
}
fn py_denied(operation: &str) -> String {
    format!(
        "import os, socket\ntry:\n {}\nexcept OSError as e:\n assert e.errno in (2, 8, 28, 44, 54, 58, 63, 76), repr(e)\n print('BLOCKED', e.errno)\nelse:\n raise AssertionError('ESCAPED')",
        operation
    )
}
fn js_assert(condition: &str) -> String {
    format!("if (!({condition})) throw Error('ESCAPED'); console.log('BLOCKED');")
}
fn suite_limits() -> Limits {
    Limits {
        fuel: 2_000_000_000,
        ..Limits::default()
    }
}

fn request(language: Language, code: String, limits: Limits) -> ExecutionRequest {
    ExecutionRequest {
        language,
        code,
        stdin: String::new(),
        limits,
    }
}

#[test]
#[ignore = "invoked with an isolated host environment by escape_suite"]
fn adversarial_child() {
    assert_eq!(
        std::env::var("SANDBOXRUN_HOST_CANARY").unwrap(),
        "HOST_SECRET_MUST_NOT_REACH_GUEST_7b260fb1"
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture =
        Fixture(std::env::temp_dir().join(format!("sandboxrun-escape-{}", uuid::Uuid::new_v4())));
    std::fs::create_dir_all(&fixture.0).unwrap();
    let host_file = fixture.0.join("host-canary.txt");
    let canary = "HOST_FILE_MUST_NOT_REACH_GUEST_8d510ce7";
    std::fs::write(&host_file, canary).unwrap();
    let file = serde_json::to_string(&host_file.to_str().unwrap()).unwrap();
    let directory = serde_json::to_string(&fixture.0.to_str().unwrap()).unwrap();
    let sandbox = Sandbox::new(root.join("runtimes")).unwrap();
    let positive = |lang, code: &str| {
        let r = sandbox
            .execute(request(lang, code.to_string(), suite_limits()))
            .unwrap();
        assert_eq!(r.status, "ok", "positive control: {}", r.stderr);
        assert!(r.stdout.contains("CONTROL_OK 42"), "{}", r.stdout);
    };
    positive(
        Language::Python,
        "import json, math; print('CONTROL_OK', json.loads('{\"n\":36}')['n'] + int(math.sqrt(36)))",
    );
    positive(
        Language::Javascript,
        "console.log('CONTROL_OK', JSON.parse('{\"n\":36}').n + new Uint8Array([6])[0]);",
    );
    let mut cases = Vec::new();
    let mut add = |name, category, language, code| {
        cases.push(Case {
            name,
            category,
            language,
            code,
            limits: suite_limits(),
            expected: "ok",
        })
    };
    add(
        "python_read_host_canary",
        "filesystem-denial",
        Language::Python,
        py_denied(&format!("open({file}).read()")),
    );
    add(
        "python_read_proc_environment",
        "filesystem-denial",
        Language::Python,
        py_denied("open('/proc/self/environ','rb').read()"),
    );
    add(
        "python_read_ungranted_descriptor",
        "descriptor-denial",
        Language::Python,
        py_denied("os.read(1024, 100)"),
    );
    add(
        "python_traverse_embedded_lib",
        "filesystem-denial",
        Language::Python,
        py_denied("open('/lib/../../etc/passwd').read()"),
    );
    add(
        "python_create_host_symlink",
        "filesystem-denial",
        Language::Python,
        py_denied(&format!("os.symlink('/etc/passwd', {directory} + '/link')")),
    );
    add(
        "python_overwrite_host_canary",
        "filesystem-denial",
        Language::Python,
        py_denied(&format!("open({file}, 'w').write('ESCAPED')")),
    );
    add(
        "python_create_temporary_file",
        "filesystem-denial",
        Language::Python,
        py_denied(&format!(
            "open({directory} + '/new-file', 'w').write('ESCAPED')"
        )),
    );
    add(
        "python_overwrite_embedded_stdlib",
        "filesystem-denial",
        Language::Python,
        py_denied("os.write(os.open('/lib/python3.15/os.py', os.O_WRONLY), b'ESCAPED')"),
    );
    add(
        "python_create_host_directory",
        "filesystem-denial",
        Language::Python,
        py_denied(&format!("os.mkdir({directory} + '/new-dir')")),
    );
    add(
        "python_list_host_directory",
        "filesystem-denial",
        Language::Python,
        py_denied(&format!("os.listdir({directory})")),
    );
    add(
        "python_delete_host_canary",
        "filesystem-denial",
        Language::Python,
        py_denied(&format!("os.unlink({file})")),
    );
    add("python_host_environment", "environment-isolation", Language::Python, "import os; assert os.getenv('SANDBOXRUN_HOST_CANARY') is None; assert set(os.environ) == {'PYTHONHOME'}; print('BLOCKED')".into());
    add(
        "python_write_ungranted_descriptor",
        "descriptor-denial",
        Language::Python,
        py_denied("os.write(1024, b'ESCAPED')"),
    );
    add(
        "python_fork_capability_absent",
        "capability-absence",
        Language::Python,
        "import os; assert not hasattr(os,'fork'); print('BLOCKED')".into(),
    );
    add("python_native_ffi_capability_absent", "capability-absence", Language::Python, "try:\n import _ctypes\nexcept ModuleNotFoundError as e:\n assert e.name == '_ctypes'\n print('BLOCKED')\nelse:\n raise AssertionError('ESCAPED')".into());
    add(
        "python_tcp_socket_denied",
        "network-denial",
        Language::Python,
        py_denied("__import__('socket').socket(2, 1)"),
    );
    add(
        "python_udp_socket_denied",
        "network-denial",
        Language::Python,
        py_denied("__import__('socket').socket(2, 2)"),
    );
    add(
        "python_dns_capability_absent",
        "capability-absence",
        Language::Python,
        "import _socket; assert not hasattr(_socket,'getaddrinfo'); print('BLOCKED')".into(),
    );
    add(
        "python_system_capability_absent",
        "capability-absence",
        Language::Python,
        "import os; assert not hasattr(os,'system'); print('BLOCKED')".into(),
    );
    add(
        "javascript_read_host_canary",
        "filesystem-denial",
        Language::Javascript,
        js_assert(&format!("std.loadFile({file}) === null")),
    );
    add(
        "javascript_overwrite_host_canary",
        "filesystem-denial",
        Language::Javascript,
        js_assert(&format!("std.open({file}, 'w') === null")),
    );
    add(
        "javascript_host_environment",
        "environment-isolation",
        Language::Javascript,
        js_assert("std.getenv('SANDBOXRUN_HOST_CANARY') === undefined"),
    );
    add(
        "javascript_exec_capability_absent",
        "capability-absence",
        Language::Javascript,
        js_assert("typeof os.exec === 'undefined'"),
    );
    add(
        "javascript_fetch_capability_absent",
        "capability-absence",
        Language::Javascript,
        js_assert("typeof fetch === 'undefined'"),
    );
    add(
        "javascript_require_capability_absent",
        "capability-absence",
        Language::Javascript,
        js_assert("typeof require === 'undefined'"),
    );
    add(
        "javascript_process_capability_absent",
        "capability-absence",
        Language::Javascript,
        js_assert("typeof process === 'undefined'"),
    );
    add(
        "javascript_list_host_directory",
        "filesystem-denial",
        Language::Javascript,
        js_assert(&format!("os.readdir({directory})[1] !== 0")),
    );

    add(
        "javascript_delete_host_canary",
        "filesystem-denial",
        Language::Javascript,
        js_assert(&format!("os.remove({file}) !== 0")),
    );
    cases.push(Case {
        name: "javascript_sleep_hostcall_denied",
        category: "blocking-hostcall-denial",
        language: Language::Javascript,
        code: "os.sleep(1000);".into(),
        limits: suite_limits(),
        expected: "capability_denied",
    });
    cases.push(Case {
        name: "python_sleep_hostcall_denied",
        category: "blocking-hostcall-denial",
        language: Language::Python,
        code: "import time; time.sleep(1)".into(),
        limits: suite_limits(),
        expected: "capability_denied",
    });
    for (name, language, code) in [
        (
            "python_fuel_exhaustion",
            Language::Python,
            "while True: pass",
        ),
        (
            "javascript_fuel_exhaustion",
            Language::Javascript,
            "while(true) {}",
        ),
    ] {
        let mut limits = suite_limits();
        limits.fuel = 300_000_000;
        limits.wall_time_ms = 5000;
        cases.push(Case {
            name,
            category: "resource-limit",
            language,
            code: code.into(),
            limits,
            expected: "fuel_limit",
        });
    }
    for (name, language, code) in [
        ("python_wall_deadline", Language::Python, "while True: pass"),
        (
            "javascript_wall_deadline",
            Language::Javascript,
            "while(true) {}",
        ),
    ] {
        let mut limits = suite_limits();
        limits.wall_time_ms = 50;
        limits.fuel = 5_000_000_000;
        cases.push(Case {
            name,
            category: "resource-limit",
            language,
            code: code.into(),
            limits,
            expected: "time_limit",
        });
    }
    for (name, language, code) in [
        (
            "python_memory_growth",
            Language::Python,
            "x = bytearray(256 * 1024 * 1024)",
        ),
        (
            "javascript_memory_growth",
            Language::Javascript,
            "let x = new Uint8Array(256 * 1024 * 1024);",
        ),
    ] {
        let mut limits = suite_limits();
        limits.memory_bytes = 96 * 1024 * 1024;
        cases.push(Case {
            name,
            category: "resource-limit",
            language,
            code: code.into(),
            limits,
            expected: "memory_limit",
        });
    }
    for (name, language, code) in [
        (
            "python_output_limit",
            Language::Python,
            "print('x' * 4096, flush=True)",
        ),
        (
            "javascript_output_limit",
            Language::Javascript,
            "console.log('x'.repeat(4096));",
        ),
    ] {
        let mut limits = suite_limits();
        limits.output_bytes = 256;
        cases.push(Case {
            name,
            category: "resource-limit",
            language,
            code: code.into(),
            limits,
            expected: "output_limit",
        });
    }
    for (language, code) in [
        (
            Language::Python,
            "__sandboxrun_cross_run_secret = 'PERSISTED'; print('seeded')",
        ),
        (
            Language::Javascript,
            "globalThis.__sandboxrun_cross_run_secret='PERSISTED'; console.log('seeded');",
        ),
    ] {
        let seed = sandbox
            .execute(request(language, code.into(), suite_limits()))
            .unwrap();
        assert_eq!(seed.status, "ok");
    }
    cases.push(Case {
        name: "python_fresh_global_state",
        category: "cross-run-isolation",
        language: Language::Python,
        code: "assert '__sandboxrun_cross_run_secret' not in globals(); print('BLOCKED')".into(),
        limits: suite_limits(),
        expected: "ok",
    });
    cases.push(Case {
        name: "javascript_fresh_global_state",
        category: "cross-run-isolation",
        language: Language::Javascript,
        code: js_assert("typeof globalThis.__sandboxrun_cross_run_secret === 'undefined'"),
        limits: suite_limits(),
        expected: "ok",
    });
    assert_eq!(cases.len(), 40);
    let mut results = Vec::<Value>::new();
    for case in cases {
        let marker = format!("START:{}", case.name);
        let prefix = match case.language {
            Language::Python => format!("print({:?}, flush=True)\n", marker),
            Language::Javascript => format!("console.log({:?});\n", marker),
        };
        let receipt = sandbox
            .execute(request(case.language, prefix + &case.code, case.limits))
            .unwrap();
        let reached = receipt.stdout.contains(&marker);
        let status_matches = receipt.status == case.expected;
        let explicit_denial =
            case.expected != "ok" || receipt.stdout.lines().any(|l| l.starts_with("BLOCKED"));
        let no_canary = !receipt.stdout.contains(canary)
            && !receipt.stderr.contains(canary)
            && !receipt
                .stdout
                .contains("HOST_SECRET_MUST_NOT_REACH_GUEST_7b260fb1")
            && !receipt
                .stderr
                .contains("HOST_SECRET_MUST_NOT_REACH_GUEST_7b260fb1");
        let host_unchanged = std::fs::read_to_string(&host_file).ok().as_deref() == Some(canary)
            && std::fs::read_dir(&fixture.0).unwrap().count() == 1;
        let bounded_output = receipt.stdout.len() <= receipt.limits.output_bytes
            && receipt.stderr.len() <= receipt.limits.output_bytes;
        // 250 ms allows host scheduling jitter; a blocked one-second sleep must fail.
        let duration_budget_ms = if case.category == "blocking-hostcall-denial" {
            250.0
        } else {
            receipt.limits.wall_time_ms as f64 + 250.0
        };
        let bounded_duration = receipt.duration_ms <= duration_budget_ms;
        let passed = reached
            && status_matches
            && explicit_denial
            && no_canary
            && host_unchanged
            && bounded_output
            && bounded_duration;
        results.push(json!({"name":case.name,"category":case.category,"passed":passed,"start_marker_reached":reached,"expected_status":case.expected,"observed_status":receipt.status,"explicit_denial_or_limit":explicit_denial,"host_canary_unchanged":host_unchanged,"no_secret_disclosure":no_canary,"output_within_cap":bounded_output,"duration_within_budget":bounded_duration,"duration_budget_with_scheduler_tolerance_ms":duration_budget_ms,"duration_ms":receipt.duration_ms,"fuel_consumed":receipt.fuel_consumed,"stdout":receipt.stdout,"stderr":receipt.stderr}));
    }
    positive(
        Language::Python,
        "import json; print('CONTROL_OK', json.loads('42'))",
    );
    positive(
        Language::Javascript,
        "console.log('CONTROL_OK', JSON.parse('42'));",
    );
    // Invalid UTF-8 is lossy decoded; the serialized UTF-8 receipt still must respect the byte cap.
    let mut utf8_limits = suite_limits();
    utf8_limits.output_bytes = 256;
    let utf8 = sandbox
        .execute(request(
            Language::Python,
            "import os; os.write(1, bytes([255]) * 4096)".into(),
            utf8_limits,
        ))
        .unwrap();
    assert_eq!(utf8.status, "output_limit");
    assert!(utf8.output_truncated);
    assert!(
        utf8.stdout.len() <= 256,
        "UTF-8 replacement expanded the output beyond its cap"
    );
    let passed = results.iter().filter(|v| v["passed"] == true).count();
    let report = json!({"suite":"40 named adversarial capability and resource checks","warning":"Regression evidence, not a proof against all escapes or an external security audit.","host":format!("{} {}",std::env::consts::OS,std::env::consts::ARCH),"generated_at_unix_seconds":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),"runtime_source_sha256":format!("{:x}",Sha256::digest(std::fs::read(root.join("src/lib.rs")).unwrap())),"suite_source_sha256":format!("{:x}",Sha256::digest(std::fs::read(root.join("tests/escape_suite.rs")).unwrap())),"startup_ms":sandbox.startup_ms,"total":40,"passed":passed,"positive_controls_before_and_after":"passed","invalid_utf8_output_cap_regression":"passed","cases":results});
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(
        root.join("docs/escape-results.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    assert_eq!(
        passed,
        40,
        "{}",
        serde_json::to_string_pretty(&report).unwrap()
    );
}
