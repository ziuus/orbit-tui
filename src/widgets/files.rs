use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::monitors::files::{self, PreviewContent};
use ratatui::style::Color;

pub fn render(
    f: &mut Frame,
    area: Rect,
    app: &mut crate::app::App,
) {
    let theme = &app.theme;
    let is_focused = app.focused_panel == Some(crate::app::PanelId::Files);
    let selected_idx = &mut app.panel_states.files_selected;
    let scroll = &mut app.panel_states.files_scroll;
    let snap = files::snapshot();

    // Split 40% list, 60% preview
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .spacing(1)
        .split(area);

    let has_input = app.panel_states.files_search_input_active || app.panel_states.files_rename_input_active;
    let (list_area, input_area) = if has_input {
        let v = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([ratatui::layout::Constraint::Min(0), ratatui::layout::Constraint::Length(3)])
            .split(chunks[0]);
        (v[0], Some(v[1]))
    } else {
        (chunks[0], None)
    };
    let preview_area = chunks[1];

    let num_items = snap.items.len();

    // Adjust selected index
    let max_idx = num_items.saturating_sub(1);
    *selected_idx = (*selected_idx).min(max_idx);
    let selected = *selected_idx;

    let cur_dir = snap.current_dir.to_string_lossy();
    let header = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            format!(" 📁 {}", cur_dir),
            Style::default().fg(theme.accent),
        )]),
        Line::from(""),
    ]);

    let border_color = if is_focused {
        theme.accent
    } else {
        theme.surface
    };

    let list_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(border_color));

    // Apply the block to the whole list_area, but we have to render it first and then render inner chunks
    let inner_list_area = list_block.inner(list_area);
    f.render_widget(list_block, list_area);

    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner_list_area);

    f.render_widget(header, list_chunks[0]);

    let visible_items = list_chunks[1].height as usize;
    if selected < *scroll {
        *scroll = selected;
    } else if selected >= *scroll + visible_items && visible_items > 0 {
        *scroll = selected.saturating_sub(visible_items - 1);
    }

    let mut list_lines = Vec::new();
    for (i, item) in snap
        .items
        .iter()
        .enumerate()
        .skip(*scroll)
        .take(visible_items)
    {
        let prefix = if i == selected { " > " } else { "   " };
        let icon = if item.is_dir { "📁" } else { "📄" };
        let style = if i == selected {
            Style::default().fg(theme.accent)
        } else if item.is_dir {
            Style::default().fg(theme.text)
        } else {
            Style::default().fg(theme.dim)
        };
        list_lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(theme.accent)),
            Span::styled(format!("{} ", icon), style),
            Span::styled(&item.name, style),
        ]));
    }

    f.render_widget(Paragraph::new(list_lines), list_chunks[1]);

    // Preview
    let mut preview_lines = Vec::new();
    if num_items > 0 {
        if let Some(preview) = &snap.preview {
            match preview {
                PreviewContent::Text(text) => {
                    for line in text.lines() {
                        preview_lines.push(Line::from(Span::styled(
                            line,
                            Style::default().fg(theme.text),
                        )));
                    }
                }
                PreviewContent::Image(dynamic_image) => {
                    let (w, h) = image::GenericImageView::dimensions(dynamic_image);
                    preview_lines.push(Line::from(Span::styled(
                        format!("Image {}x{}", w, h),
                        Style::default().fg(theme.dim),
                    )));
                    preview_lines.push(Line::from(""));

                    let mut img_area = preview_area;
                    img_area.x += 1;
                    img_area.y += 3;
                    img_area.width = img_area.width.saturating_sub(2);
                    img_area.height = img_area.height.saturating_sub(4);

                    if !app.panel_states.pixel_images && app.image_picker.is_some() {
                        if let Some(picker) = &app.image_picker {
                            let path_key = snap.items.get(*selected_idx).map(|i| i.path.to_string_lossy().to_string()).unwrap_or_default();
                            let protocol = app.image_protocols.entry(path_key).or_insert_with(|| {
                                picker.new_resize_protocol(dynamic_image.clone())
                            });
                            let image_widget = ratatui_image::StatefulImage::new();
                            f.render_stateful_widget(image_widget, img_area, protocol);
                        }
                    } else {
                        // Fallback to pixelated (braille)
                        let braille_lines = crate::widgets::braille_image::render_image(
                            dynamic_image,
                            img_area.width,
                            img_area.height,
                        );
                        preview_lines.extend(braille_lines);
                    }
                }
            }
        }
    }

    f.render_widget(
        Paragraph::new(preview_lines).wrap(Wrap { trim: false }),
        preview_area,
    );

    if let Some(i_area) = input_area {
        let (title, content) = if app.panel_states.files_rename_input_active {
            ("Rename (Enter to save)", &app.panel_states.files_rename_input)
        } else {
            ("Search (Esc to cancel)", &app.panel_states.files_search_input)
        };
        let b = Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(theme.accent))
            .title(title);
        f.render_widget(Paragraph::new(format!("{}█", content)).block(b), i_area);
    }
}
