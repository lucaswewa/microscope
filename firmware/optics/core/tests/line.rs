//! Lines from bytes, in whatever pieces USB delivers them, and lines to send.

use core::fmt::Write;

use optics_core::{Controller, Line, LineBuffer, MAX_LINE, Request};

/// What a buffer makes of some bytes: each line's text, or `None` for one that it rejected.
fn lines(buffer: &mut LineBuffer, bytes: &[u8]) -> Vec<Option<String>> {
    bytes
        .iter()
        .filter_map(|&byte| buffer.push(byte))
        .map(|request| match request {
            Request::Line(line) => Some(line.as_str().to_string()),
            Request::Rejected => None,
            Request::HostLost => panic!("only the USB side says this"),
        })
        .collect()
}

#[test]
fn a_line_can_arrive_in_pieces() {
    let mut buffer = LineBuffer::new();
    assert_eq!(lines(&mut buffer, b"led "), []);
    assert_eq!(lines(&mut buffer, b"1 o"), []);
    assert_eq!(lines(&mut buffer, b"n\n"), [Some("led 1 on".to_string())]);
}

#[test]
fn several_lines_can_arrive_at_once() {
    let mut buffer = LineBuffer::new();
    let found = lines(&mut buffer, b"status\nversion\nled");
    assert_eq!(
        found,
        [Some("status".to_string()), Some("version".to_string())]
    );
    assert_eq!(
        lines(&mut buffer, b" all off\n"),
        [Some("led all off".to_string())]
    );
}

#[test]
fn either_line_ending_works_and_empty_lines_are_ignored() {
    let mut buffer = LineBuffer::new();
    assert_eq!(
        lines(&mut buffer, b"status\r\n\r\n\n"),
        [Some("status".to_string())]
    );
}

#[test]
fn the_longest_line_is_taken_and_a_longer_one_is_rejected_whole() {
    let mut buffer = LineBuffer::new();
    let longest = "x".repeat(MAX_LINE);
    let found = lines(&mut buffer, format!("{longest}\n").as_bytes());
    assert_eq!(found, [Some(longest.clone())]);

    assert_eq!(
        lines(&mut buffer, format!("{longest}y\n").as_bytes()),
        [None],
        "one byte over"
    );
    assert_eq!(
        lines(
            &mut buffer,
            format!("{longest}y and more\nstatus\n").as_bytes()
        ),
        [None, Some("status".to_string())]
    );
}

#[test]
fn bytes_that_are_not_printable_ascii_reject_the_line_only() {
    let mut buffer = LineBuffer::new();
    for bad in [&b"led \xC3\xA9\n"[..], b"led\t1\n", b"\x00\n", b"\xFF\n"] {
        assert_eq!(lines(&mut buffer, bad), [None], "{bad:?}");
    }
    assert_eq!(
        lines(&mut buffer, b"status\n"),
        [Some("status".to_string())]
    );
}

#[test]
fn clearing_forgets_a_partial_line() {
    let mut buffer = LineBuffer::new();
    lines(&mut buffer, b"led 1 o");
    buffer.clear();
    assert_eq!(
        lines(&mut buffer, b"status\n"),
        [Some("status".to_string())]
    );
}

#[test]
fn a_rejected_line_has_its_own_reply() {
    assert_eq!(Controller::rejected_line().as_str(), "err bad-line");
}

#[test]
fn lines_are_sent_with_one_line_feed() {
    let mut line = Line::new();
    write!(line, "ok").unwrap();
    assert_eq!(line.to_send(), b"ok\n");
    assert_eq!(line.to_send(), b"ok\n", "asking again adds no second one");
}

#[test]
fn the_longest_line_still_has_room_for_its_line_feed() {
    let mut line = Line::new();
    write!(line, "{}", "x".repeat(MAX_LINE)).unwrap();
    assert_eq!(line.to_send().len(), MAX_LINE + 1);
    assert_eq!(line.to_send().last(), Some(&b'\n'));
}

#[test]
fn text_past_the_end_is_cut_and_reported() {
    let mut line = Line::new();
    assert!(write!(line, "{}", "x".repeat(MAX_LINE + 5)).is_err());
    assert_eq!(line.as_str().len(), MAX_LINE);
}
