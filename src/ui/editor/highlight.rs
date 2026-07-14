use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::ui::theme::{Role, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorLanguage {
    Yaml,
    Toml,
    Env,
    Dockerfile,
    Json,
    Plain,
}

pub fn highlight_line(
    language: EditorLanguage,
    source: &str,
    line_idx: usize,
    theme: &Theme,
) -> Line<'static> {
    let line = source.lines().nth(line_idx).unwrap_or("");
    match language {
        EditorLanguage::Env => highlight_env(line, theme),
        EditorLanguage::Yaml => highlight_yaml(line, theme),
        EditorLanguage::Dockerfile => highlight_dockerfile(line, theme),
        EditorLanguage::Toml => highlight_toml(line, theme),
        EditorLanguage::Json => highlight_json(line, theme),
        EditorLanguage::Plain => Line::from(vec![Span::styled(
            line.to_string(),
            theme.style(Role::Text),
        )]),
    }
}

fn highlight_env(line: &str, theme: &Theme) -> Line<'static> {
    if line.trim_start().starts_with('#') {
        return Line::from(vec![Span::styled(line.to_string(), comment(theme))]);
    }
    if let Some((key, rest)) = line.split_once('=') {
        return Line::from(vec![
            Span::styled(key.to_string(), key_style(theme)),
            Span::styled("=".to_string(), theme.style(Role::Text)),
            Span::styled(rest.to_string(), string_style(theme)),
        ]);
    }
    Line::from(vec![Span::styled(
        line.to_string(),
        theme.style(Role::Text),
    )])
}

fn highlight_yaml(line: &str, theme: &Theme) -> Line<'static> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return Line::from(vec![Span::styled(line.to_string(), comment(theme))]);
    }
    if trimmed.starts_with("- ") {
        let rest = &line[line.find('-').unwrap_or(0) + 1..];
        return Line::from(vec![
            Span::styled("-".to_string(), keyword(theme)),
            Span::styled(" ".to_string(), theme.style(Role::Text)),
            Span::styled(rest.trim_start().to_string(), string_style(theme)),
        ]);
    }
    highlight_kv(line, theme)
}

fn highlight_dockerfile(line: &str, theme: &Theme) -> Line<'static> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return Line::from(vec![Span::styled(line.to_string(), comment(theme))]);
    }
    let upper = trimmed.to_ascii_uppercase();
    const INSTRUCTIONS: [&str; 12] = [
        "FROM",
        "RUN",
        "CMD",
        "LABEL",
        "EXPOSE",
        "ENV",
        "ADD",
        "COPY",
        "ENTRYPOINT",
        "VOLUME",
        "USER",
        "WORKDIR",
    ];
    for instr in INSTRUCTIONS {
        if upper.starts_with(instr)
            && trimmed
                .get(instr.len()..)
                .map(|rest: &str| rest.is_empty() || rest.starts_with(' '))
                .unwrap_or(false)
        {
            let split = line.find(instr).unwrap_or(0);
            return Line::from(vec![
                Span::styled(line[..split].to_string(), theme.style(Role::Text)),
                Span::styled(instr.to_string(), keyword(theme)),
                Span::styled(line[split + instr.len()..].to_string(), string_style(theme)),
            ]);
        }
    }
    highlight_kv(line, theme)
}

fn highlight_kv(line: &str, theme: &Theme) -> Line<'static> {
    if line.trim_start().starts_with('#') {
        return Line::from(vec![Span::styled(line.to_string(), comment(theme))]);
    }
    let mut spans = Vec::new();
    let mut rest = line;
    if let Some(idx) = line.find(':') {
        spans.push(Span::styled(line[..idx].to_string(), key_style(theme)));
        spans.push(Span::styled(":".to_string(), theme.style(Role::Text)));
        rest = &line[idx + 1..];
    }
    if rest.contains('"') || rest.contains('\'') {
        spans.push(Span::styled(rest.to_string(), string_style(theme)));
    } else {
        spans.push(Span::styled(rest.to_string(), theme.style(Role::Text)));
    }
    if spans.is_empty() {
        Line::from(vec![Span::styled(
            line.to_string(),
            theme.style(Role::Text),
        )])
    } else {
        Line::from(spans)
    }
}

fn highlight_toml(line: &str, theme: &Theme) -> Line<'static> {
    if line.trim_start().starts_with('#') {
        return Line::from(vec![Span::styled(line.to_string(), comment(theme))]);
    }
    if line.contains('=') {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            return Line::from(vec![
                Span::styled(parts[0].to_string(), key_style(theme)),
                Span::styled("=".to_string(), theme.style(Role::Text)),
                Span::styled(parts[1].to_string(), string_style(theme)),
            ]);
        }
    }
    if line.starts_with('[') {
        return Line::from(vec![Span::styled(line.to_string(), keyword(theme))]);
    }
    Line::from(vec![Span::styled(
        line.to_string(),
        theme.style(Role::Text),
    )])
}

fn highlight_json(line: &str, theme: &Theme) -> Line<'static> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return Line::from(vec![Span::styled(line.to_string(), comment(theme))]);
    }
    if trimmed.contains('"') {
        return highlight_kv(line, theme);
    }
    if trimmed.ends_with('{')
        || trimmed.ends_with('}')
        || trimmed.ends_with('[')
        || trimmed.ends_with(']')
    {
        return Line::from(vec![Span::styled(line.to_string(), keyword(theme))]);
    }
    Line::from(vec![Span::styled(
        line.to_string(),
        theme.style(Role::Text),
    )])
}

fn comment(theme: &Theme) -> Style {
    theme.style(Role::TextMuted)
}

fn key_style(theme: &Theme) -> Style {
    theme.style(Role::Primary)
}

fn string_style(theme: &Theme) -> Style {
    theme.style(Role::Success)
}

fn keyword(theme: &Theme) -> Style {
    theme.style(Role::Accent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_key_value_colored() {
        let theme = crate::ui::theme::ThemeRegistry::active("pico8");
        let line = highlight_line(EditorLanguage::Env, "FOO=bar", 0, &theme);
        assert!(!line.spans.is_empty());
    }
}
