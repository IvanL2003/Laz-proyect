use std::env;
use std::fs;

pub enum Command {
    Run(RunConfig),
    Format(FormatConfig),
}

pub struct RunConfig {
    pub filename: String,
    pub source: String,
}

pub struct FormatConfig {
    pub filename: String,
    pub source: String,
    pub write_in_place: bool,
}

pub fn parse_args() -> Result<Command, String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err(format!(
            "Usage: {} <filename.lz>\n       {} fmt <filename.lz> [--write]\n       {} --help",
            args[0], args[0], args[0]
        ));
    }

    match args[1].as_str() {
        "--help" | "-h" => {
            println!("Laz v{}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("Usage:");
            println!("  {} <filename.lz>                Run a .lz program", args[0]);
            println!("  {} fmt <filename.lz>            Format and print to stdout", args[0]);
            println!("  {} fmt --write <filename.lz>    Format and overwrite file", args[0]);
            println!();
            println!("Options:");
            println!("  --help, -h       Show this help message");
            println!("  --version, -v    Show version");
            std::process::exit(0);
        }
        "--version" | "-v" => {
            println!("Laz v{}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        "fmt" => {
            if args.len() < 3 {
                return Err(format!(
                    "Usage: {} fmt <filename.lz> [--write]",
                    args[0]
                ));
            }

            let write_in_place = args.iter().any(|a| a == "--write");

            // Find the filename (first arg after "fmt" that's not a flag)
            let filename = args[2..]
                .iter()
                .find(|a| !a.starts_with("--"))
                .ok_or_else(|| format!("Usage: {} fmt <filename.lz> [--write]", args[0]))?
                .clone();

            let source = fs::read_to_string(&filename)
                .map_err(|e| format!("Error reading '{}': {}", filename, e))?;

            Ok(Command::Format(FormatConfig {
                filename,
                source,
                write_in_place,
            }))
        }
        _ => {
            let filename = &args[1];
            let source = fs::read_to_string(filename)
                .map_err(|e| format!("Error reading '{}': {}", filename, e))?;

            Ok(Command::Run(RunConfig {
                filename: filename.clone(),
                source,
            }))
        }
    }
}
