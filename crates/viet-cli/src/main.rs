use std::path::Path;
use std::process::ExitCode;

use viet_cli::corpus;
use viet_core::{Config, Method, ToneStyle};

const USAGE: &str = "\
directtype — multi-language input engine driver

USAGE:
  viet-cli type [--method <telex|simple-telex|vni>] \\
                [--tone-style <old|new>] <text>
  viet-cli check [corpus-directory]

DEFAULTS:
  --method telex
  --tone-style new
  check directory: tests/corpus

EXIT CODES:
  0  success / corpus passes
  1  corpus mismatch
  2  usage error or unreadable corpus
";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        None | Some("-h") | Some("--help") => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
        Some("type") => command_type(&arguments[1..]),
        Some("check") => command_check(arguments.get(1).map(String::as_str)),
        Some(other) => {
            eprintln!("error: unknown command '{other}'");
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn command_type(arguments: &[String]) -> ExitCode {
    let mut method = Method::default();
    let mut tone_style = ToneStyle::default();
    let mut text: Option<&str> = None;

    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--method" => {
                let Some(value) = arguments.get(index + 1) else {
                    return usage_error("--method needs a value");
                };
                match value.as_str() {
                    "telex" => method = Method::Telex,
                    "simple-telex" => method = Method::SimpleTelex,
                    "vni" => method = Method::Vni,
                    other => return usage_error(&format!("unknown method '{other}'")),
                }
                index += 2;
            }
            "--tone-style" => {
                let Some(value) = arguments.get(index + 1) else {
                    return usage_error("--tone-style needs a value");
                };
                match value.as_str() {
                    "new" => tone_style = ToneStyle::New,
                    "old" => tone_style = ToneStyle::Old,
                    other => return usage_error(&format!("unknown tone style '{other}'")),
                }
                index += 2;
            }
            value if value.starts_with("--") => {
                return usage_error(&format!("unknown option '{value}'"));
            }
            value => {
                if text.is_some() {
                    return usage_error("expected exactly one text argument");
                }
                text = Some(value);
                index += 1;
            }
        }
    }

    let Some(text) = text else {
        return usage_error("missing text argument");
    };
    let configuration = Config {
        method,
        tone_style,
        ..Config::default()
    };
    println!("{}", corpus::commit_text(configuration, text));
    ExitCode::SUCCESS
}

fn command_check(directory: Option<&str>) -> ExitCode {
    let directory = directory.unwrap_or("tests/corpus");
    let report = match corpus::run(Path::new(directory)) {
        Ok(report) => report,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(2);
        }
    };

    for failure in &report.failures {
        eprintln!(
            "{}:{}: input {:?}: expected {:?}, got {:?}",
            failure.file.display(),
            failure.line,
            failure.input,
            failure.expected,
            failure.actual
        );
    }

    if report.failures.is_empty() {
        println!("{} cases passed", report.total);
        ExitCode::SUCCESS
    } else {
        eprintln!("{} of {} cases failed", report.failures.len(), report.total);
        ExitCode::from(1)
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    eprint!("{USAGE}");
    ExitCode::from(2)
}
