//! The command line, through the built binary: exit codes and the fallback
//! server.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Output, Stdio};

const SERVER: &str = env!("CARGO_BIN_EXE_microscope-server");

fn run(args: &[&str]) -> Output {
    Command::new(SERVER)
        .args(args)
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
fn fallback_serves_an_error_page_instead_of_exiting() {
    let mut child = Command::new(SERVER)
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
}
