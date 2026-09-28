use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::cell::RefCell;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;

use crate::theme::Theme;


static ASYNC_RESULT: LazyLock<Arc<Mutex<Option<CachedMedia>>>> = LazyLock::new(|| Arc::new(Mutex::new(None)));
static ASYNC_LOADING_PATH: LazyLock<Arc<Mutex<String>>> = LazyLock::new(|| Arc::new(Mutex::new(String::new())));

thread_local! {
    static CACHED_IMAGE: RefCell<Option<CachedMedia>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug, Default)]
pub struct MediaInfo {
    pub is_fallback: bool,
    pub is_dir: bool,
    pub current_idx: usize,
    pub total_images: usize,
    pub file_name: String,
    pub file_path: String,
    pub orig_width: u32,
    pub orig_height: u32,
    pub file_size_kb: u64,
    pub palette: Vec<Color>,
}

pub fn current_media_info() -> Option<MediaInfo> {
    CACHED_IMAGE.with(|c| c.borrow().as_ref().map(|cache| cache.info.clone()))
}

struct CachedMedia {
    path: String,
    area_width: u16,
    area_height: u16,
    lines: Vec<Line<'static>>,
    img: Option<image::DynamicImage>,

    // Slideshow state
    is_dir: bool,
    images: Vec<String>,
    current_idx: usize,
    last_tick: u64,
    info: MediaInfo,
}

pub fn render(f: &mut Frame, area: Rect, app: &mut crate::app::App, path: &str, tick: u64) {
    let theme = &app.theme;
    // If path is empty, we will use the built-in default image
    let actual_path = if path.is_empty() {
        "default_fallback"
    } else {
        path
    };

    if area.width < 10 || area.height < 5 {
        return;
    }

    let needs_update = CACHED_IMAGE.with(|c| {
        let cache = c.borrow();
        if cache.is_none() {
            return true;
        }
        let cache = cache.as_ref().unwrap();
        if cache.path != actual_path
            || cache.area_width != area.width
            || cache.area_height != area.height
        {
            return true;
        }
        if cache.is_dir {
            // switch every ~150 ticks (5 sec at 30fps)
            if tick > cache.last_tick + 150 {
                return true;
            }
        }
        false
    });

    if needs_update {
        let loading_path = {
            let p = ASYNC_LOADING_PATH.lock().unwrap();
            p.clone()
        };
        let target_key = format!("{}|{}x{}", actual_path, area.width, area.height);

        // Check if the result is ready
        let mut ready = false;
        {
            let mut res = ASYNC_RESULT.lock().unwrap();
            if let Some(r) = res.take() {
                CACHED_IMAGE.with(|c| {
                    *c.borrow_mut() = Some(r);
                });
                ready = true;
                *ASYNC_LOADING_PATH.lock().unwrap() = String::new();
            }
        }

        if !ready && loading_path != target_key {
            *ASYNC_LOADING_PATH.lock().unwrap() = target_key.clone();
            let expanded_path = if let Some(stripped) = path.strip_prefix("~/") {
                let home = std::env::var("HOME").unwrap_or_else(|_| "".to_string());
                format!("{}/{}", home, stripped)
            } else if path == "~" {
                std::env::var("HOME").unwrap_or_else(|_| "".to_string())
            } else {
                path.to_string()
            };

            let path_buf = std::path::Path::new(&expanded_path);
            let mut is_dir = false;
            let mut images = Vec::new();
            let mut current_idx = 0;
            let mut file_to_load = expanded_path.clone();

            CACHED_IMAGE.with(|c| {
                if let Some(cache) = c.borrow().as_ref() {
                    if cache.path == actual_path
                        && cache.area_width == area.width
                        && cache.area_height == area.height
                    {
                        is_dir = cache.is_dir;
                        images = cache.images.clone();
                        current_idx = cache.current_idx;
                    }
                }
            });

            if path_buf.is_dir() {
                is_dir = true;
                if images.is_empty() {
                    if let Ok(entries) = std::fs::read_dir(path_buf) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if p.is_file() {
                                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                                    let ext = ext.to_lowercase();
                                    if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "gif" || ext == "webp" {
                                        images.push(p.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }
                    }
                    images.sort();
                }
                if !images.is_empty() {
                    current_idx = (current_idx + 1) % images.len();
                    file_to_load = images[current_idx].clone();
                }
            }
            
            let actual_path_clone = actual_path.to_string();
            let path_clone = path.to_string();
            let target_w = area.width as u32;
            let target_h = (area.height * 2) as u32;
            let theme_red = theme.red;
            
            thread::spawn(move || {
                let mut lines = Vec::new();
                let load_result = if actual_path_clone == "default_fallback" {
                    image::load_from_memory(include_bytes!("../assets/default_media.jpg"))
                } else {
                    image::open(&file_to_load)
                };

                let mut cached_img = None;
                let mut orig_width = 0;
                let mut orig_height = 0;
                let mut palette = Vec::new();

                match load_result {
                    Ok(img) => {
                        orig_width = img.width();
                        orig_height = img.height();
                        cached_img = Some(img.clone());

                        let rgb_img = img.to_rgb8();
                        let thumb = image::imageops::thumbnail(&rgb_img, target_w, target_h);
                        let (w, h) = thumb.dimensions();

                        if w > 0 && h > 0 {
                            let sample_coords = [
                                (w / 4, h / 4), (w / 2, h / 3), (3 * w / 4, h / 4),
                                (w / 3, h / 2), (w / 2, h / 2), (2 * w / 3, 2 * h / 3),
                            ];
                            for (sx, sy) in sample_coords {
                                let p = thumb.get_pixel(sx.min(w - 1), sy.min(h - 1));
                                palette.push(Color::Rgb(p[0], p[1], p[2]));
                            }
                        }

                        for y in (0..h).step_by(2) {
                            let mut spans = Vec::new();
                            for x in 0..w {
                                let top = thumb.get_pixel(x, y);
                                let bottom = if y + 1 < h { Some(thumb.get_pixel(x, y + 1)) } else { None };
                                let top_color = Color::Rgb(top[0], top[1], top[2]);
                                let style = ratatui::style::Style::default().fg(top_color);
                                let style = if let Some(b) = bottom {
                                    style.bg(Color::Rgb(b[0], b[1], b[2]))
                                } else {
                                    style
                                };
                                spans.push(Span::styled("▀", style));
                            }
                            lines.push(Line::from(spans));
                        }
                    }
                    Err(e) => {
                        lines = vec![Line::from(vec![Span::styled(
                            format!("Failed to load {}: {}", file_to_load, e),
                            ratatui::style::Style::default().fg(theme_red),
                        )])];
                    }
                }

                let file_name = if actual_path_clone == "default_fallback" {
                    "porsche_911_dusk.jpg".to_string()
                } else {
                    std::path::Path::new(&file_to_load)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "custom_media".to_string())
                };

                let file_size_kb = if actual_path_clone == "default_fallback" {
                    166
                } else {
                    std::fs::metadata(&file_to_load).map(|m| m.len().div_ceil(1024)).unwrap_or(0)
                };

                let info = MediaInfo {
                    is_fallback: actual_path_clone == "default_fallback",
                    is_dir, current_idx, total_images: if is_dir { images.len() } else { 1 },
                    file_name, file_path: file_to_load,
                    orig_width, orig_height, file_size_kb, palette,
                };

                let result = CachedMedia {
                    path: path_clone,
                    area_width: target_w as u16,
                    area_height: target_h as u16 / 2,
                    lines,
                    img: cached_img,
                    is_dir,
                    images,
                    current_idx,
                    last_tick: tick,
                    info,
                };
                *ASYNC_RESULT.lock().unwrap() = Some(result);
                crate::anim::request(60);
            });
        }
    }

    CACHED_IMAGE.with(|c| {
        if let Some(cache) = c.borrow().as_ref() {
            let mut rendered = false;
            if !app.panel_states.pixel_images {
                if let (Some(picker), Some(tx), Some(img)) = (
                    &app.image_picker,
                    &app.image_resize_tx,
                    &cache.img,
                ) {
                    let path_key = cache.path.clone();
                    let img_clone = img.clone();
                    let picker_clone = picker.clone();
                    let tx_clone = tx.clone();
                    let proto = app.thread_protocols.entry(path_key).or_insert_with(|| {
                        let stateful = picker_clone.new_resize_protocol(img_clone);
                        ratatui_image::thread::ThreadProtocol::new(tx_clone, Some(stateful))
                    });
                    let image_widget = ratatui_image::StatefulImage::new();
                    f.render_stateful_widget(image_widget, area, proto);
                    rendered = true;
                }
            }
            if !rendered {
                let p = Paragraph::new(cache.lines.clone()).alignment(Alignment::Center);
                f.render_widget(p, area);
            }
        }
    });
}
