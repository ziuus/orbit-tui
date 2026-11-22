use image::codecs::gif::GifEncoder;
use image::{Delay, Frame as ImgFrame, RgbaImage};
use orbit_tui::widgets::pinned_media::{
    calculate_transition_progress, decode_media_bytes, dither_dissolve, image_to_half_blocks,
    interpolate_pixels, should_step_frame, should_switch_slide, step_frame, transition_pixels,
    ImageBuffer, TransitionMode,
};
use std::time::Duration;

fn make_color_buf(w: u32, h: u32, r: u8, g: u8, b: u8) -> ImageBuffer {
    let mut buf = ImageBuffer::new(w, h);
    for p in buf.pixels_mut() {
        *p = image::Rgb([r, g, b]);
    }
    buf
}

fn create_multi_frame_gif(w: u32, h: u32, count: usize, delay_ms: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        for i in 0..count {
            let mut img = RgbaImage::new(w, h);
            let color = ((i * 60) % 256) as u8;
            for p in img.pixels_mut() {
                *p = image::Rgba([color, 255 - color, color / 2, 255]);
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
    bytes
}

#[test]
fn integration_pixel_transition_lifecycle() {
    let from = make_color_buf(16, 12, 10, 20, 30);
    let to = make_color_buf(16, 12, 210, 220, 230);

    // Step through a 10-step transition lifecycle
    let steps = 10;
    for i in 0..=steps {
        let progress = i as f32 / steps as f32;
        let blended = interpolate_pixels(&from, &to, progress);
        assert_eq!(blended.dimensions(), (16, 12));

        let p = blended.get_pixel(8, 6);
        let expected_r = (10.0 * (1.0 - progress) + 210.0 * progress).round() as u8;
        let expected_g = (20.0 * (1.0 - progress) + 220.0 * progress).round() as u8;
        let expected_b = (30.0 * (1.0 - progress) + 230.0 * progress).round() as u8;

        assert_eq!(p[0], expected_r);
        assert_eq!(p[1], expected_g);
        assert_eq!(p[2], expected_b);

        // Verify half-block conversion works on every blended frame
        let lines = image_to_half_blocks(&blended);
        assert_eq!(lines.len(), 6); // 12 height / 2 = 6 lines
        assert_eq!(lines[0].spans.len(), 16); // 16 width = 16 spans
    }
}

#[test]
fn integration_dither_dissolve_lifecycle() {
    let from = make_color_buf(8, 8, 0, 0, 0);
    let to = make_color_buf(8, 8, 255, 255, 255);

    let mut last_to_count = 0;
    for step in 0..=8 {
        let progress = step as f32 / 8.0;
        let dissolved = dither_dissolve(&from, &to, progress);
        let to_count = dissolved
            .pixels()
            .filter(|p| **p == image::Rgb([255, 255, 255]))
            .count();

        assert!(
            to_count >= last_to_count,
            "dither dissolve must be monotonic in pixel replacement"
        );
        last_to_count = to_count;
    }
    assert_eq!(last_to_count, 64);
}

#[test]
fn integration_animated_gif_playback_stepping() {
    let gif_bytes = create_multi_frame_gif(12, 10, 4, 100);
    let decoded =
        decode_media_bytes(&gif_bytes, "anim.gif", 12, 10).expect("failed to decode anim gif");

    assert!(decoded.is_animated);
    assert_eq!(decoded.frames.len(), 4);

    let mut current_frame = 0;
    let mut total_stepped = 0;

    // Simulate 12 frame steps (3 full loops through 4 frames)
    for tick in 0..12 {
        let delay = decoded.frames[current_frame].delay_ms;
        assert_eq!(delay, 100);

        if should_step_frame(100, delay) {
            current_frame = step_frame(current_frame, decoded.frames.len());
            total_stepped += 1;
        }

        let expected_frame = (tick + 1) % 4;
        assert_eq!(current_frame, expected_frame);

        // Confirm lines rendered from this frame
        let lines = &decoded.frames[current_frame].lines;
        assert_eq!(lines.len(), 5); // 10 height / 2 = 5 lines
    }
    assert_eq!(total_stepped, 12);
}

#[test]
fn integration_slideshow_timer_ticks_and_transition() {
    let mut current_tick = 0u64;
    let mut last_switch_tick = 0u64;
    let interval = 150u64;

    // Tick forward 150 ticks: no switch yet
    while current_tick <= 150 {
        assert!(!should_switch_slide(
            current_tick,
            last_switch_tick,
            interval
        ));
        current_tick += 1;
    }

    // Tick 151 triggers switch
    assert!(should_switch_slide(
        current_tick,
        last_switch_tick,
        interval
    ));
    last_switch_tick = current_tick;
    assert!(!should_switch_slide(
        current_tick,
        last_switch_tick,
        interval
    ));

    // Simulate transition progress over 500ms duration
    let duration_ms = 500u64;
    let from_slide = make_color_buf(10, 8, 100, 0, 0);
    let to_slide = make_color_buf(10, 8, 0, 100, 0);

    for elapsed_ms in [0, 125, 250, 375, 500, 600] {
        let progress = calculate_transition_progress(elapsed_ms, duration_ms);
        if elapsed_ms < 500 {
            assert!(progress < 1.0);
        } else {
            assert_eq!(progress, 1.0);
        }
        let frame = transition_pixels(&from_slide, &to_slide, progress, TransitionMode::Crossfade);
        assert_eq!(frame.dimensions(), (10, 8));
    }
}

#[test]
fn integration_fallback_single_image_stability() {
    let fallback_bytes = include_bytes!("../src/assets/default_media.jpg");
    let decoded = decode_media_bytes(fallback_bytes, "default_fallback", 20, 10)
        .expect("decode fallback asset");

    assert!(!decoded.is_animated);
    assert_eq!(decoded.frames.len(), 1);
    assert_eq!(decoded.frames[0].delay_ms, 0);
    assert!(decoded.orig_width > 0);
    assert!(decoded.orig_height > 0);
    assert_eq!(decoded.palette.len(), 6);

    let lines = &decoded.frames[0].lines;
    assert!(!lines.is_empty());
    for line in lines {
        assert!(!line.spans.is_empty());
        for span in &line.spans {
            assert_eq!(span.content, "▀");
        }
    }
}
