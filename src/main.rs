use std::path::Path;

use laz::cli::{self, Command};
use laz::lexer::Lexer;
use laz::parser::Parser;
use laz::semantic::TypeChecker;
use laz::codegen::Interpreter;
use laz::formatter::Formatter;
use laz::utils::error::format_error;

fn main() {
    let command = match cli::parse_args() {
        Ok(cmd) => cmd,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    match command {
        Command::Run(config) => run_program(config),
        Command::Format(config) => format_file(config),
    }
}

fn run_program(config: cli::RunConfig) {
    let source = &config.source;

    // Resolve base directory from the source file path (for CSV resolution)
    let base_dir = Path::new(&config.filename)
        .canonicalize()
        .map(|p| p.parent().unwrap_or(Path::new(".")).to_path_buf())
        .unwrap_or_else(|_| {
            Path::new(&config.filename)
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf()
        });

    // Lexing
    let tokens = match Lexer::new(source).tokenize() {
        Ok(tokens) => tokens,
        Err(e) => {
            eprint!("{}", format_error(&e.into(), source, &config.filename));
            std::process::exit(1);
        }
    };

    // Parsing
    let program = match Parser::new(tokens).parse() {
        Ok(program) => program,
        Err(e) => {
            eprint!("{}", format_error(&e.into(), source, &config.filename));
            std::process::exit(1);
        }
    };

    // Semantic analysis
    if let Err(errors) = TypeChecker::check(&program) {
        for e in &errors {
            eprint!("{}", format_error(&e.clone().into(), source, &config.filename));
        }
        std::process::exit(1);
    }

    // Interpretation
    let mut interpreter = Interpreter::new(base_dir);
    if let Err(e) = interpreter.run(&program) {
        eprint!("{}", format_error(&e.into(), source, &config.filename));
        std::process::exit(1);
    }
}

fn format_file(config: cli::FormatConfig) {
    let source = &config.source;

    // Lex with comments
    let (tokens, comments) = match Lexer::new(source).tokenize_with_comments() {
        Ok(result) => result,
        Err(e) => {
            eprint!("{}", format_error(&e.into(), source, &config.filename));
            std::process::exit(1);
        }
    };

    // Parse
    let program = match Parser::new(tokens).parse() {
        Ok(program) => program,
        Err(e) => {
            eprint!("{}", format_error(&e.into(), source, &config.filename));
            std::process::exit(1);
        }
    };

    // Format
    let formatted = Formatter::new(comments).format(&program);

    if config.write_in_place {
        std::fs::write(&config.filename, &formatted).unwrap_or_else(|e| {
            eprintln!("Error writing '{}': {}", config.filename, e);
            std::process::exit(1);
        });
        println!("Formatted: {}", config.filename);
    } else {
        print!("{}", formatted);
    }
}
