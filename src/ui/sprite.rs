use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::ui::theme::{Role, Theme};

/// Palette-indexed sprite; 0 = transparent.
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub px: &'static [u8],
}

const W: usize = 15;
const H: usize = 12;

// DokTUI mascot — "Doko": an original Terminal Crate, a rebrand away from the
// Docker whale / Dokploy logo. A shipping crate with a dark screen face showing
// a `>` prompt and a blinking cursor. Frames animate via anim_tick.
// 0=empty 1=crate 2=outline 3=screen 4=prompt/cursor 5=signal 6=slat 7=bolt 8=cursor-off 9=text/ok

const PX_IDLE: [u8; W * H] = mk_sprite(&[
    b"...............",
    b"..BBBBBBBBBBB..",
    b".BSSSSSSSSSSSB.",
    b".BSESSEESSSSSB.",
    b".BSSESEESSSSSB.",
    b".BSESSEESSSSSB.",
    b".BSSSSSSSSSSSB.",
    b".BAAAAAAAAAAAB.",
    b".BBBBBBBBBBBBB.",
    b".BAAAAAAAAAAAB.",
    b"..BBBBBBBBBBB..",
    b"..HH.....HH....",
]);

// Blinking terminal cursor — the crate's signature idle animation.
const PX_BLINK: [u8; W * H] = mk_sprite(&[
    b"...............",
    b"..BBBBBBBBBBB..",
    b".BSSSSSSSSSSSB.",
    b".BSESSxxSSSSSB.",
    b".BSSESxxSSSSSB.",
    b".BSESSxxSSSSSB.",
    b".BSSSSSSSSSSSB.",
    b".BAAAAAAAAAAAB.",
    b".BBBBBBBBBBBBB.",
    b".BAAAAAAAAAAAB.",
    b"..BBBBBBBBBBB..",
    b"..HH.....HH....",
]);

// Typing frame 1 — a char appears, cursor advances.
const PX_LOOK_L: [u8; W * H] = mk_sprite(&[
    b"...............",
    b"..BBBBBBBBBBB..",
    b".BSSSSSSSSSSSB.",
    b".BSESSmSEESSSB.",
    b".BSSESmSEESSSB.",
    b".BSESSmSEESSSB.",
    b".BSSSSSSSSSSSB.",
    b".BAAAAAAAAAAAB.",
    b".BBBBBBBBBBBBB.",
    b".BAAAAAAAAAAAB.",
    b"..BBBBBBBBBBB..",
    b"..HH.....HH....",
]);

// Typing frame 2 — more typed, cursor further right.
const PX_LOOK_R: [u8; W * H] = mk_sprite(&[
    b"...............",
    b"..BBBBBBBBBBB..",
    b".BSSSSSSSSSSSB.",
    b".BSESSmmSEESSB.",
    b".BSSESmmSEESSB.",
    b".BSESSmmSEESSB.",
    b".BSSSSSSSSSSSB.",
    b".BAAAAAAAAAAAB.",
    b".BBBBBBBBBBBBB.",
    b".BAAAAAAAAAAAB.",
    b"..BBBBBBBBBBB..",
    b"..HH.....HH....",
]);

// Success — screen fills green (deploy OK).
const PX_HAPPY: [u8; W * H] = mk_sprite(&[
    b"...............",
    b"..BBBBBBBBBBB..",
    b".BSSSSSSSSSSSB.",
    b".BSESSmmmmSSSB.",
    b".BSSESmmmmSSSB.",
    b".BSESSmmmmSSSB.",
    b".BSSSSSSSSSSSB.",
    b".BAAAAAAAAAAAB.",
    b".BBBBBBBBBBBBB.",
    b".BAAAAAAAAAAAB.",
    b"..BBBBBBBBBBB..",
    b"..HH.....HH....",
]);

// Broadcasting — signal ticks above the crate (plays on connect/deploy).
const PX_SPOUT: [u8; W * H] = mk_sprite(&[
    b"....L.L.L.L....",
    b"..BBBBBBBBBBB..",
    b".BSSSSSSSSSSSB.",
    b".BSESSEESSSSSB.",
    b".BSSESEESSSSSB.",
    b".BSESSEESSSSSB.",
    b".BSSSSSSSSSSSB.",
    b".BAAAAAAAAAAAB.",
    b".BBBBBBBBBBBBB.",
    b".BAAAAAAAAAAAB.",
    b"..BBBBBBBBBBB..",
    b"..HH.....HH....",
]);

// Screen glitch — cursor drop + stray artifacts, 1-frame flicker.
const PX_GLITCH: [u8; W * H] = mk_sprite(&[
    b"...............",
    b"..BBBBBBBBBBB..",
    b".BSSSSSSSSSSSB.",
    b".BSESSxxSSSmSB.",
    b".BSSESxxmSSSSB.",
    b".BSESSxxSSSSSB.",
    b".BSSSSSSSSSSSB.",
    b".BAAAAAAAAAAAB.",
    b".BBBBBBBBBBBBB.",
    b".BAAAAAAAAAAAB.",
    b"..BBBBBBBBBBB..",
    b"..HH.....HH....",
]);

const fn ch(c: u8) -> u8 {
    match c {
        b'.' => 0,
        b'B' => 1, // crate body
        b'D' => 2, // outline / shadow
        b'S' => 3, // screen panel
        b'E' => 4, // prompt / cursor
        b'L' => 5, // signal tick
        b'A' => 6, // slat line
        b'H' => 7, // corner bolt
        b'x' => 8, // cursor off (blink)
        b'm' => 9, // typed text / success
        _ => 0,
    }
}

const fn mk_row(s: &[u8; W]) -> [u8; W] {
    let mut out = [0u8; W];
    let mut i = 0;
    while i < W {
        out[i] = ch(s[i]);
        i += 1;
    }
    out
}

const fn mk_sprite(rows: &[&[u8; W]; H]) -> [u8; W * H] {
    let mut px = [0u8; W * H];
    let mut y = 0;
    while y < H {
        let row = mk_row(rows[y]);
        let mut x = 0;
        while x < W {
            px[y * W + x] = row[x];
            x += 1;
        }
        y += 1;
    }
    px
}

pub const MASCOT_IDLE: Sprite = Sprite {
    w: W,
    h: H,
    px: &PX_IDLE,
};
pub const MASCOT_BLINK: Sprite = Sprite {
    w: W,
    h: H,
    px: &PX_BLINK,
};
pub const MASCOT_LOOK_L: Sprite = Sprite {
    w: W,
    h: H,
    px: &PX_LOOK_L,
};
pub const MASCOT_LOOK_R: Sprite = Sprite {
    w: W,
    h: H,
    px: &PX_LOOK_R,
};
pub const MASCOT_HAPPY: Sprite = Sprite {
    w: W,
    h: H,
    px: &PX_HAPPY,
};
pub const MASCOT_SPOUT: Sprite = Sprite {
    w: W,
    h: H,
    px: &PX_SPOUT,
};
pub const MASCOT_GLITCH: Sprite = Sprite {
    w: W,
    h: H,
    px: &PX_GLITCH,
};

pub fn mascot_palette(theme: &Theme) -> [Color; 10] {
    [
        theme.color(Role::Bg),        // 0 transparent
        theme.color(Role::Primary),   // 1 crate body (brand color)
        theme.color(Role::TextMuted), // 2 outline / shadow
        theme.color(Role::Bg),        // 3 screen panel (recessed dark)
        theme.color(Role::Accent),    // 4 prompt / cursor (glow)
        theme.color(Role::Warning),   // 5 signal tick
        theme.color(Role::TextMuted), // 6 slat lines
        theme.color(Role::Success),   // 7 corner bolts
        theme.color(Role::Bg),        // 8 cursor off (matches screen)
        theme.color(Role::Success),   // 9 typed text / success
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MascotAnim {
    Idle,
    Blink,
    LookLeft,
    LookRight,
    Happy,
    Spout,
    Glitch,
}

pub fn mascot_anim(anim_tick: u64, theme: &Theme) -> MascotAnim {
    if !theme.motion.enabled {
        return MascotAnim::Idle;
    }

    if theme.motion.blink_every > 0 && !theme.mascot.blink.is_empty() {
        let phase = anim_tick % theme.motion.blink_every;
        if phase == 0 || phase == 1 {
            return MascotAnim::Blink;
        }
    } else if theme.motion.blink_every > 0 {
        let phase = anim_tick % theme.motion.blink_every;
        if phase == 0 || phase == 1 {
            return MascotAnim::Blink;
        }
    }

    // Brief CRT glitch flicker every ~4s at 15fps.
    if anim_tick % 64 == 32 || anim_tick % 64 == 33 {
        return MascotAnim::Glitch;
    }

    match (anim_tick / 90) % 24 {
        14..=15 => MascotAnim::Spout,
        20..=21 => MascotAnim::LookLeft,
        22..=23 => MascotAnim::LookRight,
        10..=13 => MascotAnim::Happy,
        _ => MascotAnim::Idle,
    }
}

pub fn mascot_sprite_for(anim: MascotAnim) -> &'static Sprite {
    match anim {
        MascotAnim::Idle => &MASCOT_IDLE,
        MascotAnim::Blink => &MASCOT_BLINK,
        MascotAnim::LookLeft => &MASCOT_LOOK_L,
        MascotAnim::LookRight => &MASCOT_LOOK_R,
        MascotAnim::Happy => &MASCOT_HAPPY,
        MascotAnim::Spout => &MASCOT_SPOUT,
        MascotAnim::Glitch => &MASCOT_GLITCH,
    }
}

/// Gentle idle hover bob.
pub fn mascot_bob(anim_tick: u64) -> u16 {
    match (anim_tick / 6) % 4 {
        1 | 2 => 1,
        _ => 0,
    }
}

pub fn render_sprite(s: &Sprite, pal: &[Color]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut y = 0;
    while y < s.h {
        let mut spans = Vec::new();
        for x in 0..s.w {
            let top = s.px[y * s.w + x] as usize;
            let bot = if y + 1 < s.h {
                s.px[(y + 1) * s.w + x] as usize
            } else {
                0
            };
            if top == 0 && bot == 0 {
                spans.push(Span::raw(" "));
                continue;
            }
            let fg = pal.get(top).copied().unwrap_or(Color::Reset);
            let bg = pal.get(bot).copied().unwrap_or(Color::Reset);
            spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
        }
        lines.push(Line::from(spans));
        y += 2;
    }
    lines
}

/// One-line mascot glyph for the app header.
pub fn mascot_header_glyph(theme: &Theme, anim_tick: u64) -> String {
    let sprite = mascot_sprite_for(mascot_anim(anim_tick, theme));
    let lines = render_sprite(sprite, &mascot_palette(theme));
    lines
        .first()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| ">_".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_renders_half_height_lines() {
        let pal = [Color::Black; 10];
        let lines = render_sprite(&MASCOT_IDLE, &pal);
        assert_eq!(lines.len(), MASCOT_IDLE.h / 2);
    }

    #[test]
    fn mascot_cycles_animations() {
        let theme = crate::ui::theme::ThemeRegistry::active("pico8");
        assert_eq!(mascot_anim(0, &theme), MascotAnim::Blink);
        assert_eq!(mascot_anim(5, &theme), MascotAnim::Idle);
        assert_eq!(mascot_anim(90 * 14 + 5, &theme), MascotAnim::Spout);
    }

    #[test]
    fn header_glyph_non_empty() {
        let theme = crate::ui::theme::ThemeRegistry::active("pico8");
        let g = mascot_header_glyph(&theme, 10);
        assert!(!g.is_empty());
    }
}
