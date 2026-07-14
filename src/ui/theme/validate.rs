use std::collections::HashSet;

use anyhow::{Context, Result, bail};

use super::model::{RawTheme, Role, parse_hex};

/// Validate a raw theme after inheritance merge and before color resolution.
///
/// Returns `Ok(())` for valid themes and descriptive errors for common mistakes
/// (empty name, invalid hex colors, references to missing palette keys, etc.).
pub fn validate(raw: &RawTheme) -> Result<()> {
    if raw.meta.name.trim().is_empty() {
        bail!("theme meta.name is empty");
    }
    if raw.meta.display_name.trim().is_empty() {
        bail!("theme meta.display_name is empty");
    }

    for (key, value) in &raw.palette {
        parse_hex(value)
            .with_context(|| format!("palette color `{key}` has invalid hex `{value}`"))?;
    }

    let palette_keys: HashSet<&str> = raw.palette.keys().map(|s| s.as_str()).collect();

    for (key, value) in &raw.roles {
        if Role::from_key(key).is_none() {
            // Unknown role keys are not fatal; the resolver already logs them.
            // We still check the value in case a typo is hiding an invalid color.
        }
        let trimmed = value.trim();
        if trimmed.starts_with('#') {
            parse_hex(trimmed)
                .with_context(|| format!("role `{key}` has invalid hex color `{trimmed}`"))?;
        } else if !palette_keys.contains(trimmed) {
            let available: Vec<_> = palette_keys.iter().copied().collect();
            bail!(
                "role `{key}` references unknown palette key `{trimmed}` (available: {available:?})"
            );
        }
    }

    if raw.glyphs.spinner.is_empty() {
        bail!("glyphs.spinner must not be empty");
    }
    if raw.glyphs.sparkline.is_empty() {
        bail!("glyphs.sparkline must not be empty");
    }

    if raw.motion.enabled && raw.motion.blink_every == 0 {
        bail!("motion.blink_every must be > 0 when motion.enabled is true");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_missing_palette_reference() {
        let mut raw = empty_theme();
        raw.palette.insert("void".into(), "#1a1b26".into());
        raw.roles.insert("bg".into(), "nope".into());
        assert!(validate(&raw).is_err());
    }

    #[test]
    fn validates_invalid_hex() {
        let mut raw = empty_theme();
        raw.palette.insert("void".into(), "not-a-color".into());
        assert!(validate(&raw).is_err());
    }

    #[test]
    fn validates_empty_meta() {
        let raw = empty_theme();
        assert!(validate(&raw).is_err());
    }

    fn empty_theme() -> RawTheme {
        RawTheme {
            meta: super::super::model::ThemeMeta {
                name: "".into(),
                display_name: "".into(),
                author: None,
                extends: None,
            },
            palette: Default::default(),
            roles: Default::default(),
            glyphs: Default::default(),
            motion: Default::default(),
            mascot: Default::default(),
        }
    }
}
