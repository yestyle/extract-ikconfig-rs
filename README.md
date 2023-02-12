# ikconfig

This is a Rust re-implementation of [extract-ikconfig] from Linux kernel, to extract the `.config` file from a kernel image.

This will only work when the kernel was compiled with `CONFIG_IKCONFIG`, which is enabled on Arch Linux by default but not on Ubuntu.

It supports all 7 compression algorithms in Linux kernel:

  * `CONFIG_KERNEL_GZIP`
  * `CONFIG_KERNEL_BZIP2`
  * `CONFIG_KERNEL_LZMA`
  * `CONFIG_KERNEL_XZ`
  * `CONFIG_KERNEL_LZO`
  * `CONFIG_KERNEL_LZ4`
  * `CONFIG_KERNEL_ZSTD`

# Prerequisites

This crate requires `liblzma` being present in the system before installation and `pkg-config` is used to find `liblzma` and other libraries during the build.

## Arch Linux

```
sudo pacman -S pkgconf xz
```

## Ubuntu

```
sudo apt install pkg-config liblzma-dev
```

Please refer to system manuals for other distributions. You can check if `liblzma` is installed by running:

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
ikconfig <path_of_kernel_image>
```

The extracted config file will be printed on standard output as the original shell script does. Please use output redirection to save as a file if needed, e.g.:

```
ikconfig /boot/vmlinuz-linux > .config
```

# Tests

The integration tests in this repository will compare the execution time of `ikconfig` and [extract-ikconfig] shell script.
The latter uses the commands on system to accomplish corresponding decompression, most of which are pre-installed except
[lzop(1)][man-lzop], so you might need to install it before running `cargo test`.

## Arch Linux

```
sudo pacman -S lzop
```

## Ubuntu

```
sudo apt install lzop
```

# License

This project is licensed under [GPL-3.0](COPYING) or [MIT license](LICENSE).



[extract-ikconfig]: https://github.com/torvalds/linux/blob/master/scripts/extract-ikconfig "extract-ikconfig"
[crate-ikconfig]: https://crates.io/crates/ikconfig "ikconfig"
[man-lzop]: https://linux.die.net/man/1/lzop "lzop(1)"

