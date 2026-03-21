use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

mod ast;
mod compiler;
mod lexer;
mod parser;
mod validator;

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

#[derive(Parser)]
#[command(name = "wst", about = "Well Structured Type compiler")]
struct Cli {
    #[arg(short = 'i', long)]
    input: PathBuf,

    #[arg(short = 'f', long, default_value = "json")]
    format: Format,

    /// Output path. Omit to validate only (no output written).
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum Format {
    Json,
    Ts,
    Rust,
}

impl Format {
    fn extension(&self) -> &str {
        match self {
            Format::Json => "json",
            Format::Ts => "ts",
            Format::Rust => "rs",
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.output {
        Some(ref output) => {
            if cli.input.is_dir() {
                process_dir(&cli.input, output, &cli.format)
            } else {
                process_file(&cli.input, output, &cli.format)
            }
        }
        None => {
            if cli.input.is_dir() {
                validate_dir(&cli.input)
            } else {
                let errors = check_file(&cli.input);
                if errors.is_empty() {
                    Ok(())
                } else {
                    for e in &errors {
                        eprintln!("{}", e);
                    }
                    std::process::exit(1);
                }
            }
        }
    }
}

/// Parse and semantically validate a single file. Returns all error messages.
fn check_file(input: &Path) -> Vec<String> {
    match validate_file(input) {
        Err(e) => vec![e.to_string()],
        Ok(ast) => validator::validate(&ast, input),
    }
}

/// Parse a single file for syntax validity. Returns the AST on success.
pub fn validate_file(input: &Path) -> Result<ast::File> {
    let src = std::fs::read_to_string(input)
        .with_context(|| format!("Failed to read {:?}", input))?;
    let tokens = lexer::Lexer::new(&src).tokenize()?;
    parser::Parser::new(tokens).parse_file()
}

/// Validate all `.wst` files under `input`. Prints per-file errors and
/// returns an error if any file fails.
pub fn validate_dir(input: &Path) -> Result<()> {
    use walkdir::WalkDir;

    let mut failed = 0usize;

    for entry in WalkDir::new(input) {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("wst") {
            continue;
        }

        let errors = check_file(path);
        if !errors.is_empty() {
            for e in &errors {
                eprintln!("{}: {}", path.display(), e);
            }
            failed += 1;
        }
    }

    if failed > 0 {
        anyhow::bail!("{} file(s) failed validation", failed);
    }
    Ok(())
}

fn process_dir(input: &Path, output: &Path, format: &Format) -> Result<()> {
    use walkdir::WalkDir;

    for entry in WalkDir::new(input) {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("wst") {
            continue;
        }

        let rel = path.strip_prefix(input)?;
        let out_path = output.join(rel).with_extension(format.extension());

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        process_file(path, &out_path, format)
            .with_context(|| format!("Failed to process {:?}", path))?;
    }

    Ok(())
}

fn process_file(input: &Path, output: &Path, format: &Format) -> Result<()> {
    let ast = validate_file(input)?;
    let file_dir = input.parent().unwrap_or(Path::new("."));

    let result = match format {
        Format::Json => compiler::json::compile(&ast)?,
        Format::Ts => compiler::typescript::compile(&ast, file_dir)?,
        Format::Rust => compiler::rust::compile(&ast, file_dir)?,
    };

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, result)?;

    Ok(())
}
