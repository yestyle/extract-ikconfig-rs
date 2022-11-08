use clap::{Arg, Command};
use flate2::read::GzDecoder;
use std::{
    fs::File,
    io::{self, BufReader, ErrorKind, Read, Seek, SeekFrom},
    str::from_utf8,
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
                    // let offset point to the start of the pattern
                    offset -= start - 1;
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

fn dump_config_gzip(file: &mut File, offset: usize) {
    if file.seek(SeekFrom::Start(offset as u64)).is_err() {
        eprintln!("Failed to seek to offset {offset}");
        return;
    }

    let mut buf = Vec::new();
    let size = file.read_to_end(&mut buf).unwrap_or_default();
    if size == 0 {
        eprintln!("Failed to read file");
        return;
    }

    let mut gz = GzDecoder::new(BufReader::new(&buf[..]));
    const CHUNK: usize = 1024;
    let mut bytes = vec![0; CHUNK];
    loop {
        match gz.read(&mut bytes) {
            Ok(read) => {
                if read == 0 {
                    return;
                }
                print!("{}", from_utf8(&bytes[..read]).unwrap());
            }
            Err(err) => {
                eprintln!("Failed to deflate the file: {err}");
                return;
            }
        };
    }
}

fn main() {
    let matches = Command::new(env!("CARGO_BIN_NAME"))
        .about("An utility to extract the .config file from a kernel image")
        .arg_required_else_help(true)
        .arg(Arg::new("image").help("kernel image compiled with CONFIG_IKCONFIG"))
        .get_matches();

    // "image" argument is required so could be unwrapped safely
    let image = matches.get_one::<String>("image").unwrap();
    let mut file = match File::open(image) {
        Ok(image) => image,
        Err(err) => {
            eprintln!("Failed to open file {image}: {err}");
            return;
        }
    };

    // search pattern:
    // IKCFG_ST is the start flag of in-kernel config
    // `1f 8b` is the magic number of gzip format
    // `08` is DEFLATE compression method
    let pattern = b"IKCFG_ST\x1f\x8b\x08";

    if let Ok(offset) = scan_term(&mut file, pattern) {
        // Skip "IKCFG_ST" and the rest is config_data.gz
        dump_config_gzip(&mut file, offset + "IKCFG_ST".len());
    } else {
        eprintln!(
            "In-kernel config not found. Please ensure the kernel is compiled with CONFIG_IKCONFIG"
        );
    }
}
