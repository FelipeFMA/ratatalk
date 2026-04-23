use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;
use pulldown_cmark::{Event, Parser, Tag, Options, TagEnd};

/// Parse markdown text into ratatui Lines
pub fn parse_markdown(text: &str, max_width: usize, base_style: Style) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(text, options);
    let mut lines = Vec::new();
    let mut current_line_spans = Vec::new();
    let mut current_style = base_style;
    let mut style_stack = Vec::new();
    
    let mut in_code_block = false;
    let mut in_blockquote = false;
    let mut list_index = 0;

    for event in parser {
        match event {
            Event::Start(tag) => {
                style_stack.push(current_style);
                match tag {
                    Tag::Strong => {
                        current_style = current_style.add_modifier(Modifier::BOLD);
                    }
                    Tag::Emphasis => {
                        current_style = current_style.add_modifier(Modifier::ITALIC);
                    }
                    Tag::Strikethrough => {
                        current_style = current_style.add_modifier(Modifier::CROSSED_OUT);
                    }
                    Tag::Heading { level, .. } => {
                        current_style = current_style.add_modifier(Modifier::BOLD).fg(match level {
                            pulldown_cmark::HeadingLevel::H1 => Color::Magenta,
                            pulldown_cmark::HeadingLevel::H2 => Color::Blue,
                            pulldown_cmark::HeadingLevel::H3 => Color::Cyan,
                            _ => Color::Yellow,
                        });
                        // Add empty line before heading if not at start
                        if !lines.is_empty() || !current_line_spans.is_empty() {
                            if !current_line_spans.is_empty() {
                                lines.push(Line::from(current_line_spans.split_off(0)));
                            }
                            lines.push(Line::from(""));
                        }
                    }
                    Tag::CodeBlock(_) => {
                        in_code_block = true;
                        current_style = current_style.fg(Color::Yellow);
                        if !current_line_spans.is_empty() {
                            lines.push(Line::from(current_line_spans.split_off(0)));
                        }
                    }
                    Tag::List(start) => {
                        list_index = start.unwrap_or(0);
                    }
                    Tag::Item => {
                        if !current_line_spans.is_empty() {
                            lines.push(Line::from(current_line_spans.split_off(0)));
                        }
                        let prefix = if list_index > 0 {
                            let p = format!("{}. ", list_index);
                            list_index += 1;
                            p
                        } else {
                            "• ".to_string()
                        };
                        current_line_spans.push(Span::styled(prefix, Style::default().fg(Color::Yellow)));
                    }
                    Tag::Link { .. } => {
                        current_style = current_style.fg(Color::Blue).add_modifier(Modifier::UNDERLINED);
                    }
                    Tag::Paragraph => {
                        if !lines.is_empty() || !current_line_spans.is_empty() {
                            if !current_line_spans.is_empty() {
                                lines.push(Line::from(current_line_spans.split_off(0)));
                            }
                            lines.push(Line::from(""));
                        }
                    }
                    Tag::BlockQuote(_) => {
                        in_blockquote = true;
                        current_style = current_style.add_modifier(Modifier::ITALIC).fg(Color::DarkGray);
                        if !current_line_spans.is_empty() {
                            lines.push(Line::from(current_line_spans.split_off(0)));
                        }
                    }
                    _ => {}
                }
            }
            Event::End(tag) => {
                if let Some(prev_style) = style_stack.pop() {
                    match tag {
                        TagEnd::Heading(_) => {
                            if !current_line_spans.is_empty() {
                                lines.push(Line::from(current_line_spans.split_off(0)));
                            }
                            lines.push(Line::from(""));
                        }
                        TagEnd::CodeBlock => {
                            in_code_block = false;
                        }
                        TagEnd::List(_) => {
                        }
                        TagEnd::BlockQuote(_) => {
                            in_blockquote = false;
                            if !current_line_spans.is_empty() {
                                lines.push(Line::from(current_line_spans.split_off(0)));
                            }
                        }
                        TagEnd::Paragraph => {
                            if !current_line_spans.is_empty() {
                                lines.push(Line::from(current_line_spans.split_off(0)));
                            }
                        }
                        _ => {}
                    }
                    current_style = prev_style;
                }
            }
            Event::Text(text) => {
                if in_code_block {
                    for line in text.split('\n') {
                        lines.push(Line::from(Span::styled(format!("  {}", line), current_style)));
                    }
                } else {
                    let mut current_width = current_line_spans.iter().map(|s| s.width()).sum::<usize>();
                    
                    if in_blockquote && current_line_spans.is_empty() {
                        current_line_spans.push(Span::styled("> ", current_style));
                        current_width += 2;
                    }

                    // Preserve original spacing by splitting into tokens (words and whitespace)
                    let tokens = text.split_inclusive(char::is_whitespace);

                    for token in tokens {
                        if token.contains('\n') {
                            if !current_line_spans.is_empty() {
                                lines.push(Line::from(current_line_spans.split_off(0)));
                            }
                            if in_blockquote {
                                current_line_spans.push(Span::styled("> ", current_style));
                                current_width = 2;
                            } else {
                                current_width = 0;
                            }
                            continue;
                        }

                        let token_width = token.width();
                        if max_width > 0 && current_width + token_width > max_width {
                            // Only wrap on non-whitespace or if the line is already very long
                            if !token.chars().all(|c| c.is_whitespace()) {
                                if !current_line_spans.is_empty() {
                                    lines.push(Line::from(current_line_spans.split_off(0)));
                                }
                                if in_blockquote {
                                    current_line_spans.push(Span::styled("> ", current_style));
                                    current_width = 2;
                                } else {
                                    current_width = 0;
                                }
                                // Trim leading whitespace from the wrapped word
                                let trimmed = token.trim_start();
                                current_line_spans.push(Span::styled(trimmed.to_string(), current_style));
                                current_width += trimmed.width();
                            } else {
                                // If it's just whitespace causing the overflow, just start a new line
                                lines.push(Line::from(current_line_spans.split_off(0)));
                                if in_blockquote {
                                    current_line_spans.push(Span::styled("> ", current_style));
                                    current_width = 2;
                                } else {
                                    current_width = 0;
                                }
                            }
                        } else {
                            current_line_spans.push(Span::styled(token.to_string(), current_style));
                            current_width += token_width;
                        }
                    }
                }
            }
            Event::Code(text) => {
                let code_text = text.to_string();
                let current_width = current_line_spans.iter().map(|s| s.width()).sum::<usize>();
                
                if max_width > 0 && current_width + code_text.width() > max_width {
                    if !current_line_spans.is_empty() {
                        lines.push(Line::from(current_line_spans.split_off(0)));
                    }
                    if in_blockquote {
                        current_line_spans.push(Span::styled("> ", current_style));
                    }
                }
                
                current_line_spans.push(Span::styled(
                    code_text,
                    Style::default().bg(Color::Rgb(40, 44, 52)).fg(Color::Yellow),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_line_spans.is_empty() {
                    lines.push(Line::from(current_line_spans.split_off(0)));
                }
                if in_blockquote {
                    current_line_spans.push(Span::styled("> ", current_style));
                }
            }
            _ => {}
        }
    }

    if !current_line_spans.is_empty() {
        lines.push(Line::from(current_line_spans));
    }

    // Fallback if empty
    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_simple() {
        let lines = parse_markdown("Hello **world**", 20, Style::default());
        assert!(!lines.is_empty());
        let line = &lines[0];
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content, "Hello");
        assert_eq!(line.spans[1].content, " world");
    }

    #[test]
    fn test_parse_markdown_heading() {
        let lines = parse_markdown("# Heading", 20, Style::default());
        assert!(lines.iter().any(|l| l.spans.iter().any(|s| s.content.contains("Heading"))));
    }

    #[test]
    fn test_parse_markdown_list() {
        let lines = parse_markdown("- Item 1\n- Item 2", 20, Style::default());
        assert!(lines.iter().any(|l| l.spans.iter().any(|s| s.content.contains("• "))));
    }
}
