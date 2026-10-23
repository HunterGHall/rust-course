/// A link found in a Markdown document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub line: usize, // 1-based
    pub target: String,
}

/// What a link points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    /// `https://...`, `mailto:...`
    Url,
    /// `#heading` in the same file.
    Anchor(String),
    /// `./other.md#heading`
    File {
        path: String,
        fragment: Option<String>,
    },
}

// ---------- Classification ----------

pub fn classify(target: &str) -> LinkKind {
    if let Some(anchor) = target.strip_prefix('#') {
        return LinkKind::Anchor(anchor.to_string());
    }
    if has_scheme(target) {
        return LinkKind::Url;
    }

    let (path, fragment) = match target.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment.to_string())),
        None => (target, None),
    };
    let path = path.split('?').next().unwrap_or("");
    LinkKind::File {
        path: path.to_string(),
        fragment,
    }
}

/// "https:" and "mailto:" have a scheme; a Windows drive letter such as "C:\" does not.
fn has_scheme(text: &str) -> bool {
    let Some((scheme, _)) = text.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    scheme.len() > 1
        && chars.next().is_some_and(|c| c.is_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_alphanumeric() || "+-.".contains(c))
}

// ---------- Headings and slugs ----------

/// GitHub-style anchor: "Hello, World!" -> "hello-world".
pub fn slugify(heading: &str) -> String {
    let mut slug = String::new();
    for c in heading.to_lowercase().chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            slug.push(c);
        } else if c == ' ' {
            slug.push('-');
        }
    }
    slug
}

/// The slugs of every ATX heading (`# Title`) in a document, ignoring fenced code.
pub fn heading_slugs(markdown: &str) -> Vec<String> {
    let mut slugs = Vec::new();
    let mut in_fence = false;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) {
            if let Some(text) = trimmed[hashes..].strip_prefix(' ') {
                slugs.push(slugify(text.trim().trim_end_matches('#').trim_end()));
            }
        }
    }
    slugs
}

// ---------- Extraction ----------

/// Finds inline links `[text](target)` and images `![alt](target)`, skipping code.
pub fn extract_links(markdown: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let mut in_fence = false;

    for (index, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        let cleaned = strip_inline_code(line);
        let bytes = cleaned.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            // Look for "](" — the join between the text and the target.
            if bytes[i] == b']' && bytes[i + 1] == b'(' {
                let start = i + 2;
                if let Some(len) = cleaned[start..].find(')') {
                    let target = link_target(&cleaned[start..start + len]);
                    if !target.is_empty() {
                        links.push(Link {
                            line: index + 1,
                            target: target.to_string(),
                        });
                    }
                    i = start + len;
                    continue;
                }
            }
            i += 1;
        }
    }
    links
}

/// `path "title"` -> `path`, and strips `<...>` wrappers.
fn link_target(raw: &str) -> &str {
    let raw = raw.trim();
    let raw = match raw.find(char::is_whitespace) {
        Some(space) => &raw[..space],
        None => raw,
    };
    raw.trim_start_matches('<').trim_end_matches('>')
}

/// Replaces `inline code` with spaces so links inside it are ignored.
fn strip_inline_code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
            out.push(' ');
        } else if in_code {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}
