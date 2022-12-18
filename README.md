# ikconfig

This is a Rust re-implementation of [extract-ikconfig](https://github.com/torvalds/linux/blob/master/scripts/extract-ikconfig) from Linux kernel, to extract the `.config` file from a kernel image.

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

This crate has been published onto [crates.io](https://crates.io/crates/ikconfig), so you can use the following command to install `ikconfig` executable in `~/.cargo/bin` directory:

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

# Tests

Because crates.io has a upload size limit of 10 MiB, the test cases and data are moved to [tests branch](https://github.com/yestyle/extract-ikconfig-rs/tree/tests).

# License

This project is licensed under [GPL-3.0](COPYING) or [MIT license](LICENSE).
