//! Golden-corpus runner.
//!
//! A corpus file is tab-separated, one case per line, `#` starts a comment:
//!
//! ```text
//! # input<TAB>expected
//! hello<TAB>hello
//! ```
//!
//! The file name selects the configuration: the first segment names the method
//! (`telex`, `simple-telex`, `vni`), a trailing `old` or `new` names the tone
//! style, and anything between them is a topic that the runner ignores. So
//! `tests/corpus/telex-letters.tsv` runs Telex with the default tone style, and
//! `tests/corpus/vni-new.tsv` runs VNI with the new style.

use std::fs;
use std::path::{Path, PathBuf};

use viet_core::{Action, Config, Engine, Input, Method, ToneStyle};

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
            // Whitespace ends the word, and `auto_restore` may give the keystrokes
            // back: that rewrite has to land before the whitespace itself.
            apply(&mut committed, engine.feed(Input::Boundary));
            committed.push(character);
        } else {
            // The action may rewrite text that is already on screen, so the
            // runner has to honour `backspaces` the way a frontend would.
            apply(&mut committed, engine.feed(Input::Char(character)));
        }
    }
    apply(&mut committed, engine.feed(Input::Boundary));
    committed
}

/// Make `committed` look like the application after performing `action`.
fn apply(committed: &mut String, action: Action<'_>) {
    for _ in 0..action.backspaces {
        committed.pop();
    }
    committed.push_str(action.commit);
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
    let tone_style = match parts.last().copied() {
        Some("old") => {
            parts.pop();
            ToneStyle::Old
        }
        Some("new") => {
            parts.pop();
            ToneStyle::New
        }
        // Letter and tone rules of both styles are identical except for `oa/oe/uy`
        // placement, so a topic file needs no style suffix to be unambiguous.
        _ => ToneStyle::default(),
    };
    let name = parts.join("-");

    let method = if name == "simple-telex" || name.starts_with("simple-telex-") {
        Method::SimpleTelex
    } else if name == "telex" || name.starts_with("telex-") {
        Method::Telex
    } else if name == "vni" || name.starts_with("vni-") {
        Method::Vni
    } else {
        return Err(format!(
            "{}: file name must start with a method (telex, simple-telex, vni), found '{name}'",
            path.display()
        ));
    };

    Ok(Config {
        method,
        tone_style,
        ..Config::default()
    })
}
