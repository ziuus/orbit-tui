//! Custom community dashboard and visual scene layers.
//!
//! Provides schema definitions, discovery loader, validation, and serialization
//! for community-authored layouts and visual layers.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Visual pattern kind for visual scene and background layers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum VisualKind {
    #[default]
    Ascii,
    Grid,
    Stars,
    Waves,
}

/// Screen positioning for visual scene and background layers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LayerPosition {
    #[default]
    Background,
    Foreground,
    Banner,
}

/// Configuration for an optional visual layer attached to a community layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VisualLayerConfig {
    pub kind: VisualKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default)]
    pub fps: u32,
    #[serde(default)]
    pub position: LayerPosition,
    #[serde(default = "default_true")]
    pub transparent: bool,
}

impl Default for VisualLayerConfig {
    fn default() -> Self {
        Self {
            kind: VisualKind::Ascii,
            content: None,
            file: None,
            fps: 0,
            position: LayerPosition::Background,
            transparent: true,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum VisualLayerDef {
    Simple(String),
    Detailed(VisualLayerDetailed),
}

#[derive(Deserialize)]
#[serde(default)]
struct VisualLayerDetailed {
    kind: VisualKind,
    content: Option<String>,
    file: Option<String>,
    fps: u32,
    position: LayerPosition,
    transparent: bool,
}

impl Default for VisualLayerDetailed {
    fn default() -> Self {
        Self {
            kind: VisualKind::Ascii,
            content: None,
            file: None,
            fps: 0,
            position: LayerPosition::Background,
            transparent: true,
        }
    }
}

impl<'de> Deserialize<'de> for VisualLayerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = VisualLayerDef::deserialize(deserializer)?;
        match def {
            VisualLayerDef::Simple(s) => {
                let kind = match s.trim().to_lowercase().as_str() {
                    "grid" => VisualKind::Grid,
                    "stars" | "starfield" => VisualKind::Stars,
                    "waves" => VisualKind::Waves,
                    _ => VisualKind::Ascii,
                };
                Ok(Self {
                    kind,
                    content: None,
                    file: None,
                    fps: 0,
                    position: LayerPosition::Background,
                    transparent: true,
                })
            }
            VisualLayerDef::Detailed(d) => Ok(Self {
                kind: d.kind,
                content: d.content,
                file: d.file,
                fps: d.fps,
                position: d.position,
                transparent: d.transparent,
            }),
        }
    }
}

fn default_true() -> bool {
    true
}

/// Definition of a custom community layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommunityLayer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, alias = "title", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub layout: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratios: Option<Vec<u16>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual: Option<VisualLayerConfig>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(skip)]
    pub is_external: bool,
}

pub type CommunityLayerConfig = CommunityLayer;

#[derive(Deserialize)]
#[serde(untagged)]
enum CommunityLayerDef {
    Matrix(Vec<Vec<String>>),
    Detailed(CommunityLayerDetailed),
}

#[derive(Deserialize)]
struct CommunityLayerDetailed {
    #[serde(default)]
    id: Option<String>,
    #[serde(default, alias = "title")]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    layout: Vec<Vec<String>>,
    #[serde(default)]
    ratios: Option<Vec<u16>>,
    #[serde(default)]
    scene: Option<String>,
    #[serde(default)]
    visual: Option<VisualLayerConfig>,
    #[serde(default = "default_true")]
    enabled: bool,
}

impl<'de> Deserialize<'de> for CommunityLayer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = CommunityLayerDef::deserialize(deserializer)?;
        match def {
            CommunityLayerDef::Matrix(layout) => Ok(Self {
                id: None,
                name: None,
                description: None,
                layout,
                ratios: None,
                scene: None,
                visual: None,
                enabled: true,
                is_external: false,
            }),
            CommunityLayerDef::Detailed(d) => Ok(Self {
                id: d.id,
                name: d.name,
                description: d.description,
                layout: d.layout,
                ratios: d.ratios,
                scene: d.scene,
                visual: d.visual,
                enabled: d.enabled,
                is_external: false,
            }),
        }
    }
}

impl CommunityLayer {
    /// Validates layout matrix sanity, column limits, and ratios.
    pub fn validate(&self) -> Result<(), String> {
        if self.layout.is_empty() || self.layout.iter().all(|col| col.is_empty()) {
            return Err("community layer layout matrix cannot be empty".to_string());
        }
        if self.layout.len() > 4 {
            return Err(format!(
                "community layer layout exceeds maximum 4 columns (got {})",
                self.layout.len()
            ));
        }
        if let Some(ref ratios) = self.ratios {
            if ratios.len() != self.layout.len() {
                return Err(format!(
                    "community layer ratios count ({}) must match column count ({})",
                    ratios.len(),
                    self.layout.len()
                ));
            }
            if ratios.iter().any(|&r| r == 0) {
                return Err("community layer column ratio percentage cannot be zero".to_string());
            }
        }
        Ok(())
    }

    /// Returns displayable title: name, id, or fallback.
    pub fn display_name(&self, fallback_id: &str) -> String {
        if let Some(ref name) = self.name {
            if !name.trim().is_empty() {
                return name.trim().to_string();
            }
        }
        if let Some(ref id) = self.id {
            if !id.trim().is_empty() {
                return id.trim().to_string();
            }
        }
        fallback_id.to_string()
    }
}

/// Deserializer for `DashboardConfig.layers` supporting tables, compact matrices, and array lists.
pub fn deserialize_layers<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, CommunityLayer>, D::Error>
where
    D: Deserializer<'de>,
{
    struct LayersVisitor;

    impl<'de> Visitor<'de> for LayersVisitor {
        type Value = BTreeMap<String, CommunityLayer>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map of community layers or an array of community layers")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut map = BTreeMap::new();
            while let Some((key, mut layer)) = access.next_entry::<String, CommunityLayer>()? {
                if layer.id.is_none() {
                    layer.id = Some(key.clone());
                }
                map.insert(key, layer);
            }
            Ok(map)
        }

        fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
        where
            S: SeqAccess<'de>,
        {
            let mut map = BTreeMap::new();
            let mut idx = 0;
            while let Some(mut layer) = access.next_element::<CommunityLayer>()? {
                let id = layer
                    .id
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| layer.name.clone().filter(|s| !s.trim().is_empty()))
                    .unwrap_or_else(|| format!("layer_{}", idx));
                if layer.id.is_none() {
                    layer.id = Some(id.clone());
                }
                map.insert(id, layer);
                idx += 1;
            }
            Ok(map)
        }
    }

    deserializer.deserialize_any(LayersVisitor)
}

/// Serializes only inline user-authored layers to prevent duplicating external discovered layers into `config.toml`.
pub fn serialize_inline_layers<S>(
    layers: &BTreeMap<String, CommunityLayer>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let inline: BTreeMap<&String, &CommunityLayer> = layers
        .iter()
        .filter(|(_, l)| !l.is_external)
        .collect();
    inline.serialize(serializer)
}

/// Parse multi-frame ASCII visual layer contents separated by `\n---\n` or `\x0c`.
pub fn parse_visual_frames(content: &str) -> Vec<&str> {
    if content.contains("\n---\n") {
        content
            .split("\n---\n")
            .map(|s| s.trim_matches('\n'))
            .filter(|s| !s.is_empty())
            .collect()
    } else if content.contains('\x0c') {
        content
            .split('\x0c')
            .map(|s| s.trim_matches('\n'))
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![content]
    }
}

/// Return standard community layer config search directory: `$XDG_CONFIG_HOME/orbit/layers` or `~/.config/orbit/layers`.
pub fn get_community_layers_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let mut p = PathBuf::from(xdg);
        p.push("orbit");
        p.push("layers");
        p
    } else if let Ok(home) = std::env::var("HOME") {
        let mut p = PathBuf::from(home);
        p.push(".config");
        p.push("orbit");
        p.push("layers");
        p
    } else {
        PathBuf::from(".config/orbit/layers")
    }
}

/// Return all directories checked for community layers.
pub fn get_layer_search_dirs() -> Vec<PathBuf> {
    vec![get_community_layers_dir(), PathBuf::from("layers")]
}

/// Discover community layer files across specified directories, gracefully skipping malformed files.
pub fn discover_layers_in_dirs(dirs: &[PathBuf]) -> BTreeMap<String, CommunityLayer> {
    let mut map = BTreeMap::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let mut layer = match toml::from_str::<CommunityLayer>(&content) {
                Ok(l) => l,
                Err(e) => {
                    log::warn!(
                        "Skipping invalid community layer '{}' ({}): {}",
                        stem,
                        path.display(),
                        e
                    );
                    continue;
                }
            };
            if let Err(e) = layer.validate() {
                log::warn!(
                    "Skipping community layer '{}' failing validation ({}): {}",
                    stem,
                    path.display(),
                    e
                );
                continue;
            }
            let key = layer
                .id
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(stem)
                .to_string();
            if layer.id.is_none() {
                layer.id = Some(key.clone());
            }
            layer.is_external = true;
            if !map.contains_key(&key) {
                map.insert(key, layer);
            }
        }
    }
    map
}

/// Discover community layers from default directories.
pub fn discover_layers() -> BTreeMap<String, CommunityLayer> {
    discover_layers_in_dirs(&get_layer_search_dirs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_community_layer_matrix_deserialization() {
        let toml_str = r#"
            zen = [["gauges", "cpu"], ["memory", "storage"]]
        "#;
        let map: std::collections::BTreeMap<String, CommunityLayer> =
            toml::from_str(toml_str).expect("parse map with matrix");
        let layer = &map["zen"];
        assert_eq!(layer.layout.len(), 2);
        assert_eq!(layer.layout[0], vec!["gauges", "cpu"]);
        assert_eq!(layer.layout[1], vec!["memory", "storage"]);
        assert!(layer.validate().is_ok());
    }

    #[test]
    fn test_community_layer_detailed_deserialization() {
        let toml_str = r#"
            id = "devops"
            name = "DevOps Cockpit"
            description = "High density telemetry"
            layout = [
                ["system", "cpu", "memory"],
                ["network", "processes"]
            ]
            ratios = [60, 40]
            scene = "cyberpunk"
            enabled = true
        "#;
        let layer: CommunityLayer = toml::from_str(toml_str).expect("parse detailed");
        assert_eq!(layer.id.as_deref(), Some("devops"));
        assert_eq!(layer.name.as_deref(), Some("DevOps Cockpit"));
        assert_eq!(layer.layout.len(), 2);
        assert_eq!(layer.ratios, Some(vec![60, 40]));
        assert_eq!(layer.scene.as_deref(), Some("cyberpunk"));
        assert!(layer.validate().is_ok());
    }

    #[test]
    fn test_community_layer_visual_deserialization() {
        let toml_str = r#"
            name = "Visual Layer"
            layout = [["clock"], ["weather"]]
            visual = { kind = "stars", fps = 15, position = "background", transparent = true }
        "#;
        let layer: CommunityLayer = toml::from_str(toml_str).expect("parse with visual");
        let v = layer.visual.expect("visual present");
        assert_eq!(v.kind, VisualKind::Stars);
        assert_eq!(v.fps, 15);
        assert_eq!(v.position, LayerPosition::Background);
        assert!(v.transparent);
    }

    #[test]
    fn test_community_layer_simple_visual_deserialization() {
        let toml_str = r#"
            name = "Grid Layer"
            layout = [["clock"], ["weather"]]
            visual = "grid"
        "#;
        let layer: CommunityLayer = toml::from_str(toml_str).expect("parse simple visual");
        let v = layer.visual.expect("visual present");
        assert_eq!(v.kind, VisualKind::Grid);
        assert_eq!(v.position, LayerPosition::Background);
    }

    #[test]
    fn test_community_layer_validation_empty_matrix() {
        let empty_layer = CommunityLayer {
            id: None,
            name: None,
            description: None,
            layout: vec![],
            ratios: None,
            scene: None,
            visual: None,
            enabled: true,
            is_external: false,
        };
        assert!(empty_layer.validate().is_err());

        let all_empty_cols = CommunityLayer {
            id: None,
            name: None,
            description: None,
            layout: vec![vec![], vec![]],
            ratios: None,
            scene: None,
            visual: None,
            enabled: true,
            is_external: false,
        };
        assert!(all_empty_cols.validate().is_err());
    }

    #[test]
    fn test_community_layer_validation_excess_columns() {
        let layer = CommunityLayer {
            id: None,
            name: None,
            description: None,
            layout: vec![
                vec!["a".into()],
                vec!["b".into()],
                vec!["c".into()],
                vec!["d".into()],
                vec!["e".into()],
            ],
            ratios: None,
            scene: None,
            visual: None,
            enabled: true,
            is_external: false,
        };
        assert!(layer.validate().is_err());
    }

    #[test]
    fn test_community_layer_validation_ratios_mismatch() {
        let layer = CommunityLayer {
            id: None,
            name: None,
            description: None,
            layout: vec![vec!["a".into()], vec!["b".into()]],
            ratios: Some(vec![50, 25, 25]),
            scene: None,
            visual: None,
            enabled: true,
            is_external: false,
        };
        assert!(layer.validate().is_err());
    }

    #[test]
    fn test_community_layer_validation_zero_ratio() {
        let layer = CommunityLayer {
            id: None,
            name: None,
            description: None,
            layout: vec![vec!["a".into()], vec!["b".into()]],
            ratios: Some(vec![50, 0]),
            scene: None,
            visual: None,
            enabled: true,
            is_external: false,
        };
        assert!(layer.validate().is_err());
    }

    #[test]
    fn test_parse_visual_frames() {
        let single = "frame 1";
        assert_eq!(parse_visual_frames(single), vec!["frame 1"]);

        let multi = "frame 1\n---\nframe 2\n---\nframe 3";
        assert_eq!(
            parse_visual_frames(multi),
            vec!["frame 1", "frame 2", "frame 3"]
        );

        let formfeed = "frame A\x0cframe B";
        assert_eq!(parse_visual_frames(formfeed), vec!["frame A", "frame B"]);
    }

    #[test]
    fn test_discover_layers_in_dirs_skips_corrupt_files() {
        let temp_dir = std::env::temp_dir().join(format!("orbit_test_layers_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let valid_path = temp_dir.join("valid.toml");
        fs::write(
            &valid_path,
            r#"
            name = "Valid Layer"
            layout = [["cpu"], ["memory"]]
        "#,
        )
        .expect("write valid");

        let corrupt_path = temp_dir.join("corrupt.toml");
        fs::write(&corrupt_path, "not a toml file ::: [[[ ").expect("write corrupt");

        let invalid_layout = temp_dir.join("empty.toml");
        fs::write(
            &invalid_layout,
            r#"
            name = "Empty"
            layout = []
        "#,
        )
        .expect("write empty");

        let not_toml = temp_dir.join("notes.txt");
        fs::write(&not_toml, "some notes").expect("write txt");

        let discovered = discover_layers_in_dirs(&[temp_dir.clone()]);
        assert_eq!(discovered.len(), 1);
        assert!(discovered.contains_key("valid"));
        assert!(discovered["valid"].is_external);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
