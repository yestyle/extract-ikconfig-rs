use assert_cmd::Command;
use time::Instant;

const BIN_NAME: &str = env!("CARGO_BIN_EXE_ikconfig");
const SCRIPT_NAME: &str = "tests/extract-ikconfig";
const PATH_VMLINUX_RAW: &str = "tests/data/vmlinux";
const PATH_VMLINUX_GZIP: &str = "tests/data/vmlinux.gz";
const PATH_VMLINUX_XZ: &str = "tests/data/vmlinux.xz";
const PATH_VMLINUX_BZIP2: &str = "tests/data/vmlinux.bz2";
const PATH_VMLINUX_LZMA: &str = "tests/data/vmlinux.lzma";
const PATH_VMLINUX_LZ4: &str = "tests/data/vmlinux.lz4";
const PATH_VMLINUX_ZSTD: &str = "tests/data/vmlinux.zst";

fn test_extract_vmlinux(path: &str, config: &str) {
    let output = Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(path)
        .output()
        .unwrap();

    let configs = std::str::from_utf8(&output.stdout).unwrap();
    assert!(configs.contains("CONFIG_IKCONFIG=y"));
    if !config.is_empty() {
        assert!(configs.contains(format!("CONFIG_KERNEL_{config}=y").as_str()));
    }
}

#[test]
fn test_extract_vmlinux_raw() {
    test_extract_vmlinux(PATH_VMLINUX_RAW, "");
}

#[test]
fn test_extract_vmlinux_gzip() {
    test_extract_vmlinux(PATH_VMLINUX_GZIP, "GZIP");
}

#[test]
fn test_extract_vmlinux_xz() {
    test_extract_vmlinux(PATH_VMLINUX_XZ, "XZ");
}

#[test]
fn test_extract_vmlinux_bzip2() {
    test_extract_vmlinux(PATH_VMLINUX_BZIP2, "BZIP2");
}

#[test]
fn test_extract_vmlinux_lzma() {
    test_extract_vmlinux(PATH_VMLINUX_LZMA, "LZMA");
}

#[test]
fn test_extract_vmlinux_lz4() {
    test_extract_vmlinux(PATH_VMLINUX_LZ4, "LZ4");
}

#[test]
fn test_extract_vmlinux_zstd() {
    test_extract_vmlinux(PATH_VMLINUX_ZSTD, "ZSTD");
}

fn compare_to_shell_script(path: &str) {
    println!("Extracting {}", path);
    let instant = Instant::now();
    Command::cargo_bin(BIN_NAME)
        .unwrap()
        .arg(path)
        .assert()
        .success();
    println!(
        "{:20}: {:-10} us",
        env!("CARGO_PKG_NAME"),
        instant.elapsed().whole_microseconds()
    );

    let instant = Instant::now();
    Command::new(SCRIPT_NAME).arg(path).unwrap();
    println!(
        "{:20}: {:-10} us",
        "extract-ikconfig",
        instant.elapsed().whole_microseconds()
    );
}

#[test]
fn compare_to_shell_script_vmlinux_raw() {
    compare_to_shell_script(PATH_VMLINUX_RAW);
}

#[test]
fn compare_to_shell_script_vmlinux_gzip() {
    compare_to_shell_script(PATH_VMLINUX_GZIP);
}

#[test]
fn compare_to_shell_script_vmlinux_xz() {
    compare_to_shell_script(PATH_VMLINUX_XZ);
}

#[test]
fn compare_to_shell_script_vmlinux_bzip2() {
    compare_to_shell_script(PATH_VMLINUX_BZIP2);
}

#[test]
fn compare_to_shell_script_vmlinux_lzma() {
    compare_to_shell_script(PATH_VMLINUX_LZMA);
}

#[test]
fn compare_to_shell_script_vmlinux_lz4() {
    compare_to_shell_script(PATH_VMLINUX_LZ4);
}

#[test]
fn compare_to_shell_script_vmlinux_zstd() {
    compare_to_shell_script(PATH_VMLINUX_ZSTD);
}
