use crate::gui::text::body;
use eframe::egui::Ui;
use egui_demo_lib::easy_mark::easy_mark;
use eyre::{eyre, Result};
use regex;
use regex::Regex;
use std::path::PathBuf;
use std::str::FromStr;

pub fn text_or_file(text: &str) -> Option<PathBuf> {
    let re = Regex::new(r"^!!\[(.*)]$").unwrap();
    if let Some(m) = re.captures(text) {
        if let Ok(path) = PathBuf::from_str(&m[1]) {
            return Some(path);
        }
    }

    None
}

pub fn parse_text(ui: &mut Ui, text: &str) -> Result<()> {
    let re = Regex::new(r"^!!<([[:alpha:]][[:word:]]*)>[ \t]*\n?([ \t]*\n)?").unwrap();
    if let Some(caps) = re.captures(text) {
        match &caps[1] {
            "easy_mark" => {
                easy_mark(ui, &re.replace(text, ""));
                Ok(())
            }
            parser => Err(eyre!("Unknown text parser ({parser}).")),
        }
    } else {
        ui.label(body(text));
        Ok(())
    }
}