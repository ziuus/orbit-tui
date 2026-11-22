use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::cell::RefCell;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;

pub type ImageBuffer = image::ImageBuffer<image::Rgb<u8>, Vec<u8>>;

static ASYNC_RESULT: LazyLock<Arc<Mutex<Option<CachedMedia>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));
static ASYNC_LOADING_PATH: LazyLock<Arc<Mutex<String>>> =
    LazyLock::new(|| Arc::new(Mutex::new(String::new())));

thread_local! {
    static CACHED_IMAGE: RefCell<Option<CachedMedia>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TransitionMode {
    #[default]
    Crossfade,
    Dither,
}

#[derive(Clone, Debug)]
pub struct PlaybackFrame {
    pub thumb: ImageBuffer,
    pub delay_ms: u32,
    pub lines: Vec<Line<'static>>,
}

#[derive(Clone, Debug)]
pub struct ActiveTransition {
    pub from_thumb: ImageBuffer,
    pub to_thumb: ImageBuffer,
    pub start_time: std::time::Instant,
    pub duration_ms: u64,
    pub mode: TransitionMode,
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
    pub is_animated: bool,
    pub frame_count: usize,
    pub current_frame: usize,
    pub fps: f32,
}

pub fn current_media_info() -> Option<MediaInfo> {
    CACHED_IMAGE.with(|c| c.borrow().as_ref().map(|cache| cache.info.clone()))
}

/// Reset cached media (useful for tests or hard reloads)
pub fn reset_cached_media() {
    CACHED_IMAGE.with(|c| {
        *c.borrow_mut() = None;
    });
    *ASYNC_RESULT.lock().unwrap() = None;
    *ASYNC_LOADING_PATH.lock().unwrap() = String::new();
}

struct CachedMedia {
    path: String,
    area_width: u16,
    area_height: u16,
    lines: Vec<Line<'static>>,
    img: Option<image::DynamicImage>,
    thumb: ImageBuffer,

    // Slideshow state
    is_dir: bool,
    images: Vec<String>,
    current_idx: usize,
    last_tick: u64,

    // Animated playback state
    is_animated: bool,
    frames: Vec<PlaybackFrame>,
    current_frame: usize,
    last_frame_time: std::time::Instant,

    // Smooth pixel transition state
    transition: Option<ActiveTransition>,

    info: MediaInfo,
}

/// Standard 4x4 Bayer dither threshold matrix normalized to [0..1]
const BAYER_4X4: [[f32; 4]; 4] = [
    [0.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0, 10.0 / 16.0],
    [12.0 / 16.0, 4.0 / 16.0, 14.0 / 16.0, 6.0 / 16.0],
    [3.0 / 16.0, 11.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0],
    [15.0 / 16.0, 7.0 / 16.0, 13.0 / 16.0, 5.0 / 16.0],
];

/// Smooth linear crossfade interpolation between two frame buffers.
pub fn interpolate_pixels(from: &ImageBuffer, to: &ImageBuffer, progress: f32) -> ImageBuffer {
    if to.width() == 0 || to.height() == 0 {
        return to.clone();
    }
    if from.width() == 0 || from.height() == 0 {
        return to.clone();
    }

    let t = progress.clamp(0.0, 1.0);
    let tw = to.width().max(1);
    let th = to.height().max(1);
    let fw = from.width().max(1);
    let fh = from.height().max(1);

    if t <= 0.0 && fw == tw && fh == th {
        return from.clone();
    }
    if t >= 1.0 {
        return to.clone();
    }

    let mut out = ImageBuffer::new(tw, th);
    let inv_t = 1.0 - t;

    for y in 0..th {
        let fy = if fh == th {
            y
        } else {
            ((y as u64 * fh as u64) / th as u64).min(fh.saturating_sub(1) as u64) as u32
        };
        for x in 0..tw {
            let fx = if fw == tw {
                x
            } else {
                ((x as u64 * fw as u64) / tw as u64).min(fw.saturating_sub(1) as u64) as u32
            };
            let pf = from.get_pixel(fx, fy);
            let pt = to.get_pixel(x, y);

            let r = (pf[0] as f32 * inv_t + pt[0] as f32 * t)
                .round()
                .clamp(0.0, 255.0) as u8;
            let g = (pf[1] as f32 * inv_t + pt[1] as f32 * t)
                .round()
                .clamp(0.0, 255.0) as u8;
            let b = (pf[2] as f32 * inv_t + pt[2] as f32 * t)
                .round()
                .clamp(0.0, 255.0) as u8;

            out.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    out
}

/// Ordered Bayer dither dissolve transition between two frame buffers.
pub fn dither_dissolve(from: &ImageBuffer, to: &ImageBuffer, progress: f32) -> ImageBuffer {
    if to.width() == 0 || to.height() == 0 {
        return to.clone();
    }
    if from.width() == 0 || from.height() == 0 {
        return to.clone();
    }

    let t = progress.clamp(0.0, 1.0);
    let tw = to.width().max(1);
    let th = to.height().max(1);
    let fw = from.width().max(1);
    let fh = from.height().max(1);

    if t <= 0.0 && fw == tw && fh == th {
        return from.clone();
    }
    if t >= 1.0 {
        return to.clone();
    }

    let mut out = ImageBuffer::new(tw, th);

    for y in 0..th {
        let fy = if fh == th {
            y
        } else {
            ((y as u64 * fh as u64) / th as u64).min(fh.saturating_sub(1) as u64) as u32
        };
        let row = (y % 4) as usize;
        for x in 0..tw {
            let fx = if fw == tw {
                x
            } else {
                ((x as u64 * fw as u64) / tw as u64).min(fw.saturating_sub(1) as u64) as u32
            };
            let col = (x % 4) as usize;
            let threshold = BAYER_4X4[row][col] + (0.5 / 16.0);

            let pixel = if t >= threshold {
                *to.get_pixel(x, y)
            } else {
                *from.get_pixel(fx, fy)
            };
            out.put_pixel(x, y, pixel);
        }
    }
    out
}

/// Apply the requested pixel transition mode.
pub fn transition_pixels(
    from: &ImageBuffer,
    to: &ImageBuffer,
    progress: f32,
    mode: TransitionMode,
) -> ImageBuffer {
    match mode {
        TransitionMode::Crossfade => interpolate_pixels(from, to, progress),
        TransitionMode::Dither => dither_dissolve(from, to, progress),
    }
}

/// Render an RGB image buffer to Ratatui Lines using Unicode half-blocks ('▀').
/// 1 character width = 1 pixel; 1 character height = 2 vertical subpixels.
pub fn image_to_half_blocks(thumb: &ImageBuffer) -> Vec<Line<'static>> {
    let (w, h) = thumb.dimensions();
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let mut lines = Vec::with_capacity(h.div_ceil(2) as usize);
    for y in (0..h).step_by(2) {
        let mut spans = Vec::with_capacity(w as usize);
        for x in 0..w {
            let top = thumb.get_pixel(x, y);
            let bottom = if y + 1 < h {
                Some(thumb.get_pixel(x, y + 1))
            } else {
                None
            };
            let top_color = Color::Rgb(top[0], top[1], top[2]);
            let mut style = ratatui::style::Style::default().fg(top_color);
            if let Some(b) = bottom {
                style = style.bg(Color::Rgb(b[0], b[1], b[2]));
            }
            spans.push(Span::styled("▀", style));
        }
        lines.push(Line::from(spans));
    }
    lines
}

/// Extract representative palette colors from sample coordinates.
pub fn extract_palette(thumb: &ImageBuffer) -> Vec<Color> {
    let (w, h) = thumb.dimensions();
    let mut palette = Vec::new();
    if w > 0 && h > 0 {
        let sample_coords = [
            (w / 4, h / 4),
            (w / 2, h / 3),
            (3 * w / 4, h / 4),
            (w / 3, h / 2),
            (w / 2, h / 2),
            (2 * w / 3, 2 * h / 3),
        ];
        for (sx, sy) in sample_coords {
            let p = thumb.get_pixel(sx.min(w - 1), sy.min(h - 1));
            palette.push(Color::Rgb(p[0], p[1], p[2]));
        }
    }
    palette
}

/// Discrete frame index advancement for animation loops.
pub fn step_frame(current_frame: usize, total_frames: usize) -> usize {
    if total_frames <= 1 {
        0
    } else {
        (current_frame + 1) % total_frames
    }
}

/// Check if enough time has passed to step to the next animated frame.
pub fn should_step_frame(elapsed_ms: u64, delay_ms: u32) -> bool {
    elapsed_ms >= delay_ms as u64
}

/// Check if slideshow interval has elapsed.
pub fn should_switch_slide(current_tick: u64, last_tick: u64, interval_ticks: u64) -> bool {
    current_tick > last_tick + interval_ticks
}

/// Calculate normalized transition progress in [0..1].
pub fn calculate_transition_progress(elapsed_ms: u64, duration_ms: u64) -> f32 {
    if duration_ms == 0 {
        1.0
    } else {
        (elapsed_ms as f32 / duration_ms as f32).clamp(0.0, 1.0)
    }
}

/// Check for GIF87a or GIF89a magic signature.
pub fn is_gif_data(data: &[u8]) -> bool {
    data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")
}

pub struct DecodedMedia {
    pub orig_width: u32,
    pub orig_height: u32,
    pub cached_img: Option<image::DynamicImage>,
    pub palette: Vec<Color>,
    pub frames: Vec<PlaybackFrame>,
    pub is_animated: bool,
}

/// Pure Rust media decoder supporting static images and multi-frame animated sequences (GIF).
pub fn decode_media_bytes(
    bytes: &[u8],
    file_name: &str,
    target_w: u32,
    target_h: u32,
) -> Result<DecodedMedia, String> {
    if bytes.is_empty() {
        return Err(format!("Empty media bytes for {}", file_name));
    }

    let is_gif = is_gif_data(bytes) || file_name.to_lowercase().ends_with(".gif");
    if is_gif {
        use image::codecs::gif::GifDecoder;
        use image::AnimationDecoder;
        let cursor = std::io::Cursor::new(bytes);
        if let Ok(decoder) = GifDecoder::new(cursor) {
            if let Ok(raw_frames) = decoder.into_frames().collect::<Result<Vec<_>, _>>() {
                if !raw_frames.is_empty() {
                    let orig_width = raw_frames[0].buffer().width();
                    let orig_height = raw_frames[0].buffer().height();
                    let is_animated = raw_frames.len() > 1;

                    let mut frames = Vec::with_capacity(raw_frames.len());
                    for f in raw_frames {
                        let (num, denom) = f.delay().numer_denom_ms();
                        let delay_ms = if denom == 0 || num == 0 {
                            100
                        } else {
                            (num / denom).max(20)
                        };
                        let dyn_img = image::DynamicImage::ImageRgba8(f.into_buffer());
                        let rgb = dyn_img.to_rgb8();
                        let thumb = image::imageops::thumbnail(&rgb, target_w, target_h);
                        let lines = image_to_half_blocks(&thumb);
                        frames.push(PlaybackFrame {
                            thumb,
                            delay_ms,
                            lines,
                        });
                    }

                    let palette = if !frames.is_empty() {
                        extract_palette(&frames[0].thumb)
                    } else {
                        Vec::new()
                    };

                    let cached_img = image::load_from_memory(bytes).ok();

                    return Ok(DecodedMedia {
                        orig_width,
                        orig_height,
                        cached_img,
                        palette,
                        frames,
                        is_animated,
                    });
                }
            }
        }
    }

    // Standard static image decoding (PNG, JPEG, WEBP, etc.)
    match image::load_from_memory(bytes) {
        Ok(img) => {
            let orig_width = img.width();
            let orig_height = img.height();
            let rgb_img = img.to_rgb8();
            let thumb = image::imageops::thumbnail(&rgb_img, target_w, target_h);
            let palette = extract_palette(&thumb);
            let lines = image_to_half_blocks(&thumb);
            let frames = vec![PlaybackFrame {
                thumb,
                delay_ms: 0,
                lines,
            }];

            Ok(DecodedMedia {
                orig_width,
                orig_height,
                cached_img: Some(img),
                palette,
                frames,
                is_animated: false,
            })
        }
        Err(e) => Err(format!("Failed to load {}: {}", file_name, e)),
    }
}

pub fn render(f: &mut Frame, area: Rect, app: &mut crate::app::App, path: &str, tick: u64) {
    let theme = &app.theme;
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
            if should_switch_slide(tick, cache.last_tick, 150) {
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
            if let Some(mut r) = res.take() {
                CACHED_IMAGE.with(|c| {
                    let mut borrow = c.borrow_mut();
                    if let Some(ref old) = *borrow {
                        // Smooth transition between media if both have valid images
                        if old.thumb.width() > 0 && r.thumb.width() > 0 {
                            r.transition = Some(ActiveTransition {
                                from_thumb: old.thumb.clone(),
                                to_thumb: r.thumb.clone(),
                                start_time: std::time::Instant::now(),
                                duration_ms: 500,
                                mode: TransitionMode::Crossfade,
                            });
                        }
                    }
                    *borrow = Some(r);
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
                                    if ext == "png"
                                        || ext == "jpg"
                                        || ext == "jpeg"
                                        || ext == "gif"
                                        || ext == "webp"
                                    {
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
                let load_result = if actual_path_clone == "default_fallback" {
                    decode_media_bytes(
                        include_bytes!("../assets/default_media.jpg"),
                        "default_fallback",
                        target_w,
                        target_h,
                    )
                } else {
                    match std::fs::read(&file_to_load) {
                        Ok(bytes) => decode_media_bytes(&bytes, &file_to_load, target_w, target_h),
                        Err(e) => Err(format!("Failed to read {}: {}", file_to_load, e)),
                    }
                };

                let (
                    lines,
                    cached_img,
                    orig_width,
                    orig_height,
                    palette,
                    frames,
                    is_animated,
                    thumb,
                ) = match load_result {
                    Ok(decoded) => {
                        let thumb = decoded.frames[0].thumb.clone();
                        let lines = decoded.frames[0].lines.clone();
                        (
                            lines,
                            decoded.cached_img,
                            decoded.orig_width,
                            decoded.orig_height,
                            decoded.palette,
                            decoded.frames,
                            decoded.is_animated,
                            thumb,
                        )
                    }
                    Err(e) => {
                        let err_lines = vec![Line::from(vec![Span::styled(
                            e,
                            ratatui::style::Style::default().fg(theme_red),
                        )])];
                        let blank_thumb = ImageBuffer::new(1, 1);
                        (
                            err_lines,
                            None,
                            0,
                            0,
                            Vec::new(),
                            Vec::new(),
                            false,
                            blank_thumb,
                        )
                    }
                };

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
                    std::fs::metadata(&file_to_load)
                        .map(|m| m.len().div_ceil(1024))
                        .unwrap_or(0)
                };

                let fps = if is_animated && !frames.is_empty() {
                    let avg_delay: u32 =
                        frames.iter().map(|f| f.delay_ms).sum::<u32>() / frames.len() as u32;
                    if avg_delay > 0 {
                        1000.0 / avg_delay as f32
                    } else {
                        10.0
                    }
                } else {
                    0.0
                };

                let info = MediaInfo {
                    is_fallback: actual_path_clone == "default_fallback",
                    is_dir,
                    current_idx,
                    total_images: if is_dir { images.len() } else { 1 },
                    file_name,
                    file_path: file_to_load,
                    orig_width,
                    orig_height,
                    file_size_kb,
                    palette,
                    is_animated,
                    frame_count: frames.len(),
                    current_frame: 0,
                    fps,
                };

                let result = CachedMedia {
                    path: path_clone,
                    area_width: target_w as u16,
                    area_height: target_h as u16 / 2,
                    lines,
                    img: cached_img,
                    thumb,
                    is_dir,
                    images,
                    current_idx,
                    last_tick: tick,
                    is_animated,
                    frames,
                    current_frame: 0,
                    last_frame_time: std::time::Instant::now(),
                    transition: None,
                    info,
                };
                *ASYNC_RESULT.lock().unwrap() = Some(result);
                crate::anim::request(60);
            });
        }
    }

    CACHED_IMAGE.with(|c| {
        let mut borrow = c.borrow_mut();
        let Some(cache) = borrow.as_mut() else {
            return;
        };

        let mut current_lines = cache.lines.clone();
        let mut is_transitioning = false;

        // 1. Process active transition
        if let Some(ref trans) = cache.transition {
            let elapsed = trans.start_time.elapsed().as_millis() as u64;
            let progress = calculate_transition_progress(elapsed, trans.duration_ms);
            if progress < 1.0 {
                is_transitioning = true;
                crate::anim::request(30);
                let interp =
                    transition_pixels(&trans.from_thumb, &trans.to_thumb, progress, trans.mode);
                current_lines = image_to_half_blocks(&interp);
            } else {
                cache.transition = None;
            }
        }

        // 2. Process animated frame playback (when not transitioning)
        if !is_transitioning && cache.is_animated && cache.frames.len() > 1 {
            let delay = cache.frames[cache.current_frame].delay_ms;
            let elapsed = cache.last_frame_time.elapsed().as_millis() as u64;
            if should_step_frame(elapsed, delay) {
                cache.current_frame = step_frame(cache.current_frame, cache.frames.len());
                cache.last_frame_time = std::time::Instant::now();
                cache.lines = cache.frames[cache.current_frame].lines.clone();
                cache.thumb = cache.frames[cache.current_frame].thumb.clone();
                cache.info.current_frame = cache.current_frame;
            }
            current_lines = cache.frames[cache.current_frame].lines.clone();
            let fps = (1000 / delay.max(16)).clamp(10, 60);
            crate::anim::request(fps);
        }

        // 3. Render
        let mut rendered = false;
        if !app.panel_states.pixel_images && !is_transitioning && !cache.is_animated {
            if let (Some(picker), Some(tx), Some(img)) =
                (&app.image_picker, &app.image_resize_tx, &cache.img)
            {
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
            let p = Paragraph::new(current_lines).alignment(Alignment::Center);
            f.render_widget(p, area);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::GifEncoder;
    use image::{Delay, Frame as ImgFrame, RgbaImage};
    use std::time::Duration;

    fn create_test_gif(w: u32, h: u32, count: usize, delay_ms: u32) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut buffer);
            for i in 0..count {
                let mut img = RgbaImage::new(w, h);
                let val = ((i * 50) % 256) as u8;
                for p in img.pixels_mut() {
                    *p = image::Rgba([val, val, val, 255]);
                }
                let frame = ImgFrame::from_parts(
                    img,
                    0,
                    0,
                    Delay::from_saturating_duration(Duration::from_millis(delay_ms as u64)),
                );
                encoder.encode_frame(frame).expect("encode frame");
            }
        }
        buffer
    }

    fn make_solid_buffer(w: u32, h: u32, r: u8, g: u8, b: u8) -> ImageBuffer {
        let mut buf = ImageBuffer::new(w, h);
        for p in buf.pixels_mut() {
            *p = image::Rgb([r, g, b]);
        }
        buf
    }

    #[test]
    fn test_create_gif_helper() {
        let bytes = create_test_gif(4, 4, 2, 80);
        assert!(is_gif_data(&bytes));
    }

    #[test]
    fn test_interpolate_pixels_endpoints() {
        let from = make_solid_buffer(4, 4, 0, 0, 0);
        let to = make_solid_buffer(4, 4, 255, 255, 255);

        let at_zero = interpolate_pixels(&from, &to, 0.0);
        assert_eq!(at_zero.dimensions(), (4, 4));
        assert_eq!(*at_zero.get_pixel(0, 0), image::Rgb([0, 0, 0]));

        let at_one = interpolate_pixels(&from, &to, 1.0);
        assert_eq!(at_one.dimensions(), (4, 4));
        assert_eq!(*at_one.get_pixel(0, 0), image::Rgb([255, 255, 255]));
    }

    #[test]
    fn test_interpolate_pixels_midpoint() {
        let from = make_solid_buffer(2, 2, 10, 40, 100);
        let to = make_solid_buffer(2, 2, 110, 140, 200);

        let mid = interpolate_pixels(&from, &to, 0.5);
        assert_eq!(*mid.get_pixel(0, 0), image::Rgb([60, 90, 150]));
        assert_eq!(*mid.get_pixel(1, 1), image::Rgb([60, 90, 150]));
    }

    #[test]
    fn test_interpolate_pixels_clamps() {
        let from = make_solid_buffer(2, 2, 50, 50, 50);
        let to = make_solid_buffer(2, 2, 150, 150, 150);

        let clamped_low = interpolate_pixels(&from, &to, -1.0);
        assert_eq!(*clamped_low.get_pixel(0, 0), image::Rgb([50, 50, 50]));

        let clamped_high = interpolate_pixels(&from, &to, 2.0);
        assert_eq!(*clamped_high.get_pixel(0, 0), image::Rgb([150, 150, 150]));
    }

    #[test]
    fn test_interpolate_pixels_differing_dimensions() {
        let from = make_solid_buffer(4, 4, 10, 20, 30);
        let to = make_solid_buffer(8, 6, 110, 120, 130);

        let mid = interpolate_pixels(&from, &to, 0.5);
        assert_eq!(mid.dimensions(), (8, 6));
        assert_eq!(*mid.get_pixel(0, 0), image::Rgb([60, 70, 80]));
        assert_eq!(*mid.get_pixel(7, 5), image::Rgb([60, 70, 80]));
    }

    #[test]
    fn test_interpolate_pixels_empty_buffer() {
        let empty = ImageBuffer::new(0, 0);
        let non_empty = make_solid_buffer(4, 4, 10, 20, 30);

        for &p in &[0.0, 0.5, 1.0] {
            // from empty, to non-empty
            let res = interpolate_pixels(&empty, &non_empty, p);
            assert_eq!(res.dimensions(), (4, 4));

            // from non-empty, to empty
            let res = interpolate_pixels(&non_empty, &empty, p);
            assert_eq!(res.dimensions(), (0, 0));

            // both empty
            let res = interpolate_pixels(&empty, &empty, p);
            assert_eq!(res.dimensions(), (0, 0));
        }
    }

    #[test]
    fn test_dither_dissolve_endpoints() {
        let from = make_solid_buffer(4, 4, 0, 0, 0);
        let to = make_solid_buffer(4, 4, 255, 255, 255);

        let at_zero = dither_dissolve(&from, &to, 0.0);
        assert_eq!(*at_zero.get_pixel(0, 0), image::Rgb([0, 0, 0]));
        assert_eq!(*at_zero.get_pixel(3, 3), image::Rgb([0, 0, 0]));

        let at_one = dither_dissolve(&from, &to, 1.0);
        assert_eq!(*at_one.get_pixel(0, 0), image::Rgb([255, 255, 255]));
        assert_eq!(*at_one.get_pixel(3, 3), image::Rgb([255, 255, 255]));
    }

    #[test]
    fn test_dither_dissolve_progression() {
        let from = make_solid_buffer(4, 4, 0, 0, 0);
        let to = make_solid_buffer(4, 4, 255, 255, 255);

        let count_to_pixels = |buf: &ImageBuffer| -> usize {
            buf.pixels()
                .filter(|p| **p == image::Rgb([255, 255, 255]))
                .count()
        };

        let c0 = count_to_pixels(&dither_dissolve(&from, &to, 0.0));
        let c25 = count_to_pixels(&dither_dissolve(&from, &to, 0.25));
        let c50 = count_to_pixels(&dither_dissolve(&from, &to, 0.5));
        let c75 = count_to_pixels(&dither_dissolve(&from, &to, 0.75));
        let c100 = count_to_pixels(&dither_dissolve(&from, &to, 1.0));

        assert_eq!(c0, 0);
        assert!(c25 > c0);
        assert!(c50 >= c25);
        assert!(c75 >= c50);
        assert_eq!(c100, 16);
    }

    #[test]
    fn test_dither_dissolve_empty_buffer() {
        let empty = ImageBuffer::new(0, 0);
        let non_empty = make_solid_buffer(4, 4, 10, 20, 30);

        for &p in &[0.0, 0.5, 1.0] {
            // from empty, to non-empty
            let res = dither_dissolve(&empty, &non_empty, p);
            assert_eq!(res.dimensions(), (4, 4));

            // from non-empty, to empty
            let res = dither_dissolve(&non_empty, &empty, p);
            assert_eq!(res.dimensions(), (0, 0));

            // both empty
            let res = dither_dissolve(&empty, &empty, p);
            assert_eq!(res.dimensions(), (0, 0));
        }
    }

    #[test]
    fn test_transition_pixels_modes() {
        let from = make_solid_buffer(4, 4, 0, 0, 0);
        let to = make_solid_buffer(4, 4, 200, 200, 200);

        let crossfade = transition_pixels(&from, &to, 0.5, TransitionMode::Crossfade);
        assert_eq!(*crossfade.get_pixel(0, 0), image::Rgb([100, 100, 100]));

        let dither = transition_pixels(&from, &to, 0.5, TransitionMode::Dither);
        // Under dither, every pixel is either 0 or 200
        let p = dither.get_pixel(0, 0);
        assert!(p[0] == 0 || p[0] == 200);
    }

    #[test]
    fn test_image_to_half_blocks() {
        let mut buf = ImageBuffer::new(2, 2);
        buf.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        buf.put_pixel(1, 0, image::Rgb([0, 255, 0]));
        buf.put_pixel(0, 1, image::Rgb([0, 0, 255]));
        buf.put_pixel(1, 1, image::Rgb([255, 255, 0]));

        let lines = image_to_half_blocks(&buf);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 2);

        // Check first span
        let s0 = &lines[0].spans[0];
        assert_eq!(s0.content, "▀");
        assert_eq!(s0.style.fg, Some(Color::Rgb(255, 0, 0)));
        assert_eq!(s0.style.bg, Some(Color::Rgb(0, 0, 255)));

        // Check second span
        let s1 = &lines[0].spans[1];
        assert_eq!(s1.content, "▀");
        assert_eq!(s1.style.fg, Some(Color::Rgb(0, 255, 0)));
        assert_eq!(s1.style.bg, Some(Color::Rgb(255, 255, 0)));
    }

    #[test]
    fn test_image_to_half_blocks_odd_height() {
        let mut buf = ImageBuffer::new(2, 3);
        buf.put_pixel(0, 0, image::Rgb([10, 10, 10]));
        buf.put_pixel(0, 1, image::Rgb([20, 20, 20]));
        buf.put_pixel(0, 2, image::Rgb([30, 30, 30]));

        let lines = image_to_half_blocks(&buf);
        assert_eq!(lines.len(), 2);
        // Line 1: top=10, bottom=20
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Rgb(10, 10, 10)));
        assert_eq!(lines[0].spans[0].style.bg, Some(Color::Rgb(20, 20, 20)));
        // Line 2: top=30, bottom=None
        assert_eq!(lines[1].spans[0].style.fg, Some(Color::Rgb(30, 30, 30)));
        assert_eq!(lines[1].spans[0].style.bg, None);
    }

    #[test]
    fn test_empty_image_to_half_blocks() {
        let buf = ImageBuffer::new(0, 0);
        let lines = image_to_half_blocks(&buf);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_extract_palette() {
        let buf = make_solid_buffer(10, 10, 42, 84, 126);
        let palette = extract_palette(&buf);
        assert_eq!(palette.len(), 6);
        for c in palette {
            assert_eq!(c, Color::Rgb(42, 84, 126));
        }
    }

    #[test]
    fn test_step_frame() {
        assert_eq!(step_frame(0, 4), 1);
        assert_eq!(step_frame(1, 4), 2);
        assert_eq!(step_frame(2, 4), 3);
        assert_eq!(step_frame(3, 4), 0); // wrap
        assert_eq!(step_frame(0, 1), 0); // single frame stays
        assert_eq!(step_frame(0, 0), 0); // empty frame list safety
    }

    #[test]
    fn test_should_step_frame() {
        assert!(!should_step_frame(49, 50));
        assert!(should_step_frame(50, 50));
        assert!(should_step_frame(100, 50));
    }

    #[test]
    fn test_should_switch_slide() {
        assert!(!should_switch_slide(100, 0, 150));
        assert!(!should_switch_slide(150, 0, 150));
        assert!(should_switch_slide(151, 0, 150));
    }

    #[test]
    fn test_calculate_transition_progress() {
        assert_eq!(calculate_transition_progress(0, 500), 0.0);
        assert_eq!(calculate_transition_progress(250, 500), 0.5);
        assert_eq!(calculate_transition_progress(500, 500), 1.0);
        assert_eq!(calculate_transition_progress(750, 500), 1.0); // clamped
        assert_eq!(calculate_transition_progress(100, 0), 1.0); // zero duration edge case
    }

    #[test]
    fn test_is_gif_data() {
        assert!(is_gif_data(b"GIF89a\x01\x00..."));
        assert!(is_gif_data(b"GIF87a\x01\x00..."));
        assert!(!is_gif_data(b"\x89PNG\r\n\x1a\n..."));
        assert!(!is_gif_data(b"\xff\xd8\xff\xe0..."));
    }

    #[test]
    fn test_decode_media_bytes_animated_gif() {
        let gif_bytes = create_test_gif(16, 16, 3, 80);
        let decoded = decode_media_bytes(&gif_bytes, "test.gif", 16, 16).expect("decode gif");

        assert!(decoded.is_animated);
        assert_eq!(decoded.frames.len(), 3);
        assert_eq!(decoded.orig_width, 16);
        assert_eq!(decoded.orig_height, 16);
        assert_eq!(decoded.frames[0].delay_ms, 80);
        assert_eq!(decoded.frames[1].delay_ms, 80);
        assert_eq!(decoded.palette.len(), 6);
    }

    #[test]
    fn test_decode_media_bytes_single_frame() {
        let gif_bytes = create_test_gif(8, 8, 1, 100);
        let decoded = decode_media_bytes(&gif_bytes, "static.gif", 8, 8).expect("decode static");

        assert!(!decoded.is_animated);
        assert_eq!(decoded.frames.len(), 1);
        assert_eq!(decoded.orig_width, 8);
        assert_eq!(decoded.orig_height, 8);
    }

    #[test]
    fn test_fallback_media_decode() {
        let default_bytes = include_bytes!("../assets/default_media.jpg");
        let decoded = decode_media_bytes(default_bytes, "default_fallback", 32, 16)
            .expect("decode fallback jpg");

        assert!(!decoded.is_animated);
        assert_eq!(decoded.frames.len(), 1);
        assert!(decoded.orig_width > 0);
        assert!(decoded.orig_height > 0);
        assert_eq!(decoded.palette.len(), 6);
    }

    #[test]
    fn test_cached_media_reset() {
        reset_cached_media();
        assert!(current_media_info().is_none());
    }
}
