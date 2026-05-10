use std::collections::HashSet;

use aproman::pw_monitor;

#[test]
fn skips_initial_dump_and_reports_new_error() {
    let mut seen = HashSet::new();
    let (_, initial_done, errors) = pw_monitor::extract_errors(
        r#"[{"id":1,"type":"PipeWire:Interface:Node","info":{"state":"error","props":{"node.name":"old"}}}]"#,
        false,
        &mut seen,
    );
    assert!(initial_done);
    assert!(errors.is_empty());

    let (_, _, errors) = pw_monitor::extract_errors(
        r#"[{"id":2,"type":"PipeWire:Interface:Node","info":{"state":"error","props":{"node.name":"new"}}}]"#,
        true,
        &mut seen,
    );
    assert_eq!(vec!["node new entered error state"], errors);
}
