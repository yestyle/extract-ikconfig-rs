use bzip2::bufread::BzDecoder;
use clap::{Arg, Command};
use flate2::bufread::GzDecoder;
use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{Searcher, Sink, SinkMatch};
use lz4_flex::frame::FrameDecoder as Lz4Decoder;
use lzma::LzmaReader;
use regex::bytes::RegexBuilder;
use std::{
    fs::File,
    io::{self, BufReader, ErrorKind, Read, Seek, SeekFrom},
};
use zstd::stream::read::Decoder as ZstdDecoder;

// search pattern:
// IKCFG_ST is the start flag of in-kernel config
// "1f 8b 08" is the first 3 bytes of gzip header
const IKCFG_ST_FLAG_STR: &str = r"IKCFG_ST\x1f\x8b\x08";

// search patterns for compressed header
const MAGIC_NUMBER_GZIP: &str = r"\x1f\x8b\x08";
const MAGIC_NUMBER_XZ: &str = r"\xfd7zXZ\x00";
const MAGIC_NUMBER_BZIP2: &str = r"BZh";
const MAGIC_NUMBER_LZMA: &str = r"\x5d\x00\x00\x00";
const MAGIC_NUMBER_LZOP: &str = r"\x89\x4c\x5a";
const MAGIC_NUMBER_LZ4: &str = r"\x02\x21\x4c\x18";
const MAGIC_NUMBER_ZSTD: &str = r"\x28\xb5\x2f\xfd";

#[allow(dead_code)]
fn search_bytes(file: &mut File, pattern: &[u8]) -> Result<u64, io::Error> {
    let filelen = file.metadata()?.len();
    let mut start = 0;
    let mut offset: u64 = 0;

    file.seek(SeekFrom::Start(0))?;
    for b in file.bytes() {
        if let Ok(b) = b {
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
fn search_ripgrep(file: &mut File, pattern: &str) -> Result<u64, io::Error> {
    // Disable Unicode (\u flag) to search arbitrary (non-UTF-8) bytes
    let matcher = if let Ok(matcher) = RegexMatcherBuilder::new().unicode(false).build(pattern) {
        matcher
    } else {
        return Err(io::Error::from(ErrorKind::InvalidInput));
    };

    let mut offset = 0;
    file.seek(SeekFrom::Start(0))?;
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

    buff.seek(SeekFrom::Start(0))?;
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

fn dump_config_gzip(file: &mut File, offset: u64) -> Result<(), io::Error> {
    // seek to offset before passing into GzDecoder
    file.seek(SeekFrom::Start(offset))?;

    // write the decompressed config text to stdout
    io::copy(&mut GzDecoder::new(BufReader::new(file)), &mut io::stdout()).map(|_| ())
}

fn dump_config(file: &mut File) -> Result<(), io::Error> {
    search_regex(file, IKCFG_ST_FLAG_STR)
        .and_then(|offset| dump_config_gzip(file, offset + "IKCFG_ST".len() as u64))
}

fn gunzip(src: &File, dst: &mut File) -> Result<(), io::Error> {
    io::copy(&mut GzDecoder::new(BufReader::new(src)), dst).map(|_| ())
}

fn unxz(src: &File, dst: &mut File) -> Result<(), io::Error> {
    unlzma(src, dst)
}

fn bunzip2(src: &File, dst: &mut File) -> Result<(), io::Error> {
    io::copy(&mut BzDecoder::new(BufReader::new(src)), dst).map(|_| ())
}

fn unlzma(src: &File, dst: &mut File) -> Result<(), io::Error> {
    io::copy(
        &mut LzmaReader::new_decompressor(BufReader::new(src))
            .map_err(|_| io::Error::from(ErrorKind::InvalidInput))?,
        dst,
    )
    .map(|_| ())
}

fn lzop(_src: &File, _dst: &mut File) -> Result<(), io::Error> {
    Err(io::Error::from(ErrorKind::NotFound))
}

fn unlz4(src: &File, dst: &mut File) -> Result<(), io::Error> {
    // ignore the errors like extract-ikconfig.sh does
    // because lz4 compression used in linux kernel is in legacy frame format,
    // there's no explicit EndMark but implicitly signaled by EOF (End Of File).
    // however, there are more data after the lz4-compressed blocks,
    // so ignore errors here.
    _ = io::copy(&mut Lz4Decoder::new(BufReader::new(src)), dst);
    Ok(())
}

fn unzstd(src: &File, dst: &mut File) -> Result<(), io::Error> {
    // ignore the errors like extract-ikconfig.sh does,
    // otherwise "Unknown frame descriptor" because there is
    // excess data at the end of zstd frame.
    _ = io::copy(&mut ZstdDecoder::new(BufReader::new(src))?, dst);
    Ok(())
}

fn try_decompress<F>(file: &mut File, pattern: &str, decompress: F) -> Result<(), io::Error>
where
    F: Fn(&File, &mut File) -> Result<(), io::Error>,
{
    search_regex(file, pattern).and_then(|offset| {
        // decompress file[offset..] to tempfile to get raw vmlinux
        file.seek(SeekFrom::Start(offset))?;
        let mut dst = tempfile::tempfile()?;
        decompress(file, &mut dst)?;

        // search config_data.gz and dump it in raw vmlinux
        dump_config(&mut dst)
    })
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

    if dump_config(&mut file)
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_GZIP, gunzip))
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_XZ, unxz))
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_BZIP2, bunzip2))
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_LZMA, unlzma))
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_LZOP, lzop))
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_LZ4, unlz4))
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_ZSTD, unzstd))
        .is_err()
    {
        eprintln!(
            "{}: Cannot find kernel config. Please confirm kernel compiled with CONFIG_IKCONFIG.",
            env!("CARGO_BIN_NAME")
        );
    }
}
