# ikconfig

This is a Rust re-implementation of [extract-ikconfig] from Linux kernel, to extract the `.config` file from a kernel image.

This will only work when the kernel was compiled with `CONFIG_IKCONFIG`.

# Pre-installation

This crate requires `liblzma` being present in the system before installation and `pkg-config` is used to find `liblzma` and other libraries during the build.

## Arch Linux

```
sudo pacman -S pkgconf xz
```

## Ubuntu

```
sudo apt install pkg-config liblzma-dev
```

Please refer to system manuals for other distributions. You can check if liblzma is installed by running:

```
$ pkg-config --libs liblzma
```

And it should output `-llzma` if `liblzma` is correctly installed.

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

* LZO decompression is unimplemented

I haven't managed to make LZO decompression working yet, so if the kernel is compiled with `CONFIG_KERNEL_LZO`, `ikconfig` won't work. Please use the shell script [extract-ikconfig] from Linux kernel for now.

# License

This project is licensed under [GPL-3.0](COPYING) or [MIT license](LICENSE).



[extract-ikconfig]: https://github.com/torvalds/linux/blob/master/scripts/extract-ikconfig "extract-ikconfig"
[crate-ikconfig]: https://crates.io/crates/ikconfig "ikconfig"
[lz4-legacy-frame]: https://github.com/lz4/lz4/blob/dev/doc/lz4_Frame_format.md#legacy-frame "Legacy frame"
[crate-lz4-flex]: https://crates.io/crates/lz4_flex "lz4_flex"
[yestyle-lz4-flex]: https://github.com/yestyle/lz4_flex "yestyle/lz4_flex"
[self-tests-branch]: https://github.com/yestyle/extract-ikconfig-rs/tree/tests "tests branch"

