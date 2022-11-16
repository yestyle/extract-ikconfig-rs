use clap::{Arg, Command};
use flate2::bufread::GzDecoder;
use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, Sink, SinkMatch};
use regex::bytes::RegexBuilder;
use std::{
    fs::File,
    io::{self, BufReader, ErrorKind, Read, Seek, SeekFrom},
    str::from_utf8,
};

#[allow(dead_code)]
fn scan_term(file: &mut File, pattern: &str) -> Result<u64, io::Error> {
    let filelen = file.metadata()?.len();
    let mut start = 0;
    let mut offset: u64 = 0;
    for b in file.bytes() {
        if let Ok(b) = b {
            // loop will break when start is equal to pattern.len(),
            // so it's safe to unwrap
            if b == *pattern.as_bytes().get(start).unwrap() {
                start += 1;
                if start == pattern.len() {
                    // let offset point to the start of the pattern
                    offset -= start as u64 - 1;
                    break;
                }
            } else {
                start = 0;
            }
        }
        offset += 1;
    }

    if offset < filelen - pattern.len() as u64 {
        Ok(offset)
    } else {
        Err(io::Error::from(ErrorKind::NotFound))
    }
}

struct Offset<F>(F)
where
    F: FnMut(u64, &[u8]) -> Result<bool, io::Error>;

impl<F> Sink for Offset<F>
where
    F: FnMut(u64, &[u8]) -> Result<bool, io::Error>,
{
    type Error = io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, io::Error> {
        // mat.absolute_bytes_offset() is the offset of the matched line
        // mat.bytes() is the bytes of the matched line
        (self.0)(mat.absolute_byte_offset(), mat.bytes())
    }
}

fn search_pattern(file: &File, pattern: &str) -> Result<u64, io::Error> {
    let matcher = if let Ok(matcher) = RegexMatcher::new(pattern) {
        matcher
    } else {
        return Err(io::Error::from(ErrorKind::InvalidInput));
    };

    let mut offset = 0;
    Searcher::new().search_file(
        &matcher,
        file,
        Offset(|line_offset, bytes| {
            // find pattern within the line and add onto line offset
            // We are guaranteed to find a match, so the unwrap is OK.
            let mymatch = matcher.find(bytes).unwrap().unwrap();
            offset = line_offset + mymatch.start() as u64;
            Ok(true)
        }),
    )?;

    if offset != 0 {
        Ok(offset)
    } else {
        Err(io::Error::from(ErrorKind::NotFound))
    }
}

#[allow(dead_code)]
fn search_regex(file: &File, pattern: &str) -> Result<u64, io::Error> {
    let filelen = file.metadata()?.len() as usize;
    let mut buff = BufReader::new(file);
    let mut bytes = vec![0; 1024];
    let mut offset = 0;
    // Disable Unicode (\u flag) to search arbitrary (non-UTF-8) bytes
    let re = if let Ok(re) = RegexBuilder::new(pattern).unicode(false).build() {
        re
    } else {
        return Err(io::Error::from(ErrorKind::InvalidInput));
    };

    loop {
        match buff.read(&mut bytes) {
            Ok(read) => {
                if read == 0 {
                    break;
                }
                if let Some(m) = re.find(&bytes[..read]) {
                    offset += m.start();
                    break;
                } else {
                    offset += read;
                }
            }
            Err(err) => {
                eprintln!("Failed to read file: {err}");
                return Err(io::Error::from(ErrorKind::InvalidInput));
            }
        }
    }

    if offset < filelen - pattern.len() {
        Ok(offset as u64)
    } else {
        // TODO: search again around the 1KB boundaries
        Err(io::Error::from(ErrorKind::NotFound))
    }
}

fn dump_config_gzip(file: &mut File, offset: u64) {
    if file.seek(SeekFrom::Start(offset)).is_err() {
        eprintln!("Failed to seek to offset {offset}");
        return;
    }

    let mut gz = GzDecoder::new(BufReader::new(file));
    let mut bytes = vec![0; 1024];
    loop {
        match gz.read(&mut bytes) {
            Ok(read) => {
                if read == 0 {
                    return;
                }
                match from_utf8(&bytes[..read]) {
                    Ok(config) => print!("{config}"),
                    Err(err) => {
                        eprintln!("Not UTF-8 content: {err}");
                        return;
                    }
                }
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
    // "1f 8b 08" is the first 3 bytes of gzip header
    let pattern = r"IKCFG_ST\x1f\x8b\x08";

    match search_regex(&file, pattern) {
        Ok(offset) => {
            // Skip "IKCFG_ST" and the rest is config_data.gz
            dump_config_gzip(&mut file, offset + "IKCFG_ST".len() as u64);
        }
        Err(err) => {
            eprintln!("In-kernel config not found. Please ensure the kernel is compiled with CONFIG_IKCONFIG: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::prelude::*;
    use std::fs::File;

    #[test]
    fn compare_searching_methods() {
        let pattern = r"IKCFG_ST";
        let mut file = File::open("tests/data/vmlinux").unwrap();

        let start = Utc::now();
        scan_term(&mut file, pattern).unwrap();
        println!("scan_term: {} ms", (Utc::now() - start).num_milliseconds());

        file.seek(SeekFrom::Start(0)).ok();

        let start = Utc::now();
        search_pattern(&mut file, pattern).unwrap();
        println!(
            "search_pattern: {} ms",
            (Utc::now() - start).num_milliseconds()
        );
    }
}
