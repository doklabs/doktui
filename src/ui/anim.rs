use crate::ui::theme::Theme;

/// Current spinner glyph for this tick.
pub fn spinner(theme: &Theme, tick: u64) -> &str {
    if !theme.motion.enabled || theme.glyphs.spinner.is_empty() {
        return "◌";
    }
    let n = theme.glyphs.spinner.len() as u64;
    let period = theme.motion.spinner_period.max(1);
    &theme.glyphs.spinner[((tick / period) % n) as usize]
}

/// Marching lit segments for progress shimmer.
pub fn marching(tick: u64, total: usize, lit: usize) -> Vec<bool> {
    if total == 0 {
        return vec![];
    }
    let lit = lit.min(total);
    (0..total)
        .map(|i| ((i as u64 + tick) % total as u64) < lit as u64)
        .collect()
}

/// Pulse intensity 0.0–1.0.
pub fn pulse(theme: &Theme, tick: u64) -> f32 {
    if !theme.motion.enabled {
        return 1.0;
    }
    let p = theme.motion.pulse_period.max(1);
    let phase = (tick % p) as f32 / p as f32;
    (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5
}

/// Block progress bar string using theme glyphs.
pub fn progress_bar(theme: &Theme, tick: u64, width: usize, percent: u8) -> String {
    let width = width.max(1);
    let filled = (width * percent as usize / 100).min(width);
    let marching = marching(tick, width, filled);
    marching
        .iter()
        .map(|on| {
            if *on {
                theme.glyphs.bar_full.as_str()
            } else {
                theme.glyphs.bar_empty.as_str()
            }
        })
        .collect()
}

/// CPU-style bar with gradient roles via percent thresholds.
pub fn gradient_bar(theme: &Theme, width: usize, percent: u8) -> String {
    let width = width.max(1);
    let filled = (width * percent as usize / 100).min(width);
    (0..width)
        .map(|i| {
            if i >= filled {
                return theme.glyphs.bar_empty.clone();
            }
            let p = (i * 100 / width) as u8;
            let ch = theme.glyphs.bar_full.clone();
            let _ = p;
            ch
        })
        .collect()
}
