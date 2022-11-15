use assert_cmd::Command;
use chrono::Utc;

const BIN_NAME: &str = env!("CARGO_PKG_NAME");

#[test]
fn test_output_has_ikconfig_enabled() {
    let output = Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg("tests/data/vmlinux")
        .output()
        .unwrap();

    assert!(std::str::from_utf8(&output.stdout)
        .unwrap()
        .contains("CONFIG_IKCONFIG=y"));
}

#[test]
fn compare_to_shell_script() {
    let start = Utc::now();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg("tests/data/vmlinux")
        .assert()
        .success();
    println!(
        "{}: {} ms",
        BIN_NAME,
        (Utc::now() - start).num_milliseconds()
    );

    let start = Utc::now();
    Command::new("tests/extract-ikconfig")
        .arg("tests/data/vmlinux")
        .unwrap();
    println!(
        "extra-ikconfig: {} ms",
        (Utc::now() - start).num_milliseconds()
    );
}
