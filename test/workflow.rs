use std::{fs, process::Command};
#[test]
fn two_shells_live_recipe_and_frozen_review() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let docs = root.join("sources");
    fs::create_dir(&docs).unwrap();
    fs::write(docs.join("draft-6.md"), "six").unwrap();
    fs::write(docs.join("draft-5.md"), "\u{feff}café").unwrap();
    fs::write(docs.join("glossary.md"), "terms").unwrap();
    let call = |cwd: &std::path::Path, args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_clyphord"))
            .current_dir(cwd)
            .env("CLYPHORD_DB", root.join("state.db"))
            .args(args)
            .output()
            .unwrap()
    };
    let ok = |cwd: &std::path::Path, args: &[&str]| {
        let out = call(cwd, args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    };
    let pattern = if cfg!(windows) {
        r"sources\draft*.md"
    } else {
        "sources/draft*.md"
    };
    ok(root, &["add", "review", pattern, "sources/glossary.md"]);
    ok(root, &["text", "review", "Review both drafts"]);
    let expected="## draft-5.md\n\ncafé\n\n## draft-6.md\n\nsix\n\n## glossary.md\n\nterms\n\nReview both drafts";
    assert_eq!(ok(&docs, &["show", "review"]), expected);
    assert!(ok(root, &["--session", "other", "list"]).is_empty());
    assert!(!call(
        root,
        &["add", "review", "sources/glossary.md", "missing.txt"]
    )
    .status
    .success());
    assert_eq!(ok(root, &["show", "review"]), expected);
    ok(root, &["template", "save", "critique", "review"]);
    let id = ok(root, &["snapshot", "save", "review"]);
    assert!(ok(root, &["snapshot", "list"]).contains("review"));
    let sources = ok(root, &["snapshot", "sources", id.trim()]);
    assert_eq!(sources.lines().count(), 3);
    assert!(sources.lines().next().unwrap().ends_with("draft-5.md"));
    fs::write(docs.join("draft-6.md"), "revised").unwrap();
    assert!(ok(
        &docs,
        &["--session", "other", "template", "run", "critique"]
    )
    .contains("revised"));
    assert_eq!(ok(root, &["snapshot", "show", id.trim()]), expected);
    ok(root, &["snapshot", "restore", id.trim(), "frozen"]);
    assert_eq!(ok(root, &["show", "frozen"]), expected);
    assert!(!call(root, &["snapshot", "restore", id.trim(), "frozen"])
        .status
        .success());
    ok(root, &["save", "frozen", "export.md"]);
    assert!(!call(root, &["save", "review", "export.md"])
        .status
        .success());
    assert_eq!(
        fs::read_to_string(root.join("export.md")).unwrap(),
        expected
    );
    fs::remove_file(docs.join("glossary.md")).unwrap();
    assert!(!call(root, &["show", "review"]).status.success());
}

#[test]
fn publish_writes_once_after_all_inputs_succeed() {
    use clyphord::{publish_buffer, Entry};
    use std::cell::{Cell, RefCell};
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("notes.md");
    fs::write(&path, "café").unwrap();
    let entries = [
        Entry::new("literal", "Review".into()),
        Entry::new("file", path.display().to_string()),
    ];
    let writes = Cell::new(0);
    let clipboard = RefCell::new(String::from("old clipboard"));
    let write = |doc: &clyphord::Rendered| {
        writes.set(writes.get() + 1);
        clipboard.replace(doc.text.clone());
        Ok(())
    };
    for _ in 0..2 {
        publish_buffer(&entries, true, "\n\n", write).unwrap();
    }
    assert_eq!(writes.get(), 2);
    assert_eq!(*clipboard.borrow(), "Review\n\n## notes.md\n\ncafé");
    fs::remove_file(path).unwrap();
    assert!(publish_buffer(&entries, true, "\n\n", write).is_err());
    assert_eq!(writes.get(), 2);
    assert_eq!(*clipboard.borrow(), "Review\n\n## notes.md\n\ncafé");
}
