//! Just enough Markdown for a terminal note preview: headings, lists and
//! task lists, quotes, code blocks, rules, YAML front matter, and inline
//! bold / italic / strike / `code` / [[wiki links]] / [links](url) / #tags.
//! Anything unrecognised is shown as written.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Theme;

/// Render `src` into styled lines. Consecutive blank lines collapse to one.
pub fn render(src: &str, theme: &Theme) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut in_code = false;
    // Blank lines are held back one step so a gap between two list items
    // (common in hand-written notes) doesn't spread the list out.
    let (mut pending_blank, mut last_was_item) = (false, false);
    let mut lines = src.lines().peekable();

    // YAML front matter: a leading `---` block. Skipped entirely.
    if lines.peek().is_some_and(|l| l.trim() == "---") {
        lines.next();
        for l in lines.by_ref() {
            if l.trim() == "---" {
                break;
            }
        }
    }

    for raw in lines {
        let trimmed = raw.trim_start();
        if !in_code {
            if trimmed.is_empty() {
                pending_blank = true;
                continue;
            }
            let item = is_item(trimmed);
            if std::mem::take(&mut pending_blank)
                && !(item && last_was_item)
                && out.last().is_some_and(|l| l.width() > 0)
            {
                out.push(Line::default());
            }
            last_was_item = item;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code = !in_code;
            if in_code {
                let lang = trimmed.trim_start_matches(['`', '~']).trim();
                out.push(Line::from(Span::styled(
                    format!("  ╭─ {}", if lang.is_empty() { "code" } else { lang }),
                    Style::default().fg(theme.dim),
                )));
            } else {
                out.push(Line::from(Span::styled(
                    "  ╰─",
                    Style::default().fg(theme.dim),
                )));
            }
            continue;
        }
        if in_code {
            out.push(Line::from(vec![
                Span::styled("  │ ", Style::default().fg(theme.dim)),
                Span::styled(raw.to_string(), Style::default().fg(theme.yellow)),
            ]));
            continue;
        }
        let indent = " ".repeat((raw.len() - trimmed.len()).min(8));
        let base = Style::default().fg(theme.text);

        // Headings.
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            let text = trimmed[hashes..].trim();
            let style = match hashes {
                1 => Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                2 => Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
                _ => Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            };
            // A heading always gets breathing room above it.
            if out.last().is_some_and(|l| l.width() > 0) {
                out.push(Line::default());
            }
            out.push(Line::from(inline(text, style, theme)));
            continue;
        }

        // Horizontal rule.
        let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.len() >= 3
            && (compact.chars().all(|c| c == '-')
                || compact.chars().all(|c| c == '*')
                || compact.chars().all(|c| c == '_'))
        {
            out.push(Line::from(Span::styled(
                "─".repeat(24),
                Style::default().fg(theme.surface),
            )));
            continue;
        }

        // Quote.
        if let Some(rest) = trimmed.strip_prefix('>') {
            let style = Style::default()
                .fg(theme.dim)
                .add_modifier(Modifier::ITALIC);
            let mut spans = vec![Span::styled(
                format!("{}▎ ", indent),
                Style::default().fg(theme.secondary),
            )];
            spans.extend(inline(rest.trim_start(), style, theme));
            out.push(Line::from(spans));
            continue;
        }

        // Task list item.
        let task = ["- [ ] ", "* [ ] ", "- [x] ", "- [X] ", "* [x] "]
            .iter()
            .find_map(|p| {
                trimmed
                    .strip_prefix(p)
                    .map(|rest| (p.contains('x') || p.contains('X'), rest))
            });
        if let Some((done, rest)) = task {
            let (mark, style) = if done {
                (
                    "☑ ",
                    Style::default()
                        .fg(theme.dim)
                        .add_modifier(Modifier::CROSSED_OUT),
                )
            } else {
                ("☐ ", base)
            };
            let mut spans = vec![Span::styled(
                format!("{}{}", indent, mark),
                Style::default().fg(if done { theme.green } else { theme.accent }),
            )];
            spans.extend(inline(rest, style, theme));
            out.push(Line::from(spans));
            continue;
        }

        // Bullet or numbered list item.
        let bullet = ["- ", "* ", "+ "]
            .iter()
            .find_map(|p| trimmed.strip_prefix(p));
        let numbered = trimmed.split_once(". ").filter(|(n, _)| {
            !n.is_empty() && n.len() <= 3 && n.chars().all(|c| c.is_ascii_digit())
        });
        if let Some(rest) = bullet {
            let dot = if indent.is_empty() { "• " } else { "◦ " };
            let mut spans = vec![Span::styled(
                format!("{}{}", indent, dot),
                Style::default().fg(theme.accent),
            )];
            spans.extend(inline(rest, base, theme));
            out.push(Line::from(spans));
            continue;
        }
        if let Some((n, rest)) = numbered {
            let mut spans = vec![Span::styled(
                format!("{}{}. ", indent, n),
                Style::default().fg(theme.accent),
            )];
            spans.extend(inline(rest, base, theme));
            out.push(Line::from(spans));
            continue;
        }

        let mut spans = Vec::new();
        if !indent.is_empty() {
            spans.push(Span::raw(indent));
        }
        spans.extend(inline(trimmed, base, theme));
        out.push(Line::from(spans));
    }
    while out.last().is_some_and(|l| l.width() == 0) {
        out.pop();
    }
    out
}

/// A bullet, numbered or task list item.
fn is_item(trimmed: &str) -> bool {
    ["- ", "* ", "+ "].iter().any(|p| trimmed.starts_with(p))
        || trimmed.split_once(". ").is_some_and(|(n, _)| {
            !n.is_empty() && n.len() <= 3 && n.chars().all(|c| c.is_ascii_digit())
        })
}

/// Inline markup within one line, on top of `base`.
fn inline(text: &str, base: Style, theme: &Theme) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let (mut bold, mut italic, mut strike) = (false, false, false);
    let style_now = |bold: bool, italic: bool, strike: bool| {
        let mut s = base;
        if bold {
            s = s.add_modifier(Modifier::BOLD);
        }
        if italic {
            s = s.add_modifier(Modifier::ITALIC);
        }
        if strike {
            s = s.add_modifier(Modifier::CROSSED_OUT);
        }
        s
    };
    let flush = |buf: &mut String, spans: &mut Vec<Span<'static>>, style: Style| {
        if !buf.is_empty() {
            spans.push(Span::styled(std::mem::take(buf), style));
        }
    };
    let find = |from: usize, pat: &[char]| -> Option<usize> {
        (from..chars.len().saturating_sub(pat.len() - 1)).find(|&j| chars[j..j + pat.len()] == *pat)
    };
    let word_char = |i: usize| chars.get(i).is_some_and(|c| c.is_alphanumeric());

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let cur = style_now(bold, italic, strike);
        match c {
            '`' => {
                if let Some(end) = find(i + 1, &['`']) {
                    flush(&mut buf, &mut spans, cur);
                    let code: String = chars[i + 1..end].iter().collect();
                    spans.push(Span::styled(
                        code,
                        Style::default().fg(theme.yellow).bg(theme.surface),
                    ));
                    i = end + 1;
                    continue;
                }
            }
            '[' if next == Some('[') => {
                if let Some(end) = find(i + 2, &[']', ']']) {
                    flush(&mut buf, &mut spans, cur);
                    let inner: String = chars[i + 2..end].iter().collect();
                    let shown = inner.rsplit('|').next().unwrap_or(&inner).to_string();
                    spans.push(Span::styled(
                        shown,
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                    i = end + 2;
                    continue;
                }
            }
            '[' => {
                if let Some(close) = find(i + 1, &[']']) {
                    if chars.get(close + 1) == Some(&'(') {
                        if let Some(paren) = find(close + 2, &[')']) {
                            flush(&mut buf, &mut spans, cur);
                            let label: String = chars[i + 1..close].iter().collect();
                            spans.push(Span::styled(
                                label,
                                cur.fg(theme.secondary).add_modifier(Modifier::UNDERLINED),
                            ));
                            i = paren + 1;
                            continue;
                        }
                    }
                }
            }
            '*' | '_' if next == Some(c) => {
                flush(&mut buf, &mut spans, cur);
                bold = !bold;
                i += 2;
                continue;
            }
            '~' if next == Some('~') => {
                flush(&mut buf, &mut spans, cur);
                strike = !strike;
                i += 2;
                continue;
            }
            // Single `*`/`_` only toggles italics at a word boundary, so
            // snake_case and 2*3 stay literal.
            '*' | '_'
                if (italic && !word_char(i + 1))
                    || (!italic && !word_char(i.wrapping_sub(1)) && word_char(i + 1)) =>
            {
                flush(&mut buf, &mut spans, cur);
                italic = !italic;
                i += 1;
                continue;
            }
            '#' if !word_char(i.wrapping_sub(1)) && next.is_some_and(|n| n.is_alphanumeric()) => {
                let end = (i + 1..chars.len())
                    .find(|&j| !(chars[j].is_alphanumeric() || "-_/".contains(chars[j])))
                    .unwrap_or(chars.len());
                flush(&mut buf, &mut spans, cur);
                spans.push(Span::styled(
                    chars[i..end].iter().collect::<String>(),
                    Style::default().fg(theme.secondary),
                ));
                i = end;
                continue;
            }
            _ => {}
        }
        buf.push(c);
        i += 1;
    }
    flush(&mut buf, &mut spans, style_now(bold, italic, strike));
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    fn t() -> Theme {
        Theme::dark()
    }

    #[test]
    fn inline_markers_are_consumed_and_styled() {
        let spans = inline(
            "a **bold** and *it* `x` [[Note|alias]] [site](http://x)",
            Style::default(),
            &t(),
        );
        let s: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(s, "a bold and it x alias site");
        let bold = spans.iter().find(|s| s.content == "bold").unwrap();
        assert!(bold.style.add_modifier.contains(Modifier::BOLD));
        let it = spans.iter().find(|s| s.content == "it").unwrap();
        assert!(it.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn snake_case_and_arithmetic_stay_literal() {
        let s: String = inline("my_var_name and 2*3*4", Style::default(), &t())
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert_eq!(s, "my_var_name and 2*3*4");
    }

    #[test]
    fn blocks_render_without_markdown_syntax() {
        let md = "---\ntags: x\n---\n# Title\n\n\n\n- one\n\n  - two\n- [ ] todo\n- [x] done\n> quote\n1. first\n```rust\nlet a = 1;\n```\n---\nplain #tag";
        let out = text(&render(md, &t()));
        assert_eq!(
            out,
            vec![
                "Title",
                "",
                "• one",
                "  ◦ two",
                "☐ todo",
                "☑ done",
                "▎ quote",
                "1. first",
                "  ╭─ rust",
                "  │ let a = 1;",
                "  ╰─",
                "────────────────────────",
                "plain #tag",
            ]
        );
    }
}
