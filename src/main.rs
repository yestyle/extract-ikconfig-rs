use clap::{Arg, Command};
use flate2::bufread::GzDecoder;
use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, Sink, SinkMatch};
use std::{
    fs::File,
    io::{self, BufReader, ErrorKind, Read, Seek, SeekFrom},
    str::from_utf8,
};

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

fn search_pattern(file: &File, pattern: &[u8]) -> Result<usize, io::Error> {
    let matcher = RegexMatcher::new(from_utf8(pattern).unwrap()).unwrap();
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
        Ok(offset as usize)
    } else {
        Err(io::Error::from(ErrorKind::NotFound))
    }
}

fn dump_config_gzip(file: &mut File, offset: usize) {
    if file.seek(SeekFrom::Start(offset as u64)).is_err() {
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
    let pattern = b"IKCFG_ST";

    if let Ok(offset) = search_pattern(&file, pattern) {
        // Skip "IKCFG_ST" and the rest is config_data.gz
        dump_config_gzip(&mut file, offset + "IKCFG_ST".len());
    } else {
        eprintln!(
            "In-kernel config not found. Please ensure the kernel is compiled with CONFIG_IKCONFIG"
        );
    }
}
