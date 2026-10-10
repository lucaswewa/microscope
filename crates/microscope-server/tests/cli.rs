//! The command line, through the built binary: exit codes and the fallback
//! server. Each run is in a temporary folder, since the server writes log
//! files to `.microscope/logs` by default.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Output, Stdio};

const SERVER: &str = env!("CARGO_BIN_EXE_microscope-server");

fn run(args: &[&str]) -> Output {
    let folder = tempfile::tempdir().expect("a temporary folder");
    Command::new(SERVER)
        .args(args)
        .current_dir(folder.path())
        .output()
        .expect("the server binary runs")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("an exit code")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn version_exits_0() {
    let output = run(&["--version"]);
    assert_eq!(code(&output), 0);
    assert_eq!(
        text(&output.stdout).trim(),
        format!("microscope-server {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn an_invalid_command_line_exits_2() {
    assert_eq!(code(&run(&["--no-such-option"])), 2);
}

#[test]
fn no_configuration_exits_1() {
    let output = run(&[]);
    assert_eq!(code(&output), 1);
    assert!(text(&output.stderr).contains("No configuration"));
}

#[test]
fn a_missing_configuration_file_exits_1() {
    let output = run(&["-c", "no-such-config.json"]);
    assert_eq!(code(&output), 1);
    assert!(text(&output.stderr).contains("Could not find configuration file"));
}

#[test]
fn both_config_and_json_exit_1() {
    let output = run(&["-c", "a.json", "-j", "{}"]);
    assert_eq!(code(&output), 1);
}

#[test]
fn an_unknown_thing_type_exits_3() {
    let output = run(&["-j", r#"{"things": {"x": "no.such:Thing"}}"#]);
    assert_eq!(code(&output), 3);
    assert!(text(&output.stdout).contains("No Thing type is registered as 'no.such:Thing'"));
}

#[test]
fn an_invalid_application_config_exits_3() {
    let output = run(&[
        "-j",
        r#"{"things": {}, "application_config": {"data_folder": 3}}"#,
    ]);
    assert_eq!(code(&output), 3);
    assert!(text(&output.stdout).contains("application_config"));
}

#[test]
fn fallback_serves_an_error_page_with_the_log_instead_of_exiting() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let mut child = Command::new(SERVER)
        .current_dir(folder.path())
        .args([
            "--fallback",
            "--port",
            "0",
            "-j",
            r#"{"things": {"x": "no.such:Thing"}}"#,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the server binary runs");
    let stdout = child.stdout.take().expect("stdout is piped");
    let address = BufReader::new(stdout)
        .lines()
        .map_while(Result::ok)
        .find_map(|line| line.strip_prefix("listening on http://").map(str::to_owned));
    let page = address.map(|address| {
        let mut stream = TcpStream::connect(&address).expect("connects");
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n")
            .expect("writes");
        let mut page = String::new();
        stream.read_to_string(&mut page).expect("reads");
        page
    });
    let _ = child.kill();
    let _ = child.wait();

    let page = page.expect("the fallback server printed its address");
    assert!(page.contains("no.such:Thing"), "{page}");
    // The server log so far, which says where the log files are.
    assert!(page.contains("application configuration"), "{page}");

    // The default log folder, relative to the working directory.
    let log_files: Vec<_> = std::fs::read_dir(folder.path().join(".microscope/logs"))
        .expect("the log folder exists")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        log_files
            .iter()
            .any(|name| name.starts_with("microscope.") && name.ends_with(".log")),
        "{log_files:?}"
    );
}

#[test]
fn print_openapi_writes_the_committed_document() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let output = run(&[
        "-c",
        &format!("{root}/configs/simulation.json"),
        "--print-openapi",
    ]);
    assert_eq!(code(&output), 0, "{}", text(&output.stderr));
    let printed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON");
    assert!(printed["paths"]["/api/v1/stage/move_relative"].is_object());
    let committed = std::fs::read_to_string(format!("{root}/web/src/api/generated/openapi.json"))
        .expect("the committed document");
    let committed: serde_json::Value = serde_json::from_str(&committed).expect("JSON");
    assert!(
        printed == committed,
        "web/src/api/generated/openapi.json is out of date: run `npm run api:openapi` and \
         `npm run api:types` in web/, and commit the results (ADR-0021)"
    );
}
