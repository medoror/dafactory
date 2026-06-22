//! Selecting the next backlog intent (SPEC `run` step 2). The next intent is the
//! first unchecked checkbox item (`- [ ]`) in BACKLOG.md, in file order. Checked
//! items (`- [x]`) are addressed and skipped, and anything inside an HTML comment is
//! ignored (the scaffold hides its example intent that way). The full line is handed
//! to the agent verbatim; the id/title are parsed out for the evidence bundle.

use crate::agent::Intent;

/// The first unaddressed intent, or `None` if every item is checked (or there are
/// none). Content inside `<!-- ... -->` is not considered.
pub fn next_intent(backlog: &str) -> Option<Intent> {
    strip_html_comments(backlog)
        .lines()
        .find_map(parse_unchecked)
}

/// Replace `[ ]` with `[x]` on the first line whose trimmed content equals `raw`.
/// `raw` is already trimmed (from `Intent.raw`). Returns the original text if the
/// line is not found — safe no-op.
pub fn tick_intent(backlog: &str, raw: &str) -> String {
    let mut ticked = false;
    let body = backlog
        .lines()
        .map(|line| {
            if !ticked && line.trim() == raw {
                ticked = true;
                line.replacen("[ ]", "[x]", 1)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if backlog.ends_with('\n') {
        body + "\n"
    } else {
        body
    }
}

/// Remove every `<!-- ... -->` span (including multi-line and unterminated ones) so a
/// commented-out example intent is never mistaken for a real one. HTML comments do
/// not nest, so a simple scan for matching delimiters is sufficient.
fn strip_html_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => return out, // unterminated comment: drop the remainder
        }
    }
    out.push_str(rest);
    out
}

/// Parse a single line into an `Intent` iff it is an unchecked checkbox item.
fn parse_unchecked(line: &str) -> Option<Intent> {
    let trimmed = line.trim();
    let body = unchecked_body(trimmed)?;
    // Strip surrounding markdown bold and trailing punctuation for the readable bits.
    let cleaned = body.trim().trim_matches('*').trim();
    Some(Intent {
        id: extract_id(cleaned),
        title: extract_title(cleaned),
        raw: trimmed.to_string(),
    })
}

/// The text after the `- [ ]` / `* [ ]` marker, or `None` if `line` is not an
/// unchecked checkbox item. Checked items use `[x]`/`[X]` and do not match.
fn unchecked_body(line: &str) -> Option<&str> {
    let after_bullet = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))?;
    let after_box = after_bullet.strip_prefix("[ ]")?;
    Some(after_box.trim_start())
}

/// First token that looks like an intent id (letters then digits, e.g. `B3`),
/// ignoring surrounding punctuation.
fn extract_id(cleaned: &str) -> Option<String> {
    cleaned
        .split_whitespace()
        .map(|tok| tok.trim_matches(|c: char| !c.is_alphanumeric()))
        .find(|tok| is_id(tok))
        .map(str::to_string)
}

fn is_id(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|c| c.is_ascii_alphanumeric())
        && token
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && token.chars().any(|c| c.is_ascii_digit())
}

/// The human title: the text after the first em dash if present, else the whole
/// cleaned line; trailing markdown/punctuation trimmed.
fn extract_title(cleaned: &str) -> String {
    let tail = cleaned.split_once('—').map(|(_, t)| t).unwrap_or(cleaned);
    tail.trim().trim_end_matches(['.', '*']).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_select_first_unchecked_item_and_parse_id_and_title() {
        let backlog = "\
# BACKLOG
- [x] **B1 (→ S001) — init scaffolds a line.**
- [x] **B2 (→ S002) — validate runs the judge.**
- [ ] **B3 (→ S003) — run --once happy path.**
- [ ] **B4 (→ S004) — run can do nothing.**
";
        let intent = next_intent(backlog).unwrap();

        assert_eq!(intent.id.as_deref(), Some("B3"));
        assert_eq!(intent.title, "run --once happy path");
        assert!(intent.raw.contains("B3"));
    }

    #[test]
    fn should_return_none_when_every_item_is_checked() {
        let backlog = "- [x] **B1 — done.**\n- [x] **B2 — done.**\n";
        assert!(next_intent(backlog).is_none());
    }

    #[test]
    fn should_return_none_for_empty_backlog() {
        assert!(next_intent("# BACKLOG\n\nno items here\n").is_none());
    }

    #[test]
    fn should_handle_item_without_an_id() {
        let intent = next_intent("- [ ] Build the parser\n").unwrap();
        assert_eq!(intent.id, None);
        assert_eq!(intent.title, "Build the parser");
    }

    #[test]
    fn should_skip_checked_items_with_capital_x() {
        let backlog = "- [X] **B1 — done.**\n- [ ] **B2 — next up.**\n";
        let intent = next_intent(backlog).unwrap();
        assert_eq!(intent.id.as_deref(), Some("B2"));
    }

    #[test]
    fn should_ignore_intents_hidden_in_an_html_comment() {
        // The scaffolded BACKLOG.md hides its example intent in an HTML comment; it
        // must not be treated as a real open intent.
        let backlog = "\
# BACKLOG
<!--
- [ ] **B1 (→ S001) — <short title>.**
  <One or two sentences describing the intent.>
-->
";
        assert!(next_intent(backlog).is_none());
    }

    #[test]
    fn should_select_the_real_intent_after_a_commented_example() {
        let backlog = "\
<!--
- [ ] **B1 — example, ignore me.**
-->
- [ ] **B2 — real work.**
";
        let intent = next_intent(backlog).unwrap();
        assert_eq!(intent.id.as_deref(), Some("B2"));
    }

    #[test]
    fn should_ignore_a_single_line_comment_intent() {
        assert!(next_intent("<!-- - [ ] **B1 — x.** -->\n").is_none());
    }

    #[test]
    fn should_tick_the_matching_intent_line() {
        let backlog = "- [x] **B1 — done.**\n- [ ] **B2 — do it.**\n- [ ] **B3 — later.**\n";
        let result = tick_intent(backlog, "- [ ] **B2 — do it.**");
        assert_eq!(
            result,
            "- [x] **B1 — done.**\n- [x] **B2 — do it.**\n- [ ] **B3 — later.**\n"
        );
    }

    #[test]
    fn should_leave_text_unchanged_when_line_not_found() {
        let backlog = "- [ ] **B1 — something.**\n";
        let result = tick_intent(backlog, "- [ ] **B99 — nonexistent.**");
        assert_eq!(result, backlog);
    }

    #[test]
    fn should_only_tick_the_first_matching_line() {
        let backlog = "- [ ] **B1 — dup.**\n- [ ] **B1 — dup.**\n";
        let result = tick_intent(backlog, "- [ ] **B1 — dup.**");
        assert_eq!(result, "- [x] **B1 — dup.**\n- [ ] **B1 — dup.**\n");
    }

    #[test]
    fn should_preserve_trailing_newline() {
        let backlog = "- [ ] **B1 — x.**\n";
        let result = tick_intent(backlog, "- [ ] **B1 — x.**");
        assert!(result.ends_with('\n'), "trailing newline must be preserved");
    }
}
