use std::path::Path;

use aproman::config;

#[test]
fn parse_conf_ignores_comments_and_blank_lines() {
    let entries = config::parse_conf(
        "\n# comment\n--card=alsa_card.pci-0000_01_00.1\n--profile=pro-audio\n",
        Path::new("test.conf"),
    )
    .unwrap();
    assert_eq!(
        vec![
            (
                "--card".to_string(),
                "alsa_card.pci-0000_01_00.1".to_string()
            ),
            ("--profile".to_string(), "pro-audio".to_string()),
        ],
        entries
    );
}

#[test]
fn parse_conf_rejects_malformed_line() {
    let err = config::parse_conf("card=bad\n", Path::new("test.conf")).unwrap_err();
    assert!(err.to_string().contains("malformed line"));
}
