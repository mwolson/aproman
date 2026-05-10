use aproman::service_files;

#[test]
fn bundled_service_files_name_aproman() {
    assert!(service_files::SYSTEMD_UNIT.contains("aproman"));
    assert!(service_files::OPENRC_USER.contains("aproman"));
    assert!(service_files::OPENRC_SYSTEM.contains("aproman"));
}
