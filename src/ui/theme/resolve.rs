use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use ratatui::style::Color;

use super::model::{
    GlyphSet, Mascot, Motion, RawGlyphs, RawMascot, RawMotion, RawTheme, Role, Theme, ThemeMeta,
    parse_hex,
};

pub fn merge_raw(base: RawTheme, over: RawTheme) -> RawTheme {
    let mut palette = base.palette;
    palette.extend(over.palette);

    let mut roles = base.roles;
    roles.extend(over.roles);

    RawTheme {
        meta: over.meta,
        palette,
        roles,
        glyphs: merge_glyphs(base.glyphs, over.glyphs),
        motion: merge_motion(base.motion, over.motion),
        mascot: merge_mascot(base.mascot, over.mascot),
    }
}

fn merge_glyphs(base: RawGlyphs, over: RawGlyphs) -> RawGlyphs {
    RawGlyphs {
        bar_full: pick(over.bar_full, base.bar_full),
        bar_empty: pick(over.bar_empty, base.bar_empty),
        spinner: if over.spinner.is_empty() {
            base.spinner
        } else {
            over.spinner
        },
        dot_on: pick(over.dot_on, base.dot_on),
        dot_warn: pick(over.dot_warn, base.dot_warn),
        dot_off: pick(over.dot_off, base.dot_off),
        sparkline: if over.sparkline.is_empty() {
            base.sparkline
        } else {
            over.sparkline
        },
        check: pick(over.check, base.check),
        arrow: pick(over.arrow, base.arrow),
        star: pick(over.star, base.star),
        diamond: pick(over.diamond, base.diamond),
        cross: pick(over.cross, base.cross),
        warning: pick(over.warning, base.warning),
        info: pick(over.info, base.info),
    }
}

fn pick(over: String, base: String) -> String {
    if over.is_empty() {
        base
    } else {
        over
    }
}

fn merge_motion(base: RawMotion, over: RawMotion) -> RawMotion {
    let _ = base;
    over
}

fn merge_mascot(base: RawMascot, over: RawMascot) -> RawMascot {
    RawMascot {
        idle: if over.idle.is_empty() { base.idle } else { over.idle },
    }
}

fn resolve_color(value: &str, palette: &HashMap<String, String>) -> Result<Color> {
    let trimmed = value.trim();
    if trimmed.starts_with('#') {
        return parse_hex(trimmed)
            .with_context(|| format!("invalid hex color `{trimmed}`"));
    }
    if let Some(hex) = palette.get(trimmed) {
        return parse_hex(hex).with_context(|| format!("palette `{trimmed}` → `{hex}` invalid"));
    }
    bail!("unknown palette key `{trimmed}`")
}

pub fn raw_to_theme(raw: RawTheme, fallback: &Theme) -> Result<Theme> {
    let mut roles = HashMap::new();
    for (key, v) in &raw.roles {
        if let Some(role) = Role::from_key(key) {
            roles.insert(role, resolve_color(v, &raw.palette)?);
        } else {
            tracing::warn!("unknown theme role key `{key}`");
        }
    }
    for role in Role::ALL {
        if !roles.contains_key(&role) {
            roles.insert(role, fallback.color(role));
        }
    }

    Ok(Theme {
        meta: ThemeMeta {
            name: raw.meta.name,
            display_name: raw.meta.display_name,
            author: raw.meta.author,
            extends: raw.meta.extends,
        },
        roles,
        glyphs: GlyphSet::from(raw.glyphs),
        motion: Motion::from(raw.motion),
        mascot: Mascot::from(raw.mascot),
    })
}

pub fn parse_raw(toml_str: &str) -> Result<RawTheme> {
    toml::from_str(toml_str).context("failed to parse theme TOML")
}

pub fn resolve_theme(
    raw: RawTheme,
    load_base: impl Fn(&str) -> Result<RawTheme>,
    fallback: &Theme,
) -> Result<Theme> {
    let merged = if let Some(base_name) = &raw.meta.extends {
        let base = load_base(base_name)?;
        merge_raw(base, raw)
    } else {
        raw
    };
    super::validate::validate(&merged)
        .context("theme validation failed")?;
    raw_to_theme(merged, fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gruvbox_material_embedded_resolves() {
        let fallback = super::super::registry::fallback_theme();
        let raw = parse_raw(include_str!("../../../themes/gruvbox-material.toml")).unwrap();
        let theme = raw_to_theme(raw, &fallback).unwrap();
        assert!(theme.color(Role::Primary) != Color::Reset);
    }
}
