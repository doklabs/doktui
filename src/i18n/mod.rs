use std::fmt;
use std::fs;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use fluent::{FluentArgs, FluentResource};
use fluent_bundle::FluentBundle;
use unic_langid::LanguageIdentifier;

use crate::config::paths;

const EMBEDDED_EN: &str = include_str!("../../locales/en.ftl");

/// Thread-safe localization handle (Fluent bundle behind a mutex).
#[derive(Clone)]
pub struct I18n {
    inner: Arc<Mutex<I18nInner>>,
}

impl fmt::Debug for I18n {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("I18n")
            .field("locale", &self.locale())
            .finish()
    }
}

struct I18nInner {
    locale: String,
    bundle: FluentBundle<FluentResource>,
    fallback: bool,
}

impl I18n {
    /// Load locale from user config dir, falling back to embedded English.
    pub fn load(locale: &str) -> Result<Self> {
        let langid: LanguageIdentifier = locale
            .parse()
            .unwrap_or_else(|_| "en".parse().expect("en is valid"));
        let mut bundle = FluentBundle::new(vec![langid]);
        let mut fallback = false;

        let user_path = paths::locales_dir()
            .ok()
            .map(|d| d.join(format!("{locale}.ftl")));
        let loaded = user_path
            .as_ref()
            .filter(|p| p.exists())
            .and_then(|p| fs::read_to_string(p).ok());

        let source = if let Some(content) = loaded {
            content
        } else if locale != "en" {
            fallback = true;
            tracing::warn!("locale `{locale}` not found — falling back to English");
            EMBEDDED_EN.to_string()
        } else {
            EMBEDDED_EN.to_string()
        };

        let resource = FluentResource::try_new(source).map_err(|(_, errors)| {
            anyhow::anyhow!("failed to parse locale resource: {errors:?}")
        })?;
        bundle
            .add_resource(resource)
            .map_err(|errors| anyhow::anyhow!("failed to add locale resource to bundle: {errors:?}"))?;

        Ok(Self {
            inner: Arc::new(Mutex::new(I18nInner {
                locale: locale.to_string(),
                bundle,
                fallback,
            })),
        })
    }

    pub fn locale(&self) -> String {
        self.inner.lock().expect("i18n lock").locale.clone()
    }

    pub fn used_fallback(&self) -> bool {
        self.inner.lock().expect("i18n lock").fallback
    }

    /// Resolve a message id; returns the id itself when missing.
    pub fn t(&self, key: &str) -> String {
        self.t_fmt(key, &[])
    }

    /// Resolve with Fluent placeholders, e.g. `t_fmt("greeting", &[("name", "Ada")])`.
    pub fn t_fmt(&self, key: &str, args: &[(&str, &str)]) -> String {
        let inner = self.inner.lock().expect("i18n lock");
        let msg = match inner.bundle.get_message(key) {
            Some(m) => m,
            None => return key.to_string(),
        };
        let pattern = match msg.value() {
            Some(p) => p,
            None => return key.to_string(),
        };
        let mut fluent_args = FluentArgs::new();
        for (k, v) in args {
            fluent_args.set(*k, v.to_string());
        }
        inner
            .bundle
            .format_pattern(pattern, Some(&fluent_args), &mut Vec::new())
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_embedded_english() {
        let i18n = I18n::load("en").expect("en loads");
        assert_eq!(i18n.t("nav-home"), "Home");
        assert_eq!(i18n.t("brand-name"), "DokTUI");
    }

    #[test]
    fn missing_key_returns_key() {
        let i18n = I18n::load("en").unwrap();
        assert_eq!(i18n.t("nonexistent.key.xyz"), "nonexistent.key.xyz");
    }

    #[test]
    fn fmt_substitutes_placeholders() {
        let i18n = I18n::load("en").unwrap();
        let s = i18n.t_fmt("confirm-remove-container", &[("name", "web")]);
        assert!(s.contains("web"));
    }
}
