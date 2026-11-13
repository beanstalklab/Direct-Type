use std::path::PathBuf;

use viet_cli::corpus;
use viet_core::Config;

fn corpus_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus")
}

/// Every corpus case, paired with the configuration its file name selects.
fn every_case() -> Vec<(Config, corpus::Case)> {
    let mut cases = Vec::new();
    let files = corpus::case_files(&corpus_directory()).expect("corpus directory should load");
    for file in files {
        let configuration = corpus::configuration_from_filename(&file)
            .expect("corpus file name should name a method");
        for case in corpus::read_cases(&file).expect("corpus file should load") {
            cases.push((configuration, case));
        }
    }
    cases
}

/// Upper-case the first letter of every word, the way a sentence starts.
fn title_case(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut at_word_start = true;
    for character in text.chars() {
        if character.is_whitespace() {
            result.push(character);
            at_word_start = true;
            continue;
        }
        if at_word_start {
            result.extend(character.to_uppercase());
        } else {
            result.push(character);
        }
        at_word_start = false;
    }
    result
}

#[test]
fn repository_corpus_passes() {
    let report = corpus::run(&corpus_directory()).expect("corpus should load");

    assert!(
        report.total >= 10,
        "expected at least 10 cases, found {}",
        report.total
    );
    assert!(
        report.failures.is_empty(),
        "corpus failures: {:#?}",
        report.failures
    );
}

#[test]
fn whitespace_ends_a_word() {
    let committed = corpus::commit_text(viet_core::Config::default(), "hello ban");
    assert_eq!(committed, "hello ban");
}

#[test]
fn case_never_changes_a_word() {
    // The corpus is written one case per case, and a user types each of those words
    // in upper case or at the start of a sentence. Deriving the twins here covers
    // every case, including ones added later, instead of keeping a second copy of
    // every file in sync by hand.
    let mut twins = 0;
    for (configuration, case) in every_case() {
        if case.input.chars().any(char::is_lowercase) {
            assert_eq!(
                corpus::commit_text(configuration, &case.input.to_uppercase()),
                case.expected.to_uppercase(),
                "{:?} in upper case",
                case.input
            );
            twins += 1;
        }
        if title_case(&case.input) != case.input {
            assert_eq!(
                corpus::commit_text(configuration, &title_case(&case.input)),
                title_case(&case.expected),
                "{:?} in Title case",
                case.input
            );
            twins += 1;
        }
    }
    // A property that checked nothing would pass silently; the corpus is far past this.
    assert!(twins >= 100, "only {twins} case twins were checked");
}

/// A decomposed (NFD) Vietnamese letter is a base letter plus a combining mark in
/// U+0300..U+036F. The engine composes from the closed table in `unicode.rs`, so no
/// output may contain one; this is the NFC guarantee of spec §2.1 without taking a
/// Unicode dependency.
fn assert_precomposed(text: &str, context: &str) {
    for character in text.chars() {
        assert!(
            !('\u{0300}'..='\u{036F}').contains(&character),
            "{context}: {text:?} contains the combining mark U+{:04X}",
            u32::from(character)
        );
    }
}

#[test]
fn corpus_output_is_precomposed() {
    let cases = every_case();
    assert!(
        cases.len() >= 100,
        "only {} cases were checked",
        cases.len()
    );
    for (configuration, case) in cases {
        assert_precomposed(&case.expected, "an expectation");
        let actual = corpus::commit_text(configuration, &case.input);
        assert_precomposed(&actual, "the output");
        assert_precomposed(
            &corpus::commit_text(configuration, &title_case(&case.input)),
            "the Title-case output",
        );
    }
}
