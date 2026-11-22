use std::collections::BTreeMap;
use std::fs;

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use orbit_tui::app::App;
use orbit_tui::config::Config;
use orbit_tui::custom::layer::{
    discover_layers_in_dirs, parse_visual_frames, CommunityLayer, LayerPosition, VisualKind,
    VisualLayerConfig,
};
use orbit_tui::theme::Theme;
use orbit_tui::widgets::scenes::{
    render_ascii_layer, render_scene_by_name, render_visual_layer,
};

#[test]
fn integration_community_layer_dashboard_application() {
    let toml_str = r#"
        [dashboard]
        preset = "devops"

        [dashboard.layers.devops]
        name = "DevOps Cockpit"
        description = "High-density ops telemetry"
        layout = [
            ["system", "cpu", "memory"],
            ["network", "storage"],
            ["processes"]
        ]
        ratios = [35, 40, 25]
        scene = "cyberpunk"
        enabled = true

        [dashboard.layers.zen]
        name = "Zen Minimalist"
        layout = [
            ["clock"],
            ["weather"]
        ]
        ratios = [50, 50]
        enabled = true

        [dashboard.layers.disabled_layer]
        name = "Inactive"
        layout = [["clock"]]
        enabled = false
    "#;

    let mut cfg: Config = toml::from_str(toml_str).expect("deserialize config with layers");
    assert_eq!(cfg.dashboard.layers.len(), 3);

    // Verify available presets: builtins first, followed by enabled community layers in order
    let presets = cfg.dashboard.available_presets();
    assert_eq!(
        presets,
        vec![
            "overview",
            "cockpit",
            "monitoring",
            "minimal",
            "devops",
            "zen"
        ]
    );
    assert!(!presets.contains(&"disabled_layer".to_string()));

    // Apply devops preset
    cfg.dashboard.apply_preset("devops");
    assert_eq!(cfg.dashboard.preset, "devops");
    assert_eq!(cfg.dashboard.layout.len(), 3);
    assert_eq!(cfg.dashboard.layout[0], vec!["system", "cpu", "memory"]);
    assert_eq!(cfg.dashboard.layout[1], vec!["network", "storage"]);
    assert_eq!(cfg.dashboard.layout[2], vec!["processes"]);

    // Apply zen preset
    cfg.dashboard.apply_preset("zen");
    assert_eq!(cfg.dashboard.preset, "zen");
    assert_eq!(cfg.dashboard.layout.len(), 2);
    assert_eq!(cfg.dashboard.layout[0], vec!["clock"]);
    assert_eq!(cfg.dashboard.layout[1], vec!["weather"]);

    // Fallback on unknown preset: layout is preserved
    cfg.dashboard.apply_preset("unknown_nonexistent");
    assert_eq!(cfg.dashboard.preset, "unknown_nonexistent");
    assert_eq!(cfg.dashboard.layout.len(), 2);
}

#[test]
fn integration_community_layer_cycling_and_toast() {
    let mut app = App::new(Config::default());
    let mut custom_layers = BTreeMap::new();
    custom_layers.insert(
        "cloud".to_string(),
        CommunityLayer {
            id: Some("cloud".into()),
            name: Some("Cloud Operations".into()),
            description: Some("Cloud monitoring".into()),
            layout: vec![vec!["system".into()], vec!["network".into()]],
            ratios: Some(vec![50, 50]),
            scene: Some("synthwave".into()),
            visual: None,
            enabled: true,
            is_external: false,
        },
    );
    app.config.dashboard.layers = custom_layers;

    let initial_preset = app.config.dashboard.preset.clone();
    assert_eq!(initial_preset, "overview");

    // Cycle forward through all presets: overview -> cockpit -> monitoring -> minimal -> cloud -> overview
    let expected_sequence = ["cockpit", "monitoring", "minimal", "cloud", "overview"];
    for expected in expected_sequence {
        app.cycle_layout(true);
        assert_eq!(app.config.dashboard.preset, expected);
        if expected == "cloud" {
            assert!(app.toast.as_ref().map(|(s, _)| s.contains("Cloud Operations")).unwrap_or(false));
        } else {
            assert!(app.toast.as_ref().map(|(s, _)| s.contains(expected)).unwrap_or(false));
        }
    }

    // Cycle backward: overview -> cloud -> minimal -> ...
    app.cycle_layout(false);
    assert_eq!(app.config.dashboard.preset, "cloud");
    app.cycle_layout(false);
    assert_eq!(app.config.dashboard.preset, "minimal");
}

#[test]
fn integration_community_layer_render_no_panic() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("init test terminal");
    let mut app = App::new(Config::default());

    let mut custom_layers = BTreeMap::new();
    custom_layers.insert(
        "matrix_ops".to_string(),
        CommunityLayer {
            id: Some("matrix_ops".into()),
            name: Some("Matrix Ops".into()),
            description: None,
            layout: vec![
                vec!["gauges".into(), "cpu".into()],
                vec!["memory".into(), "storage".into()],
                vec!["clock".into(), "weather".into()],
            ],
            ratios: Some(vec![30, 40, 30]),
            scene: Some("synthwave".into()),
            visual: Some(VisualLayerConfig {
                kind: VisualKind::Ascii,
                content: Some("ORBIT TELEMETRY LAYER\n=====================".into()),
                file: None,
                fps: 0,
                position: LayerPosition::Banner,
                transparent: true,
            }),
            enabled: true,
            is_external: false,
        },
    );
    app.config.dashboard.layers = custom_layers;
    app.config.dashboard.apply_preset("matrix_ops");

    // Test rendering on various terminal dimensions without panic
    for (width, height) in [(120, 40), (80, 24), (85, 27), (140, 50)] {
        terminal.resize(Rect::new(0, 0, width, height)).expect("resize");
        terminal
            .draw(|f| {
                let area = f.area();
                orbit_tui::screens::dashboard::render(f, area, &mut app);
            })
            .expect("draw dashboard");
    }
}

#[test]
fn integration_corrupt_layer_resilience() {
    let temp_dir = std::env::temp_dir().join(format!("orbit_corrupt_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).expect("create temp dir");

    // 1. Valid layer
    let valid_path = temp_dir.join("valid_layer.toml");
    fs::write(
        &valid_path,
        r#"
        id = "valid_layer"
        name = "Valid Community Layer"
        layout = [["gauges"], ["cpu"]]
        ratios = [50, 50]
        "#,
    )
    .expect("write valid");

    // 2. Syntax corruption
    let corrupt_syntax = temp_dir.join("corrupt_syntax.toml");
    fs::write(&corrupt_syntax, "{{{ unclosed syntax error [[[ ===").expect("write corrupt");

    // 3. Semantic corruption: empty layout
    let empty_layout = temp_dir.join("empty_layout.toml");
    fs::write(&empty_layout, "name = 'empty'\nlayout = []").expect("write empty layout");

    // 4. Semantic corruption: excess columns (>4)
    let excess_cols = temp_dir.join("excess_cols.toml");
    fs::write(
        &excess_cols,
        "name = 'excess'\nlayout = [['a'], ['b'], ['c'], ['d'], ['e']]",
    )
    .expect("write excess cols");

    // 5. Semantic corruption: ratios mismatch
    let ratios_mismatch = temp_dir.join("mismatched_ratios.toml");
    fs::write(
        &ratios_mismatch,
        "name = 'mismatch'\nlayout = [['a'], ['b']]\nratios = [70, 20, 10]",
    )
    .expect("write mismatched ratios");

    // 6. Zero ratio
    let zero_ratio = temp_dir.join("zero_ratio.toml");
    fs::write(
        &zero_ratio,
        "name = 'zero'\nlayout = [['a'], ['b']]\nratios = [100, 0]",
    )
    .expect("write zero ratio");

    // 7. Non-toml file
    let non_toml = temp_dir.join("readme.md");
    fs::write(&non_toml, "# Community Layers Readme").expect("write markdown");

    let discovered = discover_layers_in_dirs(&[temp_dir.clone()]);

    // Only the single valid layer must be discovered
    assert_eq!(discovered.len(), 1);
    assert!(discovered.contains_key("valid_layer"));
    let layer = &discovered["valid_layer"];
    assert_eq!(layer.name.as_deref(), Some("Valid Community Layer"));
    assert!(layer.is_external);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn integration_visual_ascii_and_procedural_scene_rendering() {
    let theme = Theme::dark();
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);

    // Multi-frame animation splitting
    let frames_content = "FRAME 1\n---\nFRAME 2\n---\nFRAME 3";
    let frames = parse_visual_frames(frames_content);
    assert_eq!(frames, vec!["FRAME 1", "FRAME 2", "FRAME 3"]);

    // Transparency verification
    buf[(0, 0)].set_char('Z');
    buf[(1, 0)].set_char('Z');
    buf[(2, 0)].set_char('Z');

    // Space transparent: 'X' at 0, space at 1, 'Y' at 2
    render_ascii_layer(&mut buf, area, "X Y", &theme, true);
    assert_eq!(buf[(0, 0)].symbol(), "X");
    assert_eq!(buf[(1, 0)].symbol(), "Z"); // Kept original 'Z'
    assert_eq!(buf[(2, 0)].symbol(), "Y");

    // Procedural scenes and patterns render into buffer cleanly
    for scene_name in ["synthwave", "aurora", "lava", "cyberpunk", "blackhole", "grid", "stars", "waves"] {
        render_scene_by_name(scene_name, &mut buf, area, &theme, true, false);
    }

    // VisualLayerConfig rendering
    let vis_cfg = VisualLayerConfig {
        kind: VisualKind::Stars,
        content: None,
        file: None,
        fps: 15,
        position: LayerPosition::Background,
        transparent: true,
    };
    render_visual_layer(&vis_cfg, &mut buf, area, &theme, true, false);
}
