use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
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
