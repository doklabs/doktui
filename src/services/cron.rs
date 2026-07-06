use anyhow::Result;
use chrono::{DateTime, Utc};
use cron::Schedule;

/// Validate a cron expression (5-field standard syntax).
pub fn validate_expression(expr: &str) -> Result<()> {
    expr.trim()
        .parse::<Schedule>()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("invalid cron expression: {e}"))
}

/// True when a scheduled run is due since `last_run` (or never run).
pub fn is_due(expression: &str, last_run: Option<&str>) -> Result<bool> {
    let schedule: Schedule = expression
        .trim()
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid cron: {e}"))?;
    let now = Utc::now();
    let window_start = if let Some(ts) = last_run {
        DateTime::parse_from_rfc3339(ts)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| now - chrono::Duration::minutes(2))
    } else {
        now - chrono::Duration::minutes(2)
    };
    Ok(schedule
        .after(&window_start)
        .next()
        .is_some_and(|next| next <= now))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_cron() {
        assert!(validate_expression("0 0 3 * * *").is_ok());
    }

    #[test]
    fn rejects_invalid_cron() {
        assert!(validate_expression("not-a-cron").is_err());
    }
}
