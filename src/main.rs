use clap::{Arg, Command};

fn main() {
    let matches = Command::new(env!("CARGO_BIN_NAME"))
        .about("An utility to extract the .config file from a kernel image")
        .arg_required_else_help(true)
        .arg(Arg::new("image").help("kernel image compiled with CONFIG_IKCONFIG"))
        .get_matches();

    let image = matches.get_one::<String>("image").unwrap();
    println!("{image}");
}
