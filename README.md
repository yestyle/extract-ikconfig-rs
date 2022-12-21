# ikconfig

This is a Rust re-implementation of [extract-ikconfig] from Linux kernel, to extract the `.config` file from a kernel image.

This will only work when the kernel was compiled with `CONFIG_IKCONFIG`.

# Pre-installation

This crate requires liblzma being present in the system before installation.

## Arch Linux

```
sudo pacman -S xz
```

## Ubuntu

```
sudo apt install liblzma-dev
```

Please refer to system manuals for other distributions. You can check if liblzma is installed by using `pkg-config`:

```
$ pkg-config --libs liblzma
-llzma
```

# Install

This crate has been published onto [crates.io][crate-ikconfig], so you can use the following command to install `ikconfig` executable in `~/.cargo/bin` directory:

```
cargo install ikconfig
```

# Usage

```
ikconfig /boot/vmlinuz-linux
```

The extracted config file will be printed on standard output as the original shell script does. Please use output redirection to save as a file if needed:

```
ikconfig /boot/vmlinuz-linux > .config
```

# Limitations

1. LZO decompression is unimplemented

I haven't managed to make LZO decompression working yet, so if the kernel is compiled with `CONFIG_KERNEL_LZO`, `ikconfig` won't work. Please use the shell script [extract-ikconfig] from Linux kernel for now.

2. LZ4 decompression doesn't work if installing from crates.io

Because Linux kernel uses [legacy frame format][lz4-legacy-frame], but the LZ4 decompression library I used ([lz4_flex][crate-lz4-flex]) doesn't support it yet.

I added the support of decoding legacy frames in [my fork][yestyle-lz4-flex] and used it for ikconfig, but crates.io still pull the original one, which is reasonable.

So for now, if you want to extract `.config` file from a kernel compiled with `CONFIG_KERNEL_LZ4`, you could either use the shell script [extract-ikconfig] from Linux kernel, or clone ikconfig source code and install from there:

```
git clone --recursive https://github.com/yestyle/extract-ikconfig-rs
cd extract-ikconfig-rs
cargo install --path .
```

# Tests

Because crates.io has a upload size limit of 10 MiB, the test cases and data are moved to [tests branch][self-tests-branch].

# License

This project is licensed under [GPL-3.0](COPYING) or [MIT license](LICENSE).



[extract-ikconfig]: https://github.com/torvalds/linux/blob/master/scripts/extract-ikconfig "extract-ikconfig"
[crate-ikconfig]: https://crates.io/crates/ikconfig "ikconfig"
[lz4-legacy-frame]: https://github.com/lz4/lz4/blob/dev/doc/lz4_Frame_format.md#legacy-frame "Legacy frame"
[crate-lz4-flex]: https://crates.io/crates/lz4_flex "lz4_flex"
[yestyle-lz4-flex]: https://github.com/yestyle/lz4_flex "yestyle/lz4_flex"
[self-tests-branch]: https://github.com/yestyle/extract-ikconfig-rs/tree/tests "tests branch"

