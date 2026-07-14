use std::collections::HashMap;
use std::path::Path;

use anyhow::{Result, bail};
use ratatui::style::Color;
use tracing::warn;

use super::model::{Mascot, RawTheme, Role, Theme, ThemeMeta};
use super::resolve::{parse_raw, resolve_theme};

const BUILTIN: &[(&str, &str)] = &[
    (
        "gruvbox-material",
        include_str!("../../../themes/gruvbox-material.toml"),
    ),
    ("pico8", include_str!("../../../themes/pico8.toml")),
    ("amber", include_str!("../../../themes/amber.toml")),
    ("gameboy", include_str!("../../../themes/gameboy.toml")),
    ("synthwave", include_str!("../../../themes/synthwave.toml")),
    ("paper", include_str!("../../../themes/paper.toml")),
];

pub struct ThemeRegistry {
    themes: HashMap<String, Theme>,
}

impl ThemeRegistry {
    pub fn load_all() -> Result<Self> {
        let fallback = fallback_theme();
        let mut themes = HashMap::new();

        for (name, content) in BUILTIN {
            match resolve_from_str(content, &fallback) {
                Ok(theme) => {
                    themes.insert(name.to_string(), theme);
                }
                Err(e) => warn!("built-in theme `{name}` failed: {e}"),
            }
        }

        if let Ok(dir) = crate::config::paths::themes_dir() {
            if dir.is_dir() {
                scan_dir(&dir, &fallback, &mut themes)?;
            }
        }

        if let Ok(cwd) = std::env::current_dir() {
            let project = cwd.join(".doktui").join("themes");
            if project.is_dir() {
                scan_dir(&project, &fallback, &mut themes)?;
            }
        }

        if themes.is_empty() {
            themes.insert("gruvbox-material".into(), fallback.clone());
        }

        Ok(Self { themes })
    }

    pub fn get(&self, name: &str) -> Option<&Theme> {
        self.themes.get(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.themes.keys().map(String::as_str)
    }

    pub fn active(name: &str) -> Theme {
        match Self::load_all() {
            Ok(reg) => reg
                .get(name)
                .or_else(|| reg.get("gruvbox-material"))
                .or_else(|| reg.get("pico8"))
                .cloned()
                .unwrap_or_else(fallback_theme),
            Err(e) => {
                warn!("theme registry failed: {e}");
                fallback_theme()
            }
        }
    }
}

pub fn fallback_theme() -> Theme {
    Theme {
        meta: ThemeMeta {
            name: "fallback".into(),
            display_name: "Fallback".into(),
            author: None,
            extends: None,
        },
        roles: HashMap::from([
            (Role::Bg, Color::Rgb(40, 40, 40)),
            (Role::Surface, Color::Rgb(60, 56, 54)),
            (Role::Border, Color::Rgb(80, 73, 69)),
            (Role::Text, Color::Rgb(235, 219, 178)),
            (Role::TextMuted, Color::Rgb(146, 131, 116)),
            (Role::Primary, Color::Rgb(211, 134, 155)),
            (Role::Accent, Color::Rgb(125, 174, 163)),
            (Role::Success, Color::Rgb(169, 182, 101)),
            (Role::Warning, Color::Rgb(216, 166, 87)),
            (Role::Danger, Color::Rgb(234, 105, 98)),
            (Role::Selection, Color::Rgb(211, 134, 155)),
            (Role::Cursor, Color::Rgb(125, 174, 163)),
        ]),
        glyphs: super::model::GlyphSet::from(super::model::RawGlyphs::default()),
        motion: super::model::Motion::from(super::model::RawMotion::default()),
        mascot: Mascot {
            idle: vec!["(◕‿◕)".into()],
        },
    }
}

fn resolve_from_str(content: &str, fallback: &Theme) -> Result<Theme> {
    let raw = parse_raw(content)?;
    resolve_theme(raw, |n| load_raw_by_name(n, fallback), fallback)
}

fn load_raw_by_name(name: &str, fallback: &Theme) -> Result<RawTheme> {
    if let Some((_, content)) = BUILTIN.iter().find(|(n, _)| *n == name) {
        return parse_raw(content);
    }
    if let Ok(dir) = crate::config::paths::themes_dir() {
        let path = dir.join(format!("{name}.toml"));
        if path.exists() {
            let s = std::fs::read_to_string(&path)?;
            return parse_raw(&s);
        }
    }
    let _ = fallback;
    bail!("theme `{name}` not found")
}

fn scan_dir(dir: &Path, fallback: &Theme, themes: &mut HashMap<String, Theme>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        if let Ok(theme) = resolve_from_str(&content, fallback) {
            themes.insert(theme.meta.name.clone(), theme);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads_builtin_themes() {
        let reg = ThemeRegistry::load_all().unwrap();
        assert!(reg.get("gruvbox-material").is_some());
        assert!(reg.get("pico8").is_some());
    }
}
