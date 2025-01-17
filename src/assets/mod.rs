use crate::gui::TEXT_SIZE_ICON;
use eframe::egui::{FontFamily, FontId, RichText, WidgetText};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const IMAGE_FIXATION: &[u8] = include_bytes!("images/fixation.svg");
pub const IMAGE_RUSTACEAN: &[u8] = include_bytes!("images/rustacean.svg");
// pub const IMAGE_ICON_LOGO: &[u8] = include_bytes!("images/logo.png");

pub const FONT_ICONS_BRANDS: &[u8] = include_bytes!("fonts/fa-6-brands-regular-400.otf");
pub const FONT_ICONS_REGULAR: &[u8] = include_bytes!("fonts/fa-6-free-regular-400.otf");
pub const FONT_ICONS_SOLID: &[u8] = include_bytes!("fonts/fa-6-free-solid-900.otf");

pub enum Icon {
    Help,
    SystemInfo,
    Clipboard,
    Close,
    Folder,
    FolderTree,
    MagnifyingGlass,
}
impl Icon {
    pub fn size(self, size: f32) -> RichText {
        RichText::from(self).size(size)
    }
}
impl From<Icon> for RichText {
    fn from(icon: Icon) -> Self {
        RichText::new(match icon {
            Icon::Help => "\u{f059}",
            Icon::SystemInfo => "\u{f05a}",
            Icon::Clipboard => "\u{f328}",
            Icon::Close => "\u{f00d}",
            Icon::Folder => "\u{f07b}",
            Icon::FolderTree => "\u{f802}",
            Icon::MagnifyingGlass => "\u{f002}",
        })
        .font(FontId::new(
            TEXT_SIZE_ICON,
            FontFamily::Name("fa_free".into()),
        ))
    }
}
impl From<Icon> for WidgetText {
    fn from(icon: Icon) -> Self {
        RichText::from(icon).into()
    }
}
