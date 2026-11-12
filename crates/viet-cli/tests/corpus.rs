use std::path::PathBuf;

#[test]
fn repository_corpus_passes() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
    let report = viet_cli::corpus::run(&directory).expect("corpus should load");

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
    let committed = viet_cli::corpus::commit_text(viet_core::Config::default(), "hello ban");
    assert_eq!(committed, "hello ban");
}
