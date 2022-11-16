use clap::{Arg, Command};
use flate2::bufread::GzDecoder;
use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{Searcher, Sink, SinkMatch};
use regex::bytes::RegexBuilder;
use std::{
    fs::File,
    io::{self, BufReader, ErrorKind, Read, Seek, SeekFrom},
    str::from_utf8,
};

// search pattern:
// IKCFG_ST is the start flag of in-kernel config
// "1f 8b 08" is the first 3 bytes of gzip header
const IKCFG_ST_FLAG_STR: &str = r"IKCFG_ST\x1f\x8b\x08";

#[allow(dead_code)]
fn search_bytes(file: &mut File, pattern: &[u8]) -> Result<u64, io::Error> {
    let filelen = file.metadata()?.len();
    let mut start = 0;
    let mut offset: u64 = 0;
    for b in file.bytes() {
        if let Ok(b) = b {
            // loop will break when start is equal to pattern.len(),
            // so it's safe to unwrap
            if b == pattern[start] {
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

#[allow(dead_code)]
fn search_ripgrep(file: &File, pattern: &str) -> Result<u64, io::Error> {
    // Disable Unicode (\u flag) to search arbitrary (non-UTF-8) bytes
    let matcher = if let Ok(matcher) = RegexMatcherBuilder::new().unicode(false).build(pattern) {
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

fn search_regex(file: &File, pattern: &str) -> Result<u64, io::Error> {
    let mut buff = BufReader::new(file);
    let mut bytes = vec![0; 1024];
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
                // Note: pattern.len() is the length of the string, not bytes
                if read < pattern.len() {
                    // if remaining bytes is shorter than a pattern,
                    // search again the last length of pattern
                    buff.seek(SeekFrom::End(pattern.len() as i64))?;
                    continue;
                }
                if let Some(m) = re.find(&bytes[..read]) {
                    return Ok(buff.stream_position().unwrap() - (read - m.start()) as u64);
                } else {
                    // overlap the search around the chunk boundaries
                    // in case the pattern locates across the boundary
                    buff.seek(SeekFrom::Current(1 - pattern.len() as i64))?;
                }
            }
            Err(err) => {
                return Err(err);
            }
        }
    }

    Err(io::Error::from(ErrorKind::NotFound))
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

    match search_regex(&file, IKCFG_ST_FLAG_STR) {
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

    const PATH_VMLINUX: &str = "tests/data/vmlinux";
    const IKCFG_ST_FLAG_BYTES: &[u8] = b"IKCFG_ST\x1f\x8b\x08";
    const FLAG_OFFSET_VMLINUX: u64 = 12645664;

    #[test]
    fn test_search_bytes() {
        let mut file = File::open(PATH_VMLINUX).unwrap();
        assert_eq!(
            search_bytes(&mut file, IKCFG_ST_FLAG_BYTES).unwrap(),
            FLAG_OFFSET_VMLINUX
        );
    }

    #[test]
    fn test_search_ripgrep() {
        let mut file = File::open(PATH_VMLINUX).unwrap();
        assert_eq!(
            search_ripgrep(&mut file, IKCFG_ST_FLAG_STR).unwrap(),
            FLAG_OFFSET_VMLINUX
        );
    }

    #[test]
    fn test_search_regex() {
        let mut file = File::open(PATH_VMLINUX).unwrap();
        assert_eq!(
            search_regex(&mut file, IKCFG_ST_FLAG_STR).unwrap(),
            FLAG_OFFSET_VMLINUX
        );
    }

    #[test]
    fn compare_searching_methods() {
        let mut file = File::open(PATH_VMLINUX).unwrap();

        let start = Utc::now();
        search_bytes(&mut file, IKCFG_ST_FLAG_BYTES).unwrap();
        println!(
            "{:15}: {:-5} ms",
            "search_bytes",
            (Utc::now() - start).num_milliseconds()
        );

        file.seek(SeekFrom::Start(0)).ok();

        let start = Utc::now();
        search_ripgrep(&mut file, IKCFG_ST_FLAG_STR).unwrap();
        println!(
            "{:15}: {:-5} ms",
            "search_ripgrep",
            (Utc::now() - start).num_milliseconds()
        );

        file.seek(SeekFrom::Start(0)).ok();

        let start = Utc::now();
        search_regex(&mut file, IKCFG_ST_FLAG_STR).unwrap();
        println!(
            "{:15}: {:-5} ms",
            "search_regex",
            (Utc::now() - start).num_milliseconds()
        );
    }
}
