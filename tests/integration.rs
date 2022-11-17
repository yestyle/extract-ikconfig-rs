use assert_cmd::Command;
use chrono::Utc;

const BIN_NAME: &str = env!("CARGO_BIN_EXE_ikconfig");
const PATH_VMLINUX: &str = "tests/data/vmlinux";
const PATH_VMLINUZ: &str = "tests/data/vmlinuz-linux";

#[test]
fn test_extract_vmlinux() {
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
fn test_extract_vmlinuz() {
    let output = Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(PATH_VMLINUZ)
        .output()
        .unwrap();

    assert!(std::str::from_utf8(&output.stdout)
        .unwrap()
        .contains("CONFIG_IKCONFIG=y"));
}

#[test]
fn compare_to_shell_script_vmlinux() {
    println!("Extracting {}", PATH_VMLINUX);
    let start = Utc::now();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(PATH_VMLINUX)
        .assert()
        .success();
    println!(
        "{:20}: {:-10} us",
        env!("CARGO_PKG_NAME"),
        (Utc::now() - start).num_microseconds().unwrap()
    );

    let start = Utc::now();
    Command::new("tests/extract-ikconfig")
        .arg(PATH_VMLINUX)
        .unwrap();
    println!(
        "{:20}: {:-10} us",
        "extract-ikconfig",
        (Utc::now() - start).num_microseconds().unwrap()
    );
}

#[test]
fn compare_to_shell_script_vmlinuz() {
    println!("Extracting {}", PATH_VMLINUZ);
    let start = Utc::now();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(PATH_VMLINUZ)
        .assert()
        .success();
    println!(
        "{:20}: {:-10} us",
        env!("CARGO_PKG_NAME"),
        (Utc::now() - start).num_microseconds().unwrap()
    );

    let start = Utc::now();
    Command::new("tests/extract-ikconfig")
        .arg(PATH_VMLINUZ)
        .unwrap();
    println!(
        "{:20}: {:-10} us",
        "extract-ikconfig",
        (Utc::now() - start).num_microseconds().unwrap()
    );
}
