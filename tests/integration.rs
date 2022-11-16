use assert_cmd::Command;
use chrono::Utc;

const BIN_NAME: &str = env!("CARGO_BIN_EXE_ikconfig");
const PATH_VMLINUX: &str = "tests/data/vmlinux";

#[test]
fn test_output_has_ikconfig_enabled() {
    let output = Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(PATH_VMLINUX)
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
        .arg(PATH_VMLINUX)
        .assert()
        .success();
    println!(
        "{:20}: {:-5} ms",
        env!("CARGO_PKG_NAME"),
        (Utc::now() - start).num_milliseconds()
    );

    let start = Utc::now();
    Command::new("tests/extract-ikconfig")
        .arg(PATH_VMLINUX)
        .unwrap();
    println!(
        "{:20}: {:-5} ms",
        "extract-ikconfig",
        (Utc::now() - start).num_milliseconds()
    );
}
