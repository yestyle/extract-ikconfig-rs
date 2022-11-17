use assert_cmd::Command;
use chrono::Utc;

const BIN_NAME: &str = env!("CARGO_BIN_EXE_ikconfig");
const PATH_VMLINUX_RAW: &str = "tests/data/vmlinux";
const PATH_VMLINUX_GZIP: &str = "tests/data/vmlinux.gz";
const PATH_VMLINUX_ZSTD: &str = "tests/data/vmlinux.zst";

#[test]
fn test_extract_vmlinux_raw() {
    let output = Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(PATH_VMLINUX_RAW)
        .output()
        .unwrap();

    let configs = std::str::from_utf8(&output.stdout).unwrap();
    assert!(configs.contains("CONFIG_IKCONFIG=y"));
}

#[test]
fn test_extract_vmlinux_gzip() {
    let output = Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(PATH_VMLINUX_GZIP)
        .output()
        .unwrap();

    let configs = std::str::from_utf8(&output.stdout).unwrap();
    assert!(configs.contains("CONFIG_IKCONFIG=y"));
    assert!(configs.contains("CONFIG_KERNEL_GZIP=y"));
}

#[test]
fn test_extract_vmlinux_zstd() {
    let output = Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(PATH_VMLINUX_ZSTD)
        .output()
        .unwrap();

    let configs = std::str::from_utf8(&output.stdout).unwrap();
    assert!(configs.contains("CONFIG_IKCONFIG=y"));
    assert!(configs.contains("CONFIG_KERNEL_ZSTD=y"));
}

#[test]
fn compare_to_shell_script_vmlinux_raw() {
    println!("Extracting {}", PATH_VMLINUX_RAW);
    let start = Utc::now();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(PATH_VMLINUX_RAW)
        .assert()
        .success();
    println!(
        "{:20}: {:-10} us",
        env!("CARGO_PKG_NAME"),
        (Utc::now() - start).num_microseconds().unwrap()
    );

    let start = Utc::now();
    Command::new("tests/extract-ikconfig")
        .arg(PATH_VMLINUX_RAW)
        .unwrap();
    println!(
        "{:20}: {:-10} us",
        "extract-ikconfig",
        (Utc::now() - start).num_microseconds().unwrap()
    );
}

#[test]
fn compare_to_shell_script_vmlinux_gzip() {
    println!("Extracting {}", PATH_VMLINUX_GZIP);
    let start = Utc::now();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(PATH_VMLINUX_GZIP)
        .assert()
        .success();
    println!(
        "{:20}: {:-10} us",
        env!("CARGO_PKG_NAME"),
        (Utc::now() - start).num_microseconds().unwrap()
    );

    let start = Utc::now();
    Command::new("tests/extract-ikconfig")
        .arg(PATH_VMLINUX_GZIP)
        .unwrap();
    println!(
        "{:20}: {:-10} us",
        "extract-ikconfig",
        (Utc::now() - start).num_microseconds().unwrap()
    );
}

#[test]
fn compare_to_shell_script_vmlinux_zstd() {
    println!("Extracting {}", PATH_VMLINUX_ZSTD);
    let start = Utc::now();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(PATH_VMLINUX_ZSTD)
        .assert()
        .success();
    println!(
        "{:20}: {:-10} us",
        env!("CARGO_PKG_NAME"),
        (Utc::now() - start).num_microseconds().unwrap()
    );

    let start = Utc::now();
    Command::new("tests/extract-ikconfig")
        .arg(PATH_VMLINUX_ZSTD)
        .unwrap();
    println!(
        "{:20}: {:-10} us",
        "extract-ikconfig",
        (Utc::now() - start).num_microseconds().unwrap()
    );
}
