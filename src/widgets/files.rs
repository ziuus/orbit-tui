use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::monitors::files::{self, FileItem, PreviewContent};

/// Nerd-font icon for a file item (2 chars + space = 3 chars wide total).
fn file_icon(item: &FileItem) -> &'static str {
    if item.is_dir {
        return " ";
    }
    let ext = item
        .path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    match ext.as_str() {
        "rs" => " ",
        "toml" | "yaml" | "yml" | "json" | "ron" => " ",
        "md" | "txt" | "rst" => "󱪖 ",
        "sh" | "bash" | "zsh" | "fish" => " ",
        "py" => " ",
        "js" | "ts" | "jsx" | "tsx" => " ",
        "html" | "htm" => " ",
        "css" | "scss" | "sass" => " ",
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp" => " ",
        "mp4" | "mkv" | "avi" | "mov" | "webm" => "󰕧 ",
        "mp3" | "ogg" | "wav" | "flac" | "opus" => " ",
        "pdf" => " ",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "zst" | "rar" | "7z" => " ",
        "deb" | "rpm" | "pkg" => " ",
        "wasm" => " ",
        "lock" => " ",
        "log" => "󱂬 ",
        "env" | "cfg" | "conf" | "ini" => " ",
        "git" => " ",
        _ => " ",
    }
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "     -".to_string();
    }
    if bytes < 1024 {
        format!("{:>4}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:>3}KB", bytes / 1024)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:>3}MB", bytes / (1024 * 1024))
    } else {
        format!("{:>3}GB", bytes / (1024 * 1024 * 1024))
    }
}

/// Shorten a path for display: ~/Projects/foo → ~/P/foo
fn short_path(p: &std::path::Path) -> String {
    let s = p.to_string_lossy();
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() {
        if let Some(rel) = s.strip_prefix(&home) {
            let rel = rel.trim_start_matches('/');
            // Show only last 2 components if path is deep
            let parts: Vec<&str> = rel.split('/').collect();
            if parts.len() <= 3 {
                return format!("~/{}", rel);
            }
            // Abbreviate middle: ~/a/b/.../last
            return format!("~/{}/…/{}", parts[0], parts[parts.len() - 1]);
        }
    }
    if s.len() > 30 {
        format!("…{}", &s[s.len() - 28..])
    } else {
        s.to_string()
    }
}

pub fn render(f: &mut Frame, area: Rect, app: &mut crate::app::App) {
    let theme = &app.theme;
    let is_focused = app.focused_panel == Some(crate::app::PanelId::Files);
    let mut selected_idx = app.panel_states.files_selected;
    let mut scroll = app.panel_states.files_scroll;
    let snap = files::snapshot();

    // ── Layout: left = list, right = preview ──────────────────────────────
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    // Input bar at bottom of list column when active
    let has_input = app.panel_states.files_search_input_active
        || app.panel_states.files_rename_input_active
        || app.panel_states.files_mkdir_input_active;

    let (list_area, input_area) = if has_input {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(chunks[0]);
        (v[0], Some(v[1]))
    } else {
        (chunks[0], None)
    };
    let preview_area = chunks[1];

    let items = app.get_filtered_files(&snap);
    let num_items = items.len();

    // Clamp selection
    let max_idx = num_items.saturating_sub(1);
    selected_idx = selected_idx.min(max_idx);
    let selected = selected_idx;
    app.panel_states.files_selected = selected;

    // ── List border with divider ───────────────────────────────────────────
    let border_color = if is_focused {
        theme.accent
    } else {
        theme.surface
    };
    let list_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(border_color));
    let inner_list_area = list_block.inner(list_area);
    f.render_widget(list_block, list_area);

    // ── Breadcrumb path bar (2 rows: path + blank separator) ──────────────
    let path_str = short_path(&snap.current_dir);
    // truncate to fit
    let avail_w = inner_list_area.width as usize;
    let path_disp = if path_str.chars().count() > avail_w {
        let skip = path_str.chars().count() - avail_w + 1;
        format!("…{}", path_str.chars().skip(skip).collect::<String>())
    } else {
        path_str
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" ", Style::default().fg(theme.accent)),
            Span::styled(
                path_disp,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            "─".repeat(inner_list_area.width as usize),
            Style::default().fg(theme.surface),
        )]),
    ]);

    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner_list_area);

    f.render_widget(header, list_chunks[0]);

    // ── File list ──────────────────────────────────────────────────────────
    let visible_items = list_chunks[1].height as usize;
    if selected < scroll {
        scroll = selected;
    } else if selected >= scroll + visible_items && visible_items > 0 {
        scroll = selected.saturating_sub(visible_items - 1);
    }
    app.panel_states.files_scroll = scroll;
    crate::screens::hit(
        list_chunks[1],
        crate::screens::Hit::Rows {
            panel: crate::app::PanelId::Files,
            first: scroll,
        },
    );

    let name_w = (avail_w).saturating_sub(3 + 6); // icon(2) + space(1) + size(5) + space(1)
    let name_w = name_w.max(8);

    let mut list_lines: Vec<Line> = Vec::new();

    if num_items == 0 && !app.panel_states.files_search_input.is_empty() {
        list_lines.push(Line::from(Span::styled(
            " no matches",
            Style::default().fg(theme.dim),
        )));
    } else if num_items == 0 {
        list_lines.push(Line::from(Span::styled(
            " empty directory",
            Style::default().fg(theme.dim),
        )));
    }

    for (i, item) in items.iter().enumerate().skip(scroll).take(visible_items) {
        let is_sel = i == selected;
        let icon = file_icon(item);
        let name_trunc = crate::widgets::meter::ellipsize(&item.name, name_w);
        let name_pad = format!("{:<w$}", name_trunc, w = name_w);
        let size_str = if item.is_dir {
            "  dir".to_string()
        } else {
            format_size(item.size)
        };

        let (icon_style, name_style, size_style) = if is_sel {
            (
                Style::default().fg(theme.bg).bg(theme.accent),
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(theme.bg).bg(theme.accent),
            )
        } else if item.is_dir {
            (
                Style::default().fg(theme.accent),
                Style::default().fg(theme.text),
                Style::default().fg(theme.dim),
            )
        } else {
            (
                Style::default().fg(theme.dim),
                Style::default().fg(theme.text),
                Style::default().fg(theme.dim),
            )
        };

        list_lines.push(Line::from(vec![
            Span::styled(format!(" {}", icon), icon_style),
            Span::styled(name_pad, name_style),
            Span::styled(format!(" {} ", size_str), size_style),
        ]));
    }

    f.render_widget(Paragraph::new(list_lines), list_chunks[1]);

    // ── Input bar ─────────────────────────────────────────────────────────
    if let Some(i_area) = input_area {
        let (title, content) = if app.panel_states.files_mkdir_input_active {
            (
                "New folder — Enter create · Esc cancel",
                &app.panel_states.files_mkdir_input,
            )
        } else if app.panel_states.files_rename_input_active {
            (
                "Rename — Enter save · Esc cancel",
                &app.panel_states.files_rename_input,
            )
        } else {
            (
                "/search — Enter/Esc done",
                &app.panel_states.files_search_input,
            )
        };
        let b = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(Span::styled(
                format!(" {} ", title),
                Style::default().fg(theme.dim),
            ));
        f.render_widget(Paragraph::new(format!(" {}▌", content)).block(b), i_area);
    }

    // ── Preview panel ─────────────────────────────────────────────────────
    let mut preview_lines: Vec<Line> = Vec::new();

    if num_items == 0 {
        // nothing selected
    } else if let Some(item) = items.get(selected) {
        // Preview header: path + type
        let ext = item
            .path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let kind = if item.is_dir {
            "directory".to_string()
        } else if ext.is_empty() {
            "file".to_string()
        } else {
            ext.to_string()
        };
        let size_label = if item.is_dir {
            String::new()
        } else {
            format!(" · {}", format_size(item.size).trim())
        };
        preview_lines.push(Line::from(vec![Span::styled(
            format!(" {} {}", file_icon(item), item.name),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )]));
        preview_lines.push(Line::from(vec![Span::styled(
            format!(" {}{}", kind, size_label),
            Style::default().fg(theme.dim),
        )]));
        preview_lines.push(Line::from(vec![Span::styled(
            format!(
                " {}",
                "─".repeat(preview_area.width.saturating_sub(2) as usize)
            ),
            Style::default().fg(theme.surface),
        )]));

        if let Some(preview) = &snap.preview {
            match preview {
                PreviewContent::Text(text) => {
                    for line in text
                        .lines()
                        .take(preview_area.height.saturating_sub(4) as usize)
                    {
                        // Basic syntax hint: leading # → accent, --- → dim, else text
                        let style = if line.starts_with("# ")
                            || line.starts_with("## ")
                            || line.starts_with("### ")
                        {
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD)
                        } else if line.starts_with("//")
                            || line.starts_with('#')
                            || line.starts_with("--")
                        {
                            Style::default().fg(theme.dim)
                        } else if line.starts_with("fn ")
                            || line.starts_with("pub ")
                            || line.starts_with("struct ")
                            || line.starts_with("impl ")
                            || line.starts_with("use ")
                            || line.starts_with("mod ")
                        {
                            Style::default().fg(theme.yellow)
                        } else {
                            Style::default().fg(theme.text)
                        };
                        preview_lines.push(Line::from(Span::styled(line, style)));
                    }
                }
                PreviewContent::Image(dynamic_image) => {
                    let (w, h) = image::GenericImageView::dimensions(dynamic_image);
                    preview_lines.push(Line::from(Span::styled(
                        format!(" {}×{} image", w, h),
                        Style::default().fg(theme.dim),
                    )));
                    preview_lines.push(Line::from(""));

                    let mut img_area = preview_area;
                    img_area.x += 1;
                    img_area.y += 4;
                    img_area.width = img_area.width.saturating_sub(2);
                    img_area.height = img_area.height.saturating_sub(5);

                    if !app.panel_states.pixel_images && app.image_picker.is_some() {
                        if let Some(picker) = &app.image_picker {
                            let path_key = snap
                                .items
                                .get(selected_idx)
                                .map(|i| i.path.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let protocol =
                                app.image_protocols.entry(path_key).or_insert_with(|| {
                                    picker.new_resize_protocol(dynamic_image.clone())
                                });
                            let image_widget = ratatui_image::StatefulImage::new();
                            f.render_stateful_widget(image_widget, img_area, protocol);
                        }
                    } else {
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
}
