//! Golden-corpus runner.
//!
//! A corpus file is tab-separated, one case per line, `#` starts a comment:
//!
//! ```text
//! # input<TAB>expected
//! hello<TAB>hello
//! ```
//!
//! The configuration comes from the file name: `telex-new.tsv`,
//! `simple-telex-old.tsv`, `vni-new.tsv`.

use std::fs;
use std::path::{Path, PathBuf};

use viet_core::{Config, Engine, Input, Method, ToneStyle};

/// One corpus line: the keystrokes and the text they must produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub line: usize,
    pub input: String,
    pub expected: String,
}

/// A case whose produced text differs from its expectation.
#[derive(Debug)]
pub struct Failure {
    pub file: PathBuf,
    pub line: usize,
    pub input: String,
    pub expected: String,
    pub actual: String,
}

/// Aggregate result of running a corpus directory.
#[derive(Debug, Default)]
pub struct Report {
    pub total: usize,
    pub failures: Vec<Failure>,
}

/// Run every `.tsv` file in `directory`, in name order so results are stable.
pub fn run(directory: &Path) -> Result<Report, String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;

    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("tsv"))
        .collect();
    files.sort();

    let mut report = Report::default();
    for file in files {
        let configuration = configuration_from_filename(&file)?;
        for case in read_cases(&file)? {
            report.total += 1;
            let actual = commit_text(configuration, &case.input);
            if actual != case.expected {
                report.failures.push(Failure {
                    file: file.clone(),
                    line: case.line,
                    input: case.input,
                    expected: case.expected,
                    actual,
                });
            }
        }
    }
    Ok(report)
}

/// Feed `input` through a fresh engine and return everything committed.
pub fn commit_text(configuration: Config, input: &str) -> String {
    let mut engine = Engine::new(configuration);
    let mut committed = String::new();

    for character in input.chars() {
        if character.is_whitespace() {
            // Whitespace ends the word; the engine never emits it itself.
            engine.feed(Input::Boundary);
            committed.push(character);
        } else {
            committed.push_str(engine.feed(Input::Char(character)).commit);
        }
    }
    engine.feed(Input::Boundary);
    committed
}

fn read_cases(path: &Path) -> Result<Vec<Case>, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;

    let mut cases = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let mut fields = line.split('\t');
        let input = fields.next().unwrap_or_default();
        let Some(expected) = fields.next() else {
            return Err(format!(
                "{}:{line_number}: expected two tab-separated fields",
                path.display()
            ));
        };
        if fields.next().is_some() {
            return Err(format!(
                "{}:{line_number}: expected two tab-separated fields, found more",
                path.display()
            ));
        }

        cases.push(Case {
            line: line_number,
            input: input.to_string(),
            expected: expected.to_string(),
        });
    }
    Ok(cases)
}

fn configuration_from_filename(path: &Path) -> Result<Config, String> {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("{}: file name is not valid UTF-8", path.display()))?;

    let mut parts: Vec<&str> = stem.split('-').collect();
    let Some(tone_style_name) = parts.pop() else {
        return Err(format!(
            "{}: file name must end with a tone style",
            path.display()
        ));
    };
    let method_name = parts.join("-");

    let method = match method_name.as_str() {
        "telex" => Method::Telex,
        "simple-telex" => Method::SimpleTelex,
        "vni" => Method::Vni,
        other => {
            return Err(format!(
                "{}: unknown input method '{other}'",
                path.display()
            ))
        }
    };
    let tone_style = match tone_style_name {
        "new" => ToneStyle::New,
        "old" => ToneStyle::Old,
        other => return Err(format!("{}: unknown tone style '{other}'", path.display())),
    };

    Ok(Config {
        method,
        tone_style,
        ..Config::default()
    })
}
