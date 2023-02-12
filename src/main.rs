use argh::FromArgs;
use byteorder::{BigEndian, ReadBytesExt};
use bzip2::bufread::BzDecoder;
use flate2::bufread::GzDecoder;
#[cfg(test)]
use grep_matcher::Matcher;
#[cfg(test)]
use grep_regex::RegexMatcherBuilder;
#[cfg(test)]
use grep_searcher::{Searcher, Sink, SinkMatch};
use lz4_flex::frame::FrameDecoder as Lz4Decoder;
use lzma::LzmaReader;
use regex::bytes::RegexBuilder;
use std::{
    fs::File,
    io::{self, BufReader, ErrorKind, Read, Seek, SeekFrom, Write},
    mem::size_of_val,
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
const MAGIC_NUMBER_LZO: &str = r"\x89\x4c\x5a";
const MAGIC_NUMBER_LZ4: &str = r"\x02\x21\x4c\x18";
const MAGIC_NUMBER_ZSTD: &str = r"\x28\xb5\x2f\xfd";

#[cfg(test)]
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

#[cfg(test)]
struct Offset<F>(F)
where
    F: FnMut(u64, &[u8]) -> Result<bool, io::Error>;

#[cfg(test)]
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

#[cfg(test)]
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

fn f_read8(buf: &mut BufReader<&File>) -> u8 {
    buf.read_u8().unwrap()
}

fn f_read16(buf: &mut BufReader<&File>) -> u16 {
    buf.read_u16::<BigEndian>().unwrap()
}

fn f_read32(buf: &mut BufReader<&File>) -> u32 {
    buf.read_u32::<BigEndian>().unwrap()
}

fn unlzo(src: &File, dst: &mut File) -> Result<(), io::Error> {
    const LZOP_MAGIC: &[u8] = &[0x89, 0x4c, 0x5a, 0x4f, 0x00, 0x0d, 0x0a, 0x1a, 0x0a];
    const MAX_BLOCK_SIZE: usize = 64 * 1024 * 1024;
    const BLOCK_SIZE: usize = 256 * 1024;
    const F_ADLER32_D: u32 = 0x00000001;
    const F_ADLER32_C: u32 = 0x00000002;
    const F_STDIN: u32 = 0x00000004;
    const F_H_EXTRA_FIELD: u32 = 0x00000040;
    const F_CRC32_D: u32 = 0x00000100;
    const F_CRC32_C: u32 = 0x00000200;
    const F_H_FILTER: u32 = 0x00000800;

    let mut buf = BufReader::new(src);
    let mut magic = vec![0u8; size_of_val(LZOP_MAGIC)];

    if buf.read_exact(&mut magic).is_ok() && magic == LZOP_MAGIC {
        let lzo = minilzo_rs::LZO::init().unwrap();

        let mut _version_needed = 0x0900;

        let version = f_read16(&mut buf);
        if version < 0x0900 {
            return Err(io::Error::from(ErrorKind::InvalidInput));
        }

        let _lib_version = f_read16(&mut buf);
        if version >= 0x0940 {
            _version_needed = f_read16(&mut buf);
            if _version_needed > 0x1040 || _version_needed < 0x0900 {
                return Err(io::Error::from(ErrorKind::InvalidInput));
            }
        }

        let _method = f_read8(&mut buf);
        if version >= 0x0940 {
            let _level = f_read8(&mut buf);
        }
        let flags = f_read32(&mut buf);
        if flags & F_H_FILTER != 0 {
            let _filter = f_read32(&mut buf);
        }
        let mut _mode = f_read32(&mut buf);
        if flags & F_STDIN != 0 {
            _mode = 0;
        }
        let _mtime_low = f_read32(&mut buf);
        if version >= 0x0940 {
            let _mtime_high = f_read32(&mut buf);
        }

        let len = f_read8(&mut buf) as usize;
        if len > 0 {
            let mut name = vec![0u8; len];
            buf.read_exact(&mut name).ok();
        }

        let _header_checksum = f_read32(&mut buf);

        if flags & F_H_EXTRA_FIELD != 0 {
            let extra_field_len = f_read32(&mut buf) as usize;
            let mut extra_field_data = vec![0u8; extra_field_len];
            buf.read_exact(&mut extra_field_data).ok();
            let _extra_field_checksum = f_read32(&mut buf);
        }

        loop {
            // read uncompressed block size
            let dst_len = f_read32(&mut buf) as usize;

            // exit if last block
            if dst_len == 0 {
                break;
            }

            // error if split file
            if dst_len == 0xFFFFFFFF {
                eprintln!("this file is a split lzop file");
                return Err(io::Error::from(ErrorKind::InvalidInput));
            }

            if dst_len > MAX_BLOCK_SIZE {
                eprintln!("lzop file corrupted");
                return Err(io::Error::from(ErrorKind::InvalidInput));
            }

            // read compressed block size
            let src_len = f_read32(&mut buf) as usize;
            if src_len <= 0 || src_len > dst_len {
                eprintln!("lzop file corrupted");
                return Err(io::Error::from(ErrorKind::InvalidInput));
            }

            if dst_len > BLOCK_SIZE {
                eprintln!("block size too small");
                return Err(io::Error::from(ErrorKind::InvalidInput));
            }

            if flags & F_ADLER32_D != 0 {
                let _d_adler32 = f_read32(&mut buf);
            }
            if flags & F_CRC32_D != 0 {
                let _d_crc32 = f_read32(&mut buf);
            }
            if flags & F_ADLER32_C != 0 {
                if src_len < dst_len {
                    let _c_adler32 = f_read32(&mut buf);
                } else {
                    assert!(flags & F_ADLER32_D != 0);
                }
            }
            if flags & F_CRC32_C != 0 {
                if src_len < dst_len {
                    let _c_crc32 = f_read32(&mut buf);
                } else {
                    assert!(flags & F_CRC32_D != 0);
                }
            }

            // read the block
            let mut src_data = vec![0u8; src_len];
            buf.read_exact(&mut src_data)?;

            if src_len < dst_len {
                // decompress
                if let Ok(dst_data) = lzo.decompress_safe(&src_data, dst_len) {
                    if dst_data.len() == dst_len {
                        dst.write_all(&dst_data)?;
                    } else {
                        eprintln!("compressed data violation");
                        return Err(io::Error::from(ErrorKind::InvalidInput));
                    }
                } else {
                    eprintln!("compressed data violation");
                    return Err(io::Error::from(ErrorKind::InvalidInput));
                }
            } else {
                // uncompressed block
                dst.write_all(&src_data)?;
            }
        }

        Ok(())
    } else {
        Err(io::Error::from(ErrorKind::NotFound))
    }
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

        // search config_data.gz in raw vmlinux and dump it
        dump_config(&mut dst)
    })
}

#[derive(FromArgs)]
#[argh(description = "An utility to extract the .config file from a kernel image")]
struct Args {
    #[argh(positional, description = "kernel image compiled with CONFIG_IKCONFIG")]
    image: String,
}

fn main() {
    let args: Args = argh::from_env();
    let mut file = match File::open(&args.image) {
        Ok(image) => image,
        Err(err) => {
            eprintln!("Failed to open file {}: {err}", &args.image);
            return;
        }
    };

    if dump_config(&mut file)
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_GZIP, gunzip))
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_XZ, unxz))
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_BZIP2, bunzip2))
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_LZMA, unlzma))
        .or_else(|_| try_decompress(&mut file, MAGIC_NUMBER_LZO, unlzo))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use time::Instant;

    const PATH_VMLINUX_RAW: &str = "tests/data/vmlinux";
    const IKCFG_ST_FLAG_BYTES: &[u8] = b"IKCFG_ST\x1f\x8b\x08";
    const PATTERN_OFFSET_VMLINUX_RAW: u64 = 12645664;

    const PATH_VMLINUX_GZIP: &str = "tests/data/vmlinux.gz";
    const MAGIC_NUMBER_GZIP: &[u8] = b"\x1f\x8b\x08";
    const PATTERN_OFFSET_VMLINUX_GZIP: u64 = 16063;

    const PATH_VMLINUX_XZ: &str = "tests/data/vmlinux.xz";
    const MAGIC_NUMBER_XZ: &[u8] = b"\xfd7zXZ\x00";
    const PATTERN_OFFSET_VMLINUX_XZ: u64 = 16063;

    const PATH_VMLINUX_BZIP2: &str = "tests/data/vmlinux.bz2";
    const MAGIC_NUMBER_BZIP2: &[u8] = b"BZh";
    const PATTERN_OFFSET_VMLINUX_BZIP2: u64 = 16063;

    const PATH_VMLINUX_LZMA: &str = "tests/data/vmlinux.lzma";
    const MAGIC_NUMBER_LZMA: &[u8] = b"\x5d\x00\x00\x00";
    const PATTERN_OFFSET_VMLINUX_LZMA: u64 = 16063;

    const PATH_VMLINUX_LZ4: &str = "tests/data/vmlinux.lz4";
    const MAGIC_NUMBER_LZ4: &[u8] = b"\x02\x21\x4c\x18";
    const PATTERN_OFFSET_VMLINUX_LZ4: u64 = 16063;

    const PATH_VMLINUX_ZSTD: &str = "tests/data/vmlinux.zst";
    const MAGIC_NUMBER_ZSTD: &[u8] = b"\x28\xb5\x2f\xfd";
    const PATTERN_OFFSET_VMLINUX_ZSTD: u64 = 16063;

    #[test]
    fn test_search_bytes() {
        let mut file = File::open(PATH_VMLINUX_RAW).unwrap();
        assert_eq!(
            search_bytes(&mut file, IKCFG_ST_FLAG_BYTES).unwrap(),
            PATTERN_OFFSET_VMLINUX_RAW
        );

        let mut file = File::open(PATH_VMLINUX_GZIP).unwrap();
        assert_eq!(
            search_bytes(&mut file, MAGIC_NUMBER_GZIP).unwrap(),
            PATTERN_OFFSET_VMLINUX_GZIP
        );

        let mut file = File::open(PATH_VMLINUX_XZ).unwrap();
        assert_eq!(
            search_bytes(&mut file, MAGIC_NUMBER_XZ).unwrap(),
            PATTERN_OFFSET_VMLINUX_XZ
        );

        let mut file = File::open(PATH_VMLINUX_BZIP2).unwrap();
        assert_eq!(
            search_bytes(&mut file, MAGIC_NUMBER_BZIP2).unwrap(),
            PATTERN_OFFSET_VMLINUX_BZIP2
        );

        let mut file = File::open(PATH_VMLINUX_LZMA).unwrap();
        assert_eq!(
            search_bytes(&mut file, MAGIC_NUMBER_LZMA).unwrap(),
            PATTERN_OFFSET_VMLINUX_LZMA
        );

        let mut file = File::open(PATH_VMLINUX_LZ4).unwrap();
        assert_eq!(
            search_bytes(&mut file, MAGIC_NUMBER_LZ4).unwrap(),
            PATTERN_OFFSET_VMLINUX_LZ4
        );

        let mut file = File::open(PATH_VMLINUX_ZSTD).unwrap();
        assert_eq!(
            search_bytes(&mut file, MAGIC_NUMBER_ZSTD).unwrap(),
            PATTERN_OFFSET_VMLINUX_ZSTD
        );
    }

    #[test]
    fn test_search_ripgrep() {
        let mut file = File::open(PATH_VMLINUX_RAW).unwrap();
        assert_eq!(
            search_ripgrep(&mut file, IKCFG_ST_FLAG_STR).unwrap(),
            PATTERN_OFFSET_VMLINUX_RAW
        );

        let mut file = File::open(PATH_VMLINUX_GZIP).unwrap();
        assert_eq!(
            search_ripgrep(&mut file, super::MAGIC_NUMBER_GZIP).unwrap(),
            PATTERN_OFFSET_VMLINUX_GZIP
        );

        // TODO: similar to zstd below
        // let mut file = File::open(PATH_VMLINUX_XZ).unwrap();
        // assert_eq!(
        //     search_ripgrep(&mut file, super::MAGIC_NUMBER_XZ).unwrap(),
        //     PATTERN_OFFSET_VMLINUX_XZ
        // );

        let mut file = File::open(PATH_VMLINUX_BZIP2).unwrap();
        assert_eq!(
            search_ripgrep(&mut file, super::MAGIC_NUMBER_BZIP2).unwrap(),
            PATTERN_OFFSET_VMLINUX_BZIP2
        );

        let mut file = File::open(PATH_VMLINUX_LZMA).unwrap();
        assert_eq!(
            search_ripgrep(&mut file, super::MAGIC_NUMBER_LZMA).unwrap(),
            PATTERN_OFFSET_VMLINUX_LZMA
        );

        // TODO: similar to zstd below
        // let mut file = File::open(PATH_VMLINUX_LZ4).unwrap();
        // assert_eq!(
        //     search_ripgrep(&mut file, super::MAGIC_NUMBER_LZ4).unwrap(),
        //     PATTERN_OFFSET_VMLINUX_LZ4
        // );

        // TODO: fix this test case
        // There are multiple matches at offset 17613, 10991505, 10991721,
        // but search_ripgrep() misses the first match but catches the second.
        // let mut file = File::open(PATH_VMLINUX_ZSTD).unwrap();
        // assert_eq!(
        //     search_ripgrep(&mut file, super::MAGIC_NUMBER_ZSTD).unwrap(),
        //     PATTERN_OFFSET_VMLINUX_ZSTD
        // );
    }

    #[test]
    fn test_search_regex() {
        let mut file = File::open(PATH_VMLINUX_RAW).unwrap();
        assert_eq!(
            search_regex(&mut file, IKCFG_ST_FLAG_STR).unwrap(),
            PATTERN_OFFSET_VMLINUX_RAW
        );

        let mut file = File::open(PATH_VMLINUX_GZIP).unwrap();
        assert_eq!(
            search_regex(&mut file, super::MAGIC_NUMBER_GZIP).unwrap(),
            PATTERN_OFFSET_VMLINUX_GZIP
        );

        let mut file = File::open(PATH_VMLINUX_XZ).unwrap();
        assert_eq!(
            search_regex(&mut file, super::MAGIC_NUMBER_XZ).unwrap(),
            PATTERN_OFFSET_VMLINUX_XZ
        );

        let mut file = File::open(PATH_VMLINUX_BZIP2).unwrap();
        assert_eq!(
            search_regex(&mut file, super::MAGIC_NUMBER_BZIP2).unwrap(),
            PATTERN_OFFSET_VMLINUX_BZIP2
        );

        let mut file = File::open(PATH_VMLINUX_LZMA).unwrap();
        assert_eq!(
            search_regex(&mut file, super::MAGIC_NUMBER_LZMA).unwrap(),
            PATTERN_OFFSET_VMLINUX_LZMA
        );

        let mut file = File::open(PATH_VMLINUX_LZ4).unwrap();
        assert_eq!(
            search_regex(&mut file, super::MAGIC_NUMBER_LZ4).unwrap(),
            PATTERN_OFFSET_VMLINUX_LZ4
        );

        let mut file = File::open(PATH_VMLINUX_ZSTD).unwrap();
        assert_eq!(
            search_regex(&mut file, super::MAGIC_NUMBER_ZSTD).unwrap(),
            PATTERN_OFFSET_VMLINUX_ZSTD
        );
    }

    fn compare_searching_vmlinux(path: &str, bytes: &[u8], pattern: &str) {
        println!("Searching {}", path);
        let mut file = File::open(path).unwrap();

        let instant = Instant::now();
        search_bytes(&mut file, bytes).unwrap();
        println!(
            "{:15}: {:-10} us",
            "search_bytes",
            instant.elapsed().whole_microseconds()
        );

        let instant = Instant::now();
        search_ripgrep(&mut file, pattern).unwrap();
        println!(
            "{:15}: {:-10} us",
            "search_ripgrep",
            instant.elapsed().whole_microseconds()
        );

        let instant = Instant::now();
        search_regex(&mut file, pattern).unwrap();
        println!(
            "{:15}: {:-10} us",
            "search_regex",
            instant.elapsed().whole_microseconds()
        );
    }

    #[test]
    fn compare_searching_vmlinux_raw() {
        compare_searching_vmlinux(PATH_VMLINUX_RAW, IKCFG_ST_FLAG_BYTES, IKCFG_ST_FLAG_STR);
    }

    #[test]
    fn compare_searching_vmlinux_gzip() {
        compare_searching_vmlinux(
            PATH_VMLINUX_GZIP,
            MAGIC_NUMBER_GZIP,
            super::MAGIC_NUMBER_GZIP,
        );
    }

    #[test]
    fn compare_searching_vmlinux_xz() {
        compare_searching_vmlinux(PATH_VMLINUX_XZ, MAGIC_NUMBER_XZ, super::MAGIC_NUMBER_XZ);
    }

    #[test]
    fn compare_searching_vmlinux_bzip2() {
        compare_searching_vmlinux(
            PATH_VMLINUX_BZIP2,
            MAGIC_NUMBER_BZIP2,
            super::MAGIC_NUMBER_BZIP2,
        );
    }

    #[test]
    fn compare_searching_vmlinux_lzma() {
        compare_searching_vmlinux(
            PATH_VMLINUX_LZMA,
            MAGIC_NUMBER_LZMA,
            super::MAGIC_NUMBER_LZMA,
        );
    }

    #[test]
    fn compare_searching_vmlinux_lz4() {
        compare_searching_vmlinux(PATH_VMLINUX_LZ4, MAGIC_NUMBER_LZ4, super::MAGIC_NUMBER_LZ4);
    }

    #[test]
    fn compare_searching_vmlinux_zstd() {
        compare_searching_vmlinux(
            PATH_VMLINUX_ZSTD,
            MAGIC_NUMBER_ZSTD,
            super::MAGIC_NUMBER_ZSTD,
        );
    }

    fn test_decompress<F>(path: &str, decompress: F)
    where
        F: Fn(&File, &mut File) -> Result<(), io::Error>,
    {
        let src = File::open(path).unwrap();
        let mut dst = tempfile::tempfile().unwrap();

        assert!(decompress(&src, &mut dst).is_ok());

        dst.seek(SeekFrom::Start(0)).unwrap();
        let mut config = File::open("tests/data/config").unwrap();

        let mut expected = String::new();
        let mut decompressed = String::new();
        assert_eq!(
            config.read_to_string(&mut expected).unwrap(),
            dst.read_to_string(&mut decompressed).unwrap()
        );
        assert_eq!(expected, decompressed);
    }

    #[test]
    fn test_decompress_gzip() {
        test_decompress("tests/data/config.gz", gunzip);
    }

    #[test]
    fn test_decompress_xz() {
        test_decompress("tests/data/config.xz", unxz);
    }

    #[test]
    fn test_decompress_bzip2() {
        test_decompress("tests/data/config.bz2", bunzip2);
    }

    #[test]
    fn test_decompress_lzma() {
        test_decompress("tests/data/config.lzma", unlzma);
    }

    #[test]
    fn test_decompress_lzo() {
        test_decompress("tests/data/config.lzo", unlzo);
    }

    #[test]
    fn test_decompress_lz4() {
        test_decompress("tests/data/config.lz4", unlz4);
    }

    #[test]
    fn test_decompress_zstd() {
        test_decompress("tests/data/config.zst", unzstd);
    }
}
