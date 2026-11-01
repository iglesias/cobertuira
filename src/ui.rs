use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};
use std::fs;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

const TOKYO_NIGHT_BG: Color = Color::Rgb(26, 27, 38);
const TOKYO_NIGHT_FG: Color = Color::Rgb(169, 177, 214);
const TOKYO_NIGHT_CYAN: Color = Color::Rgb(125, 207, 255);
const TOKYO_NIGHT_GREEN: Color = Color::Rgb(158, 206, 106);
const TOKYO_NIGHT_RED: Color = Color::Rgb(247, 118, 142);
const COVERAGE_GREEN_BG: Color = Color::Rgb(28, 51, 28);
const COVERAGE_RED_BG: Color = Color::Rgb(51, 28, 28);

pub fn render(f: &mut Frame, app: &mut App, syntax_set: &SyntaxSet, theme_set: &ThemeSet) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.size());

    // Background
    f.render_widget(
        Block::default().style(Style::default().bg(TOKYO_NIGHT_BG)),
        f.size(),
    );

    // Header / Filter
    let filter_active = app.active_panel == crate::app::ActivePanel::Filter;
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if filter_active {
            Color::Cyan
        } else {
            TOKYO_NIGHT_CYAN
        }))
        .title(
            " CoberTUIra - 'Tab' to switch focus, 'r' to Refresh, 'q' to Quit ",
        );

    let filter_text = if filter_active {
        format!(" Filter: {}_ ", app.filter_query)
    } else {
        format!(" Filter: {} ", app.filter_query)
    };

    let filter_para = Paragraph::new(filter_text)
        .block(header_block)
        .style(Style::default().fg(if filter_active {
            Color::White
        } else {
            TOKYO_NIGHT_FG
        }));
    f.render_widget(filter_para, chunks[0]);

    // Main Area
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    // File List
    let files_active = app.active_panel == crate::app::ActivePanel::Files;
    let selected_file = if !app.filtered_files.is_empty() {
        Some(&app.files[app.filtered_files[app.selected_index]])
    } else {
        None
    };

    let rows: Vec<Row> = app
        .filtered_files
        .iter()
        .map(|&i| {
            let file = &app.files[i];
            Row::new(vec![
                file.rel_path.clone(),
                format!("{:.1}%", file.line_pct),
                format!("{:.1}%", file.branch_pct),
            ])
            .style(Style::default().fg(TOKYO_NIGHT_FG))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(70),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ],
    )
    .header(Row::new(vec!["File", "Lines", "Branches"]).style(Style::default().fg(Color::Yellow)))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if files_active {
                Color::Cyan
            } else {
                TOKYO_NIGHT_CYAN
            }))
            .title(" Files "),
    )
    .highlight_style(
        Style::default()
            .bg(if files_active {
                Color::Rgb(60, 64, 80)
            } else {
                Color::Rgb(44, 46, 61)
            })
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    let mut table_state = TableState::default();
    table_state.select(Some(app.selected_index));
    f.render_stateful_widget(table, main_chunks[0], &mut table_state);

    // Source View
    let source_active = app.active_panel == crate::app::ActivePanel::Source;
    if let Some(file) = selected_file {
        render_source_view(
            f,
            main_chunks[1],
            app,
            file,
            syntax_set,
            theme_set,
            source_active,
        );
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(TOKYO_NIGHT_CYAN))
            .title(" Source ");
        f.render_widget(
            Paragraph::new("No files found.").block(block),
            main_chunks[1],
        );
    }

    // Footer
    let footer_text = if app.is_refreshing {
        Span::styled(
            " Refreshing coverage data... ",
            Style::default()
                .fg(TOKYO_NIGHT_RED)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " Use arrows/j/k to navigate, type to filter. ",
            Style::default().fg(TOKYO_NIGHT_FG),
        )
    };
    f.render_widget(Paragraph::new(Line::from(vec![footer_text])), chunks[2]);
}

fn render_source_view(
    f: &mut Frame,
    area: Rect,
    app: &App,
    file: &crate::coverage::ProcessedFile,
    syntax_set: &SyntaxSet,
    theme_set: &ThemeSet,
    focused: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            Color::Cyan
        } else {
            TOKYO_NIGHT_CYAN
        }))
        .title(format!(" Source: {} ", file.rel_path));
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    match fs::read_to_string(&file.path) {
        Ok(code) => {
            let syntax = syntax_set
                .find_syntax_by_extension("cpp")
                .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
            let theme = &theme_set.themes["base16-ocean.dark"];
            let mut h = HighlightLines::new(syntax, theme);

            let mut lines = Vec::new();
            let mut last_test_set: Option<Vec<String>> = None;

            for (i, line_content) in LinesWithEndings::from(&code).enumerate() {
                let line_num = (i + 1) as u32;
                if line_num < app.scroll_offset as u32 + 1
                    || lines.len() >= inner_area.height as usize
                {
                    if line_num < app.scroll_offset as u32 + 1 {
                        let _ = h.highlight_line(line_content, syntax_set);
                        continue;
                    }
                    if lines.len() >= inner_area.height as usize {
                        break;
                    }
                }

                let count = file.get_line_count(line_num);
                let gutter_style = match count {
                    None => Style::default().fg(Color::DarkGray),
                    Some(0) => Style::default().fg(TOKYO_NIGHT_RED),
                    Some(_) => Style::default().fg(TOKYO_NIGHT_GREEN),
                };

                let bg_color = match count {
                    Some(0) => Some(COVERAGE_RED_BG),
                    Some(_) if !line_content.trim().is_empty() => Some(COVERAGE_GREEN_BG),
                    _ => None,
                };

                let gutter_text = match count {
                    None => format!("{:4} \u{2502}      \u{2502} ", line_num),
                    Some(0) => format!("{:4} \u{2502} 0    \u{2502} ", line_num),
                    Some(c) => format!("{:4} \u{2502} {:<4} \u{2502} ", line_num, c),
                };

                let mut spans = vec![Span::styled(gutter_text, gutter_style)];

                let ranges: Vec<(syntect::highlighting::Style, &str)> =
                    h.highlight_line(line_content, syntax_set).unwrap();
                for (style, text) in ranges {
                    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                    spans.push(Span::styled(text.to_string(), Style::default().fg(fg)));
                }

                let mut extra = String::new();
                if let Some(branches) = file.branch_info.get(&line_num) {
                    if branches.len() > 1 || (branches.len() == 1 && branches[0] > 0) {
                        extra.push_str(&format!(" B{:?}", branches));
                    }
                }

                let current_tests = file.covered_by.get(&line_num).map(|set| {
                    let mut v: Vec<_> = set.iter().cloned().collect();
                    v.sort();
                    v
                });

                if let Some(ref tests) = current_tests {
                    if !line_content.trim().is_empty() && current_tests != last_test_set {
                        let mut test_str = tests.join(",");
                        if test_str.len() > 60 {
                            test_str.truncate(57);
                            test_str.push_str("...");
                        }
                        extra.push_str(&format!(" T:{}", test_str));
                        last_test_set = current_tests;
                    }
                } else {
                    last_test_set = None;
                }

                if !extra.is_empty() {
                    spans.push(Span::styled(
                        extra,
                        Style::default()
                            .fg(TOKYO_NIGHT_CYAN)
                            .add_modifier(Modifier::ITALIC),
                    ));
                }

                let mut line = Line::from(spans);
                if let Some(bg) = bg_color {
                    line = line.style(Style::default().bg(bg));
                }
                lines.push(line);
            }

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner_area);
        }
        Err(e) => {
            f.render_widget(
                Paragraph::new(format!("Error reading file: {}", e)),
                inner_area,
            );
        }
    }
}
