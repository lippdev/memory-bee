//! Color tokens for the dashboard. `NO_COLOR` (or `--no-color`) drops every
//! color to the terminal default; state is still readable through the glyphs
//! `●`/`◐`/`▲` drawn by the UI layer.
use ratatui::style::Color;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Dark,
    Light,
}

impl Mode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "dark" => Some(Mode::Dark),
            "light" => Some(Mode::Light),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub ink: Color,
    pub ink2: Color,
    pub ink3: Color,
    pub ink4: Color,
    pub faint: Color,
    pub accent: Color,
    pub ok: Color,
    pub warn: Color,
}

const DARK: Palette = Palette {
    bg: Color::Rgb(0x15, 0x15, 0x13),
    ink: Color::Rgb(0xee, 0xec, 0xe6),
    ink2: Color::Rgb(0xb3, 0xaf, 0xa5),
    ink3: Color::Rgb(0x85, 0x82, 0x79),
    ink4: Color::Rgb(0x55, 0x53, 0x4d),
    faint: Color::Rgb(0x35, 0x34, 0x30),
    accent: Color::Rgb(0xee, 0xb6, 0x5b),
    ok: Color::Rgb(0x9d, 0xbb, 0x8e),
    warn: Color::Rgb(0xe4, 0x8a, 0x6c),
};

const LIGHT: Palette = Palette {
    bg: Color::Rgb(0xfa, 0xf9, 0xf6),
    ink: Color::Rgb(0x1b, 0x1a, 0x17),
    ink2: Color::Rgb(0x4a, 0x48, 0x42),
    ink3: Color::Rgb(0x7a, 0x77, 0x6f),
    ink4: Color::Rgb(0xae, 0xab, 0xa1),
    faint: Color::Rgb(0xde, 0xdb, 0xd2),
    accent: Color::Rgb(0xd4, 0x9a, 0x2c),
    ok: Color::Rgb(0x4f, 0x7d, 0x46),
    warn: Color::Rgb(0xb5, 0x50, 0x2e),
};

const NONE: Palette = Palette {
    bg: Color::Reset,
    ink: Color::Reset,
    ink2: Color::Reset,
    ink3: Color::Reset,
    ink4: Color::Reset,
    faint: Color::Reset,
    accent: Color::Reset,
    ok: Color::Reset,
    warn: Color::Reset,
};

impl Palette {
    pub fn new(mode: Mode, no_color: bool) -> Self {
        if no_color {
            return NONE;
        }
        match mode {
            Mode::Dark => DARK,
            Mode::Light => LIGHT,
        }
    }
}
