use aproman::daemon::{self, Command};

#[test]
fn decode_command_accepts_cycle() {
    assert_eq!(
        Command::Cycle(None),
        daemon::decode_command(b"cycle").unwrap()
    );
    assert_eq!(
        Command::Cycle(Some("pro-audio".to_string())),
        daemon::decode_command(b"cycle pro-audio").unwrap()
    );
}

#[test]
fn decode_command_accepts_toggle_alias() {
    assert_eq!(
        Command::Cycle(None),
        daemon::decode_command(b"toggle").unwrap()
    );
    assert_eq!(
        Command::Cycle(Some("pro-audio".to_string())),
        daemon::decode_command(b"toggle pro-audio").unwrap()
    );
}

#[test]
fn decode_command_accepts_reload() {
    assert_eq!(Command::Reload, daemon::decode_command(b"reload").unwrap());
}

#[test]
fn decode_command_rejects_unknown() {
    assert!(daemon::decode_command(b"invalid").is_err());
}
