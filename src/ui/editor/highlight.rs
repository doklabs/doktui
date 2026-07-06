use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorLanguage {
    Yaml,
    Toml,
    Env,
    Dockerfile,
    Json,
    Plain,
}

pub fn highlight_line(language: EditorLanguage, source: &str, line_idx: usize) -> Line<'static> {
    let line = source.lines().nth(line_idx).unwrap_or("");
    match language {
        EditorLanguage::Env => highlight_env(line),
        EditorLanguage::Yaml => highlight_yaml(line),
        EditorLanguage::Dockerfile => highlight_dockerfile(line),
        EditorLanguage::Toml => highlight_toml(line),
        EditorLanguage::Json => highlight_json(line),
        EditorLanguage::Plain => Line::from(line.to_string()),
    }
}

fn highlight_env(line: &str) -> Line<'static> {
    if line.trim_start().starts_with('#') {
        return Line::from(Span::styled(line.to_string(), comment()));
    }
    if let Some((key, rest)) = line.split_once('=') {
        return Line::from(vec![
            Span::styled(key.to_string(), key_style()),
            Span::raw("=".to_string()),
            Span::styled(rest.to_string(), string_style()),
        ]);
    }
    Line::from(line.to_string())
}

fn highlight_yaml(line: &str) -> Line<'static> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return Line::from(Span::styled(line.to_string(), comment()));
    }
    if trimmed.starts_with("- ") {
        let rest = &line[line.find('-').unwrap_or(0) + 1..];
        return Line::from(vec![
            Span::styled("-".to_string(), keyword()),
            Span::raw(" ".to_string()),
            Span::styled(rest.trim_start().to_string(), string_style()),
        ]);
    }
    highlight_kv(line)
}

fn highlight_dockerfile(line: &str) -> Line<'static> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return Line::from(Span::styled(line.to_string(), comment()));
    }
    let upper = trimmed.to_ascii_uppercase();
    const INSTRUCTIONS: [&str; 12] = [
        "FROM", "RUN", "CMD", "LABEL", "EXPOSE", "ENV", "ADD", "COPY", "ENTRYPOINT", "VOLUME",
        "USER", "WORKDIR",
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
                Span::raw(line[..split].to_string()),
                Span::styled(instr.to_string(), keyword()),
                Span::styled(line[split + instr.len()..].to_string(), string_style()),
            ]);
        }
    }
    highlight_kv(line)
}

fn highlight_kv(line: &str) -> Line<'static> {
    if line.trim_start().starts_with('#') {
        return Line::from(Span::styled(line.to_string(), comment()));
    }
    let mut spans = Vec::new();
    let mut rest = line;
    if let Some(idx) = line.find(':') {
        spans.push(Span::styled(line[..idx].to_string(), key_style()));
        spans.push(Span::raw(":".to_string()));
        rest = &line[idx + 1..];
    }
    if rest.contains('"') || rest.contains('\'') {
        spans.push(Span::styled(rest.to_string(), string_style()));
    } else {
        spans.push(Span::raw(rest.to_string()));
    }
    if spans.is_empty() {
        Line::from(line.to_string())
    } else {
        Line::from(spans)
    }
}

fn highlight_toml(line: &str) -> Line<'static> {
    if line.trim_start().starts_with('#') {
        return Line::from(Span::styled(line.to_string(), comment()));
    }
    if line.contains('=') {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            return Line::from(vec![
                Span::styled(parts[0].to_string(), key_style()),
                Span::raw("=".to_string()),
                Span::styled(parts[1].to_string(), string_style()),
            ]);
        }
    }
    if line.starts_with('[') {
        return Line::from(Span::styled(line.to_string(), keyword()));
    }
    Line::from(line.to_string())
}

fn highlight_json(line: &str) -> Line<'static> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return Line::from(Span::styled(line.to_string(), comment()));
    }
    if trimmed.contains('"') {
        return highlight_kv(line);
    }
    if trimmed.ends_with('{') || trimmed.ends_with('}') || trimmed.ends_with('[') || trimmed.ends_with(']') {
        return Line::from(Span::styled(line.to_string(), keyword()));
    }
    Line::from(line.to_string())
}

fn comment() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn key_style() -> Style {
    Style::default().fg(Color::Yellow)
}

fn string_style() -> Style {
    Style::default().fg(Color::Green)
}

fn keyword() -> Style {
    Style::default().fg(Color::Cyan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_key_value_colored() {
        let line = highlight_line(EditorLanguage::Env, "FOO=bar", 0);
        assert!(!line.spans.is_empty());
    }
}
