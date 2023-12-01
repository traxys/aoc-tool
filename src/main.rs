use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
struct Args {
    #[clap(subcommand)]
    command: CargoCmd,
}

#[derive(clap::Subcommand, Debug)]
enum CargoCmd {
    #[clap(subcommand)]
    Aoc(Commands),
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    New,
    Run,
}

fn main() -> color_eyre::Result<()> {
    let args = Args::parse();
    dbg!(args);

    Ok(())
}
