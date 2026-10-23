use linkcheck::{LinkKind, classify, extract_links, heading_slugs, slugify};

#[test]
fn slugify_matches_github_style() {
    assert_eq!(slugify("Hello, World!"), "hello-world");
    assert_eq!(slugify("Install & Setup"), "install--setup");
    assert_eq!(slugify("snake_case-name"), "snake_case-name");
}

#[test]
fn classifies_targets() {
    assert_eq!(classify("https://rust-lang.org"), LinkKind::Url);
    assert_eq!(classify("mailto:a@b.c"), LinkKind::Url);
    assert_eq!(classify("#setup"), LinkKind::Anchor("setup".into()));
    assert_eq!(
        classify("./guide.md#install"),
        LinkKind::File {
            path: "./guide.md".into(),
            fragment: Some("install".into())
        }
    );
    assert_eq!(
        classify("img.png?raw=1"),
        LinkKind::File {
            path: "img.png".into(),
            fragment: None
        }
    );
}

#[test]
fn windows_drive_letter_is_not_a_scheme() {
    assert!(matches!(classify(r"C:\notes.md"), LinkKind::File { .. }));
}

#[test]
fn finds_links_with_line_numbers() {
    let doc = "# Title\n\nSee [one](a.md) and [two](b.md#x).\n![pic](img.png)\n";
    let links = extract_links(doc);
    let found: Vec<(usize, &str)> = links.iter().map(|l| (l.line, l.target.as_str())).collect();
    assert_eq!(found, vec![(3, "a.md"), (3, "b.md#x"), (4, "img.png")]);
}

#[test]
fn ignores_code() {
    let doc = "Use `[x](nope.md)` inline.\n\n```\n[y](nope2.md)\n```\n[real](ok.md)\n";
    let links = extract_links(doc);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "ok.md");
}

#[test]
fn strips_titles_and_angle_brackets() {
    let links = extract_links("[a](page.md \"A title\") [b](<other.md>)");
    let targets: Vec<&str> = links.iter().map(|l| l.target.as_str()).collect();
    assert_eq!(targets, vec!["page.md", "other.md"]);
}

#[test]
fn headings_skip_fenced_code() {
    let doc = "# One\n\n```\n# not a heading\n```\n\n## Two ##\n#hashtag\n";
    assert_eq!(heading_slugs(doc), vec!["one", "two"]);
}
