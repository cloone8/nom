//! Markdown to HTML for recipe text. Server only.
//!
//! Everything the user types shows up on the page: raw HTML and links with a blocked scheme
//! (`javascript:` and the like) are shown as text instead of being dropped.

use comrak::html::dangerous_url;
use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, Options, format_html, parse_document};

/// Renders user-entered Markdown to HTML that's safe to put on the page as is.
pub fn render(markdown: &str) -> String {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    // A single Enter starts a new line, like in a notes app
    options.render.hardbreaks = true;
    options.render.escape = true;

    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &options);

    // Collected first, because the edits below change the tree the iterator walks
    let nodes: Vec<_> = root.descendants().collect();

    for node in nodes {
        let value = node.data.borrow().value.clone();

        match value {
            NodeValue::HtmlBlock(block) => show_html_block(&arena, node, &block.literal),
            NodeValue::Link(link) if dangerous_url(&link.url) => {
                show_blocked_link(&arena, node, "[", &link.url, &link.title)
            }
            NodeValue::Image(link) if dangerous_url(&link.url) => {
                show_blocked_link(&arena, node, "![", &link.url, &link.title)
            }
            _ => {}
        }
    }

    let mut output = String::new();
    format_html(root, &options, &mut output).expect("writing to a String can't fail");

    output
}

fn text<'a>(arena: &'a Arena<'a>, literal: &str) -> &'a AstNode<'a> {
    arena.alloc(NodeValue::Text(literal.to_owned().into()).into())
}

/// `render.escape` emits an HTML block as bare text, so the CSS for `p` misses it and its line
/// breaks collapse.
fn show_html_block<'a>(arena: &'a Arena<'a>, node: &'a AstNode<'a>, literal: &str) {
    node.data.borrow_mut().value = NodeValue::Paragraph;

    for (i, line) in literal.lines().enumerate() {
        if i > 0 {
            node.append(arena.alloc(NodeValue::LineBreak.into()));
        }

        node.append(text(arena, line));
    }
}

/// The renderer keeps a blocked link with an empty `href`, which loses the URL the user typed.
fn show_blocked_link<'a>(
    arena: &'a Arena<'a>,
    node: &'a AstNode<'a>,
    opening: &str,
    url: &str,
    title: &str,
) {
    let label: Vec<_> = node.children().collect();
    let is_autolink = matches!(&label[..], [only] if matches!(&only.data.borrow().value, NodeValue::Text(t) if t == url));

    if !is_autolink {
        node.insert_before(text(arena, opening));
    }

    for child in label {
        node.insert_before(child);
    }

    if !is_autolink && title.is_empty() {
        node.insert_before(text(arena, &format!("]({url})")));
    } else if !is_autolink {
        node.insert_before(text(arena, &format!("]({url} \"{title}\")")));
    }

    node.detach();
}

#[cfg(test)]
mod tests {
    use super::render;

    /// Every `<` in escaped output starts a real tag, so this finds anything that got through.
    fn assert_only_safe_tags(html: &str) {
        const ALLOWED: &[&str] = &[
            "h1",
            "p",
            "a",
            "img",
            "code",
            "pre",
            "strong",
            "em",
            "del",
            "br",
            "ul",
            "ol",
            "li",
            "blockquote",
            "table",
            "thead",
            "tbody",
            "tr",
            "th",
            "td",
            "input",
        ];

        for fragment in html.split('<').skip(1) {
            let tag = fragment
                .split('>')
                .next()
                .expect("split always yields one item");
            let name = tag
                .trim_start_matches('/')
                .split([' ', '/'])
                .next()
                .expect("split always yields one item");

            assert!(
                ALLOWED.contains(&name),
                "unexpected tag <{tag}> in:\n{html}"
            );
            assert!(!tag.contains(" on"), "event handler in <{tag}>");
            assert!(
                !tag.contains("href=\"\"") && !tag.contains("src=\"\""),
                "emptied URL in <{tag}>"
            );
            assert!(
                !tag.to_lowercase().contains("script:"),
                "script URL in <{tag}>"
            );
        }
    }

    #[test]
    fn typed_html_and_blocked_links_show_as_text() {
        const INPUT: &str = r#"# Title

Inline <b>html</b> and 1 < 2 && 3 > 2.

<script>alert(1)</script>

<img src=x onerror=alert(2)>

<!-- a comment -->

<div onclick="alert(4)">
block **html**
</div>

> <iframe src="https://evil.example"></iframe>

- <style>body { display: none }</style>

[click **bold**](javascript:alert(3))

[titled](javascript:alert(7) "ti<t>le")

![pic](javascript:alert(5))

[data](data:text/html;base64,PHNjcmlwdD4=)

[case](JaVaScRiPt:alert(8))

[entity](java&#115;cript:alert(9))

[quote](javascript:alert("<b>"))

[![inner](javascript:a)](javascript:b)

<javascript:alert(6)>

[ok](https://example.com "ti<t>le")

![fine](https://example.com/a.png)

`<code>` stays, &amp; and \* and [ref]

[ref]: https://example.com
"#;

        let output = render(INPUT);

        assert_only_safe_tags(&output);
        assert!(output.contains("<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>"));
        assert!(output.contains(
            "&lt;div onclick=&quot;alert(4)&quot;&gt;<br />\nblock **html**<br />\n&lt;/div&gt;"
        ));
        assert!(output.contains("[click <strong>bold</strong>](javascript:alert(3))"));
        assert!(output.contains("<p>javascript:alert(6)</p>"));
        assert!(output.contains("<a href=\"https://example.com\" title=\"ti&lt;t&gt;le\">ok</a>"));
    }

    #[test]
    fn single_newline_is_a_line_break() {
        assert_eq!(
            render("Week de erwten.\nKook 2 uur."),
            "<p>Week de erwten.<br />\nKook 2 uur.</p>\n"
        );
    }
}
