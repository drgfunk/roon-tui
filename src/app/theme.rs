use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct ThemeColors {
    // You can either use a HashMap for dynamic keys
    #[serde(flatten)]
    colors: HashMap<String, String>,
    // Or define specific fields if you know them in advance
    // THEME_BG: String,
    // THEME_TITLE_FG: String,
    // etc.
}

impl ThemeColors {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let colors: ThemeColors = serde_yaml::from_str(&contents)?;
        Ok(colors)
    }

    pub fn get_color(&self, key: &str) -> Option<Color> {
        self.colors.get(key).and_then(|hex| parse_hex_color(hex))
    }
}

// Helper function to parse hex color strings into ratatui Color
fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');

    if hex.len() == 6 {
        // Parse RGB components
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Some(Color::Rgb(r, g, b));
        }
    }

    None
}
