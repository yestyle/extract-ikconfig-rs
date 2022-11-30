# Extract .config file from kernel image

This is a Rust re-implementation of [extract-ikconfig](https://github.com/torvalds/linux/blob/master/scripts/extract-ikconfig) from Linux kernel, to extract the .config file from a kernel image.

This will only work when the kernel was compiled with `CONFIG_IKCONFIG`.

# Install

The following command will install `ikconfig` executable in `~/.cargo/bin` directory:

```
cargo install --path .
```

# Usage

```
ikconfig /boot/vmlinuz-linux
```

The extracted config file will be printed on standard output as the original shell script does. Use redirection to save as a file if needed.
