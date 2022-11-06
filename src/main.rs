use clap::{Arg, Command};
use std::{
    fs::File,
    io::{self, ErrorKind, Read},
};

fn scan_term(file: &mut File, pattern: &[u8]) -> Result<usize, io::Error> {
    let filelen = file.metadata()?.len();
    let mut start = 0;
    let mut offset = 0;
    for b in file.bytes() {
        if let Ok(b) = b {
            if b == pattern[start] {
                start += 1;
                if start == pattern.len() {
                    // let offset point to the start of the term
                    offset -= start - 2;
                    break;
                }
            } else {
                start = 0;
            }
        }
        offset += 1;
    }

    if offset < filelen as usize - pattern.len() {
        Ok(offset)
    } else {
        Err(io::Error::from(ErrorKind::NotFound))
    }
}

fn main() {
    let matches = Command::new(env!("CARGO_BIN_NAME"))
        .about("An utility to extract the .config file from a kernel image")
        .arg_required_else_help(true)
        .arg(Arg::new("image").help("kernel image compiled with CONFIG_IKCONFIG"))
        .get_matches();

    let image = matches.get_one::<String>("image").unwrap();
    let mut file = if let Ok(image) = File::open(image) {
        image
    } else {
        eprintln!("Failed to open file: {image}");
        return;
    };

    // Prepare the search pattern
    let mut pattern = "IKCFG_ST".to_string().as_bytes().to_vec();
    pattern.extend_from_slice(&[0x1f, 0x8b, 0x08]);

    if let Ok(offset) = scan_term(&mut file, &pattern) {
        println!("{offset}");
    } else {
        eprintln!(
            "In-kernel config not found. Please ensure the kernel is compiled with CONFIG_IKCONFIG"
        );
    }
}
