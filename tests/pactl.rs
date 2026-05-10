use aproman::pactl;

const PACTL_CARDS_OUTPUT: &str = r#"Card #0
	Name: alsa_card.pci-0000_00_1f.3
	Active Profile: output:analog-stereo
	Properties:
		alsa.card_name = "HDA Intel PCH"
	ports:
		analog-output-lineout: Line Out
			Properties:
				port.type = "analog"
Card #1
	Name: alsa_card.pci-0000_01_00.1
	Active Profile: output:hdmi-stereo
	Properties:
		alsa.card_name = "HDA NVidia"
	Profiles:
		output:hdmi-stereo: Digital Stereo (HDMI) Output (sinks: 1, sources: 0, priority: 5900, available: yes)
		output:hdmi-surround: Digital Surround 5.1 (HDMI) Output (sinks: 1, sources: 0, priority: 800, available: yes)
		off: Off (sinks: 0, sources: 0, priority: 0, available: yes)
	ports:
		hdmi-output-0: HDMI / DisplayPort 1
			Properties:
				port.type = "hdmi"
"#;

#[test]
fn parse_cards_detects_hdmi_card() {
    let cards = pactl::parse_cards(PACTL_CARDS_OUTPUT);
    assert_eq!(2, cards.len());
    assert_eq!("alsa_card.pci-0000_01_00.1", cards[1].name);
    assert_eq!(Some("HDA NVidia"), cards[1].label.as_deref());
    assert!(cards[1].has_hdmi);
    assert_eq!(
        Some("output:hdmi-stereo"),
        cards[1].active_profile.as_deref()
    );
}

#[test]
fn sorted_profiles_are_priority_descending() {
    let cards = pactl::parse_cards(PACTL_CARDS_OUTPUT);
    let names: Vec<_> = cards[1]
        .sorted_profiles()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(
        vec!["output:hdmi-stereo", "output:hdmi-surround", "off"],
        names
    );
}

#[test]
fn best_available_profile_excludes_off() {
    let cards = pactl::parse_cards(PACTL_CARDS_OUTPUT);
    assert_eq!(
        Some("output:hdmi-stereo".to_string()),
        cards[1].best_available_profile()
    );
}
