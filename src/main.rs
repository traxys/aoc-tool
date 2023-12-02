use std::{
    collections::BTreeMap, fs::OpenOptions, io::Write, os::unix::process::CommandExt, process,
};

use cargo_metadata::camino::Utf8PathBuf;
use clap::{Parser, ValueEnum};
use color_eyre::eyre::{self, eyre};
use reqwest::header::{self, HeaderValue};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Part {
    #[clap(name = "1")]
    One,
    #[clap(name = "2")]
    Two,
}

#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
struct Args {
    #[arg(short, long, global = true, env = "AOC_YEAR")]
    year: u64,
    #[arg(short, long, global = true, env = "AOC_COOKIE")]
    cookie: Option<String>,
    #[arg(short, long, global = true)]
    day: Option<u64>,
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
    New {
        #[arg(long)]
        no_edit: bool,
        #[arg(long)]
        no_open: bool,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        no_fetch: bool,
    },
    Run {
        #[arg(long)]
        release: bool,
        #[arg(short, long)]
        part: Option<Part>,
        input: Option<Utf8PathBuf>,
    },
    Fetch,
    Open,
    Edit,
}

fn fetch(
    year: u64,
    day: u64,
    input_dir: &Utf8PathBuf,
    cookie: &Option<String>,
) -> color_eyre::Result<()> {
    let Some(cookie) = cookie else {
        eyre::bail!("Must provide cookie to fetch inputs")
    };

    if !input_dir.exists() {
        std::fs::create_dir(input_dir)?;
    }

    let client = reqwest::blocking::Client::new();
    let data = client
        .get(format!("https://adventofcode.com/{year}/day/{day}/input"))
        .header(
            header::COOKIE,
            HeaderValue::from_str(&format!("session={cookie}"))?,
        )
        .send()?
        .bytes()?;

    let mut input_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(input_dir.join(format!("day{day}")))?;

    input_file.write_all(&data)?;

    Ok(())
}

fn open_problem(year: u64, day: u64) -> color_eyre::Result<()> {
    process::Command::new("firefox")
        .arg(format!("https://adventofcode.com/{year}/day/{day}"))
        .spawn()?
        .wait()?;
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let metadata = cargo_metadata::MetadataCommand::new().exec()?;

    let package = metadata
        .root_package()
        .ok_or_else(|| eyre!("Missing a root package"))?;
    let problems: BTreeMap<_, _> = package
        .targets
        .iter()
        .filter_map(|p| {
            p.name
                .strip_prefix("day")
                .map(|d| Ok::<_, color_eyre::Report>((d.parse::<u64>()?, &p.src_path)))
        })
        .collect::<Result<_, _>>()?;
    let workspace_root = &metadata.workspace_root;

    let input_dir = workspace_root.join("inputs");

    let args = Args::parse();
    let CargoCmd::Aoc(command) = args.command;

    match command {
        Commands::New {
            no_edit: create_only,
            force,
            no_fetch,
            no_open,
        } => {
            let template = workspace_root.join("template.rs");

            if !template.exists() {
                eyre::bail!("No template in {workspace_root}");
            }

            let day = args
                .day
                .unwrap_or(problems.last_key_value().map(|(k, _)| *k + 1).unwrap_or(1));

            let day_file = workspace_root.join(format!("src/bin/day{day}.rs"));
            if day_file.exists() && !force {
                eyre::bail!("File {day_file} already exists, not creating");
            }

            std::fs::copy(&template, &day_file)?;

            if !no_fetch {
                fetch(args.year, day, &input_dir, &args.cookie)?;
            }

            if !no_open {
                open_problem(args.year, day)?;
            }

            if !create_only {
                return Err(std::process::Command::new(std::env::var("EDITOR")?)
                    .arg(day_file)
                    .exec()
                    .into());
            }
        }
        Commands::Open => {
            let Some(day) = args.day.or(problems.last_key_value().map(|(k, _)| *k)) else {
                eyre::bail!("No day, can't open anything");
            };

            open_problem(args.year, day)?;
        }
        Commands::Edit => {
            let Some(day) = args.day.or(problems.last_key_value().map(|(k, _)| *k)) else {
                eyre::bail!("No day file, can't edit anything");
            };

            let Some(main_file) = problems.get(&day) else {
                eyre::bail!("No file for day {day}");
            };

            return Err(std::process::Command::new(std::env::var("EDITOR")?)
                .arg(main_file)
                .exec()
                .into());
        }
        Commands::Fetch => {
            fetch(
                args.year,
                args.day
                    .unwrap_or(problems.last_key_value().map(|(k, _)| *k).unwrap_or(1)),
                &input_dir,
                &args.cookie,
            )?;
        }
        Commands::Run {
            release,
            part,
            input,
        } => {
            let mut cargo = process::Command::new(std::env::var("CARGO")?);

            let Some(day) = args.day.or(problems.last_key_value().map(|(k, _)| *k)) else {
                eyre::bail!("No day found");
            };

            let Some(file) = problems.get(&day) else {
                eyre::bail!("Day {day} not implemented");
            };

            let file = std::fs::read_to_string(file)?;

            let part = part.unwrap_or_else(|| {
                if file.contains(r#"todo!("todo part2")"#) {
                    Part::One
                } else {
                    Part::Two
                }
            });
            let part = match part {
                Part::One => "1",
                Part::Two => "2",
            };

            let day = format!("day{day}");

            cargo.args(["run", "--bin", &day]);
            if release {
                cargo.arg("--release");
            }

            cargo
                .args(["--", "--part", part, "--input"])
                .arg(input.unwrap_or_else(|| input_dir.join(day)));

            cargo.status()?;
        }
    }

    Ok(())
}
