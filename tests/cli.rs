use aproman::cli::{Cli, Command};
use clap::Parser;

#[test]
fn toggle_is_cycle_alias() {
    let parsed = Cli::parse_from(["aproman", "toggle"]);
    assert_eq!(Some(Command::Cycle), parsed.command);
}
