use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        block::{self, Block, Position, Title},
        BorderType, Borders, Clear, Gauge, HighlightSpacing, List, ListItem, Padding, Paragraph,
    },
    Frame,
};
use roon_api::transport::{volume::Scale, Repeat, State, Zone};

use crate::{
    app::{App, View},
    io::EndPoint,
};

mod theme {
    use ratatui::style::Color;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[derive(Debug, Deserialize, Serialize)]
    pub struct ThemeColors {
        // You can either use a HashMap for dynamic keys
        #[serde(flatten)]
        pub colors: HashMap<String, String>,
        // Or define specific fields if you know them in advance
        // THEME_BG: String,
        // THEME_TITLE_FG: String,
        // etc.
    }

    impl ThemeColors {
        pub fn get_theme_path() -> PathBuf {
            println!("Searching for theme file...");

            // First check XDG_DATA_HOME/roon-tui/theme.yml
            if let Some(data_dir) = dirs::data_dir() {
                let xdg_path = data_dir.join("roon-tui").join("theme.yml");
                if xdg_path.exists() {
                    return xdg_path;
                }
            }

            // Then check ~/.config/roon-tui/theme.yml
            if let Some(config_dir) = dirs::config_dir() {
                let config_path = config_dir.join("roon-tui").join("theme.yml");
                if config_path.exists() {
                    return config_path;
                }
            }

            // Finally, check the current directory
            let local_path = PathBuf::from("theme.yml");
            if local_path.exists() {
                return local_path;
            }

            // Default to XDG_DATA_HOME path even if it doesn't exist yet
            if let Some(data_dir) = dirs::data_dir() {
                return data_dir.join("roon-tui").join("theme.yml");
            }

            // Last resort fallback
            PathBuf::from("theme.yml")
        }

        pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
            let contents = fs::read_to_string(path)?;
            let colors: ThemeColors = serde_yaml::from_str(&contents)?;
            Ok(colors)
        }

        pub fn load() -> Self {
            let path = Self::get_theme_path();
            Self::load_from_file(&path).unwrap_or_else(|_| {
                eprintln!("Failed to load theme from {:?}, using defaults", path);
                Self::default()
            })
        }

        // Create a new ThemeColors with default values
        pub fn default() -> Self {
            let mut colors = HashMap::new();
            colors.insert("THEME_BG".to_string(), Color::Reset.to_string());
            colors.insert("THEME_TITLE_FG".to_string(), "#9ccfd8".to_string());
            // Add other defaults as needed...

            ThemeColors { colors }
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
}

use self::theme::ThemeColors;

const ROON_BRAND_COLOR: Color = Color::Rgb(0x75, 0x75, 0xf3);
const CUSTOM_GRAY: Color = Color::Rgb(0x80, 0x80, 0x80);
const UNI_HIGHLIGHT_SYMBOL: &str = " \u{23f5} ";
const UNI_CHECKED_SYMBOL: &str = "\u{1F5F9}";
const UNI_UNCHECKED_SYMBOL: &str = "\u{2610}";
const HIGHLIGHT_SYMBOL: &str = " > ";
const CHECKED_SYMBOL: &str = "+";
const UNCHECKED_SYMBOL: &str = "-";

lazy_static::lazy_static! {
    static ref THEME: ThemeColors = ThemeColors::load();
}

// helper function to get color from theme, should take a key and return a color or a default
fn get_theme_color(key: &str, default: Color) -> Color {
    THEME.get_color(key).unwrap_or(default)
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let size = frame.size();

    // Surrounding block
    let title = format!(" Roon TUI v{} ", env!("CARGO_PKG_VERSION"));
    let subtitle = if let Some(name) = app.core_name.as_ref() {
        format!(" {} ", name)
    } else {
        app.select_view(None);
        " No Roon Server paired/found ".to_owned()
    };
    let hint = Title::from(Span::styled(
        " ? for Help ",
        Style::default().fg(get_theme_color("THEME_HINT_FG", CUSTOM_GRAY)),
    ))
    .position(Position::Bottom)
    .alignment(Alignment::Center);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(get_border_view_style(app, None, "THEME_BORDER", ""))
        .style(Style::default().bg(get_theme_color("THEME_BG", Color::Reset))) // Apply background color to main block;
        .title(Span::styled(title, get_text_view_style(app, None)))
        .title(Span::styled(subtitle, get_text_view_style(app, None)))
        .title(hint)
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Plain);

    frame.render_widget(block, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .horizontal_margin(2)
        .vertical_margin(1)
        .constraints([Constraint::Min(8), Constraint::Length(7)].as_ref())
        .split(size);

    // Top two inner blocks
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[0]);

    draw_browse_view(frame, top_chunks[0], app);
    draw_queue_view(frame, top_chunks[1], app);
    draw_now_playing_view(frame, chunks[1], app);

    match app.selected_view {
        Some(View::Prompt) => draw_prompt_view(frame, top_chunks[0], app),
        Some(View::Zones) => draw_zones_view(frame, top_chunks[1], app),
        Some(View::Grouping) | Some(View::GroupingPreset) => {
            draw_grouping_view(frame, top_chunks[1], app);
        }
        Some(View::Help) => draw_help_view(frame, size, app),
        _ => (),
    }
}

fn draw_browse_view(frame: &mut Frame, area: Rect, app: &mut App) {
    let browse_title = format!(" {} ", app.browse.title.as_deref().unwrap_or("Browse"));
    let page_lines = area.height.saturating_sub(2) as usize; // Exclude border
    let view = Some(&View::Browse);
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(get_border_view_style(
            &app,
            view,
            "THEME_BROWSE_BORDER",
            "THEME_BROWSE_BORDER_SELECTED",
        ))
        .title(Span::styled(browse_title, get_browse_title_style(&app)));
    // .style(Style::default().bg(get_theme_color("THEME_BROWSE_BG", Color::Reset))); // Apply background color to main block;

    app.browse.prepare_paging(
        page_lines,
        |item| if item.subtitle.is_none() { 1 } else { 2 },
    );

    if let Some(browse_items) = &app.browse.items {
        let secondary_style = if app.get_selected_view().is_some() {
            Style::default().add_modifier(Modifier::ITALIC)
        } else {
            Style::default()
                .fg(CUSTOM_GRAY)
                .add_modifier(Modifier::ITALIC)
        };
        let items: Vec<ListItem> = browse_items
            .iter()
            .map(|item| {
                let subtitle = item.subtitle.as_ref().filter(|s| !s.is_empty());
                let mut lines = vec![Line::from(Span::styled(
                    &item.title,
                    get_text_view_style(&app, view),
                ))];

                if let Some(subtitle) = subtitle {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", subtitle),
                        secondary_style,
                    )));
                }

                ListItem::new(lines)
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let highlight_symbol = if app.no_unicode_symbols {
            HIGHLIGHT_SYMBOL
        } else {
            UNI_HIGHLIGHT_SYMBOL
        };
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(get_theme_color("THEME_HIGHLIGHT", ROON_BRAND_COLOR))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(highlight_symbol)
            .highlight_spacing(HighlightSpacing::Always);

        // We can now render the item list
        frame.render_stateful_widget(list, area, &mut app.browse.state);

        if let Some(View::Browse) = app.selected_view.as_ref() {
            let len = browse_items.len();

            if len > 0 {
                let progress = format!(" {}/{} ", app.browse.state.selected().unwrap() + 1, len);

                block = block.title(
                    Title::from(Span::styled(
                        progress,
                        Style::default()
                            .fg(get_theme_color("THEME_BROWSE_BORDER_SELECTED", CUSTOM_GRAY)),
                    )) // meta
                    .alignment(Alignment::Right),
                );

                if !app.input.is_empty() {
                    block = block.title(
                        Title::from(Span::styled(
                            app.input.as_str(),
                            Style::default().fg(Color::Reset),
                        ))
                        .position(Position::Bottom),
                    );
                }
            }
        }
    }

    frame.render_widget(block, area);
}

fn draw_queue_view(frame: &mut Frame, area: Rect, app: &mut App) {
    // Define a background color for the now playing view
    let background_color = get_theme_color("THEME_QUEUE_BG", Color::Reset); // Dark blue-purple background

    let page_lines = area.height.saturating_sub(2) as usize; // Exclude border
    let view = Some(&View::Queue);
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(get_border_view_style(
            &app,
            view,
            "THEME_QUEUE_BORDER",
            "THEME_QUEUE_BORDER_SELECTED",
        ))
        .title(Span::styled(" Queue ", get_queue_title_style(&app)))
        .title_alignment(Alignment::Right)
        .style(Style::default().bg(background_color)); // Apply background color to main block;

    if let Some(queue_mode) = app.queue_mode {
        block = block.title(
            Title::from(Span::styled(
                queue_mode,
                Style::default().fg(get_theme_color("THEME_TEXT_FG", CUSTOM_GRAY)),
            ))
            .position(Position::Bottom),
        );
    }

    app.queue.prepare_paging(page_lines, |item| {
        if item.two_line.line2.is_empty() {
            1
        } else {
            2
        }
    });

    if let Some(queue_items) = &app.queue.items {
        let item_len = area.width.saturating_sub(6) as usize;
        let secondary_style = if app.get_selected_view() == view {
            Style::default()
                .fg(get_theme_color("THEME_TEXT_FG_SELECTED", Color::Reset))
                .add_modifier(Modifier::ITALIC)
        } else {
            Style::default()
                .fg(get_theme_color("THEME_TEXT_FG", CUSTOM_GRAY))
                .add_modifier(Modifier::ITALIC)
        };
        let items: Vec<ListItem> = queue_items
            .iter()
            .map(|item| {
                let duration = get_time_string(item.length);
                let max_len = item_len.saturating_sub(duration.len() + 1);
                let (line1_len, line1) = trim_string(&item.two_line.line1, max_len);
                let pad_len = item_len.saturating_sub(line1_len + duration.len());
                let pad: String = (0..pad_len).map(|_| ' ').collect();
                let line1 = format!("{}{}{}", line1, pad, duration);
                let mut lines = vec![Line::from(Span::styled(
                    line1,
                    get_text_view_style(&app, view),
                ))];

                if !item.two_line.line2.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", item.two_line.line2),
                        secondary_style,
                    )));
                }

                ListItem::new(lines)
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let highlight_symbol = if app.no_unicode_symbols {
            HIGHLIGHT_SYMBOL
        } else {
            UNI_HIGHLIGHT_SYMBOL
        };
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::new()
                    .bg(ROON_BRAND_COLOR)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(highlight_symbol)
            .highlight_spacing(HighlightSpacing::Always);

        // We can now render the item list
        frame.render_stateful_widget(list, area, &mut app.queue.state);

        if let Some(View::Queue) = app.selected_view.as_ref() {
            let len = queue_items.len();

            if len > 0 {
                let progress = format!(" {}/{} ", app.queue.state.selected().unwrap() + 1, len);

                block = block.title(
                    Title::from(Span::styled(
                        progress,
                        Style::default()
                            .fg(get_theme_color("THEME_QUEUE_BORDER_SELECTED", CUSTOM_GRAY)),
                    ))
                    .alignment(Alignment::Left),
                );
            }
        } else {
            if let Some(queue_time_remaining) = get_queue_time_remaining(&app) {
                block = block.title(
                    Title::from(Span::styled(
                        queue_time_remaining,
                        Style::default().fg(get_theme_color("THEME_SUBTITLE_FG", CUSTOM_GRAY)),
                    ))
                    .alignment(Alignment::Left),
                );
            }
        }
    }

    frame.render_widget(block, area);
}

fn draw_now_playing_view(frame: &mut Frame, area: Rect, app: &App) {
    let view = Some(&View::NowPlaying);
    let background_color = get_theme_color("THEME_NOW_PLAYING_BG", Color::Reset); // Dark blue-purple background

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(get_border_view_style(
            app,
            view,
            "THEME_NOW_PLAYING_BORDER",
            "THEME_NOW_PLAYING_BORDER_SELECTED",
        ))
        .title_position(block::Position::Top)
        .padding(Padding {
            left: 1,
            right: 0,
            top: 0,
            bottom: 0,
        });

    // Clear with the background color
    frame.render_widget(Clear, area);

    if let Some(zone) = app.selected_zone.as_ref() {
        let vert_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(2)].as_ref())
            .split(area);
        let hor_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(20), Constraint::Length(14)].as_ref())
            .split(vert_chunks[0]);

        // Adjust text style to ensure readability on the new background
        let style = if app.get_selected_view() == view {
            Style::default().fg(get_theme_color("THEME_TEXT_FG_SELECTED", Color::Reset))
        } else {
            Style::default().fg(get_theme_color("THEME_TEXT_FG", CUSTOM_GRAY))
        };

        let display_name = match app.matched_preset.as_ref() {
            Some(preset) => format!("{} ({})", preset.as_str(), zone.display_name),
            None => format!(" {} ", zone.display_name.to_owned()),
        };

        // create a variable for the theme key, if this view is selected use THEME_NOW_PLAYING_TITLE_FG
        // else use THEME_TEXT_FG
        let title_fg = if is_selected_view(app, view) {
            "THEME_NOW_PLAYING_BORDER_SELECTED"
        } else {
            "THEME_TEXT_FG"
        };

        block = block.title(
            Title::from(Span::styled(
                display_name,
                get_text_view_style(app, view).fg(get_theme_color(title_fg, CUSTOM_GRAY)),
            ))
            .alignment(Alignment::Right),
        );

        if let Some(now_playing) = zone.now_playing.as_ref() {
            let metadata_block = Block::default()
                .style(Style::default().bg(background_color))
                .padding(Padding {
                    left: 4,
                    right: 0,
                    top: 1,
                    bottom: 0,
                });
            let lines = vec![
                Line::from(Span::styled(
                    &now_playing.three_line.line1,
                    style.add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(&now_playing.three_line.line2, style)),
                Line::from(Span::styled(
                    &now_playing.three_line.line3,
                    style.add_modifier(Modifier::ITALIC),
                )),
            ];
            let text = Paragraph::new(lines)
                .block(metadata_block)
                .style(Style::default());

            frame.render_widget(text, hor_chunks[0]);

            let duration = now_playing.length.unwrap_or_default();
            let seek_position = if let Some(zone_seek) = app.zone_seek.as_ref() {
                if zone_seek.seek_position.is_some() {
                    zone_seek.seek_position
                } else {
                    now_playing.seek_position
                }
            } else {
                now_playing.seek_position
            };

            draw_progress_gauge(frame, vert_chunks[1], app, view, duration, seek_position);

            let play_state_title = match zone.state {
                State::Loading => "Loading",
                State::Paused => "Paused",
                State::Playing => {
                    if app.pause_on_track_end {
                        "Pause at End of Track"
                    } else {
                        "Playing"
                    }
                }
                State::Stopped => "Stopped",
            };

            block = block.title(Span::styled(
                format!(" {} ", play_state_title),
                get_playing_title_style(app),
            ));
        } else if app.core_name.is_some() {
            let msg_block = Block::default()
                .style(Style::default().bg(background_color))
                .padding(Padding {
                    left: 0,
                    right: 0,
                    top: 1,
                    bottom: 0,
                });
            let text = Paragraph::new("Go find something to play!")
                .block(msg_block)
                .style(Style::default().bg(background_color))
                .alignment(Alignment::Center);

            frame.render_widget(text, hor_chunks[0]);
        }

        let status_block = Block::default()
            .style(Style::default().bg(background_color))
            .padding(Padding {
                left: 1,
                right: 2,
                top: 1,
                bottom: 0,
            });

        // Get status lines with background color applied
        let status_lines = get_status_lines(zone, style);

        let text = Paragraph::new(status_lines)
            .block(status_block)
            .style(Style::default().bg(background_color))
            .alignment(Alignment::Right);

        frame.render_widget(text, hor_chunks[1]);
    } else {
        let msg_block = Block::default()
            .style(Style::default().bg(background_color))
            .padding(Padding {
                left: 0,
                right: 0,
                top: 1,
                bottom: 0,
            });
        let msg = if app.core_name.is_some() {
            "No zone selected, use Ctrl-z to select one"
        } else {
            "Not paired to a Roon Server (or no server found)\n\
            Use a Roon Remote and go to Settings->Extensions to enable Roon TUI"
        };
        let text = Paragraph::new(msg)
            .block(msg_block)
            .style(Style::default().bg(background_color))
            .alignment(Alignment::Center);

        frame.render_widget(text, area);
    }

    frame.render_widget(block, area);
}

fn draw_progress_gauge(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    view: Option<&View>,
    duration: u32,
    seek_position: Option<i64>,
) -> Option<()> {
    let elapsed = seek_position? as u32;
    let progress = if duration > 0 {
        elapsed * 100 / duration
    } else {
        0
    };
    let elapsed = get_time_string(elapsed);
    let label = if duration > 0 {
        format!("{} / {}", elapsed, get_time_string(duration))
    } else {
        elapsed
    };

    // if selected view is now_playing, use the selected view style
    let style = if app.get_selected_view() == view {
        Style::default().fg(get_theme_color(
            "THEME_GAUGE_LABEL_FG_SELECTED",
            Color::Reset,
        ))
    } else {
        Style::default().fg(get_theme_color("THEME_GAUGE_LABEL_FG", CUSTOM_GRAY))
    };

    let gauge = Gauge::default()
        .block(Block::default().style(Style::default()).padding(Padding {
            left: 2,
            right: 2,
            top: 0,
            bottom: 1,
        }))
        .style(Style::default().bg(get_theme_color("THEME_NOW_PLAYING_BG", Color::Reset)))
        .gauge_style(get_gauge_view_style(app, view))
        .percent(progress as u16)
        .label(Span::styled(label, style.add_modifier(Modifier::BOLD)));

    frame.render_widget(gauge, area);

    Some(())
}

fn get_time_string(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

fn get_queue_time_remaining(app: &App) -> Option<String> {
    let zone = app.selected_zone.as_ref()?;
    let now_playing = zone.now_playing.as_ref()?;
    let queue_time_remaining = match app.zone_seek.as_ref() {
        Some(zone_seek) => zone_seek.queue_time_remaining,
        None => zone.queue_time_remaining,
    };

    if queue_time_remaining > 0 && now_playing.length.is_some() {
        Some(format!(
            " {} ",
            get_time_string(queue_time_remaining as u32)
        ))
    } else {
        None
    }
}

fn trim_string(string: &str, trim_len: usize) -> (usize, &str) {
    let trim = match string.char_indices().nth(trim_len) {
        None => string,
        Some((index, _)) => &string[..index],
    };

    (trim.chars().count(), trim)
}

fn get_status_lines(zone: &Zone, style: Style) -> Vec<Line> {
    let volume = if let Some(output) = zone.outputs.get(0) {
        if let Some(volume) = output.volume.as_ref() {
            match volume.scale {
                Scale::Incremental => "Vol Incrmnt".to_owned(),
                _ => {
                    let is_muted = volume.is_muted.unwrap_or_default();

                    if is_muted {
                        "Vol   Muted".to_owned()
                    } else {
                        let volume_level = volume.value.unwrap();

                        match volume.scale {
                            Scale::Decibel => {
                                if volume.step.unwrap() < 1.0 {
                                    format!("Vol {:5.1}dB", volume_level)
                                } else {
                                    format!("Vol {:5}dB", volume_level)
                                }
                            }
                            Scale::Number => format!("Vol {:7}", volume_level),
                            _ => String::new(),
                        }
                    }
                }
            }
        } else {
            "Vol   Fixed".to_owned()
        }
    } else {
        String::new()
    };
    let settings = &zone.settings;
    let repeat_icon = match settings.repeat {
        Repeat::All => "Repeat  All",
        Repeat::One => "Repeat  One",
        _ => "Repeat  Off",
    };

    vec![
        Line::from(Span::styled(volume, style)),
        Line::from(Span::styled(format!("{}", repeat_icon), style)),
        Line::from(Span::styled(
            format!(
                "{}",
                if settings.shuffle {
                    "Shuffle  On"
                } else {
                    "Shuffle Off"
                }
            ),
            style,
        )),
    ]
}

fn draw_prompt_view(frame: &mut Frame, area: Rect, app: &mut App) {
    let view = Some(&View::Prompt);
    let area = upper_bar(area);
    let max_len = area.width.saturating_sub(3) as usize;
    app.set_max_input_len(max_len);

    let prompt = app.prompt.as_str();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(get_border_view_style(&app, view, "", ""))
        .title(Span::styled(prompt, get_text_view_style(&app, view)))
        .title_alignment(Alignment::Left);

    frame.render_widget(Clear, area); // This clears out the background

    let input = Line::from(Span::styled(
        app.input.as_str(),
        Style::default().fg(Color::Reset),
    ));
    let input = Paragraph::new(input)
        .style(Style::default().fg(ROON_BRAND_COLOR))
        .block(block);

    frame.render_widget(input, area);

    // Make the cursor visible and ask ratatui to put it at the specified coordinates after
    // rendering
    frame.set_cursor(
        // Draw the cursor at the current position in the input field.
        // This position can be controlled via the left and right arrow key
        area.x + app.cursor_position.clamp(0, max_len) as u16 + 1,
        // Move one line down, from the border to the input line
        area.y + 1,
    );
}

fn draw_zones_view(frame: &mut Frame, area: Rect, app: &mut App) {
    let view = Some(&View::Zones);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(get_border_view_style(&app, view, "", "THEME_ZONES_BORDER"))
        .title(Span::styled(
            " Zones ",
            get_text_view_style(&app, view)
                .fg(get_theme_color("THEME_ZONES_TITLE_FG", CUSTOM_GRAY))
                .bg(get_theme_color("THEME_ZONES_TITLE_BG", Color::Reset)),
        ))
        .title_alignment(Alignment::Left);

    let area = bottom_right_rect(50, 50, area);
    let page_lines = area.height.saturating_sub(2) as usize; // Exclude border

    // Define a background color for the zones view
    let background_color = get_theme_color("THEME_ZONES_BG", Color::Reset);

    frame.render_widget(Clear, area); // This clears out the background

    // Create a background block with the desired color
    let background = Block::default().style(Style::default().bg(background_color));
    frame.render_widget(background, area);

    app.zones.prepare_paging(page_lines, |_| 1);

    if let Some(zones) = app.zones.items.as_ref() {
        let items: Vec<ListItem> = zones
            .iter()
            .map(|(end_point, name)| {
                let name = match end_point {
                    EndPoint::Preset(_) => format!("[{}]", name),
                    EndPoint::Output(_) => format!("<{}>", name),
                    EndPoint::Zone(_) => name.to_owned(),
                };
                let line = Span::styled(name, get_text_view_style(&app, view));
                ListItem::new(Line::from(line)).style(Style::default().bg(background_color))
                // Apply background to each item
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let highlight_symbol = if app.no_unicode_symbols {
            HIGHLIGHT_SYMBOL
        } else {
            UNI_HIGHLIGHT_SYMBOL
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().bg(background_color)),
            )
            .highlight_style(
                Style::default()
                    .bg(get_theme_color("THEME_ZONES_HIGHLIGHT", ROON_BRAND_COLOR))
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(background_color)) // Apply background to the list itself
            .highlight_symbol(highlight_symbol);

        // We can now render the item list
        frame.render_stateful_widget(list, area, &mut app.zones.state);
    }

    frame.render_widget(block, area);
}

fn draw_grouping_view(frame: &mut Frame, area: Rect, app: &mut App) -> Option<()> {
    let view = if app.selected_view == Some(View::GroupingPreset) {
        View::GroupingPreset
    } else {
        View::Grouping
    };
    let background_color = get_theme_color("THEME_GROUP_BG", Color::Reset);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(get_border_view_style(
            app,
            Some(&view),
            "",
            "THEME_ZONES_BORDER",
        ))
        .title(Span::styled(
            " Grouping ",
            get_text_view_style(&app, None)
                .fg(get_theme_color("THEME_ZONES_TITLE_FG", CUSTOM_GRAY))
                .bg(get_theme_color("THEME_ZONES_TITLE_BG", Color::Reset)),
        ))
        .title_alignment(Alignment::Left);
    // .style(Style::default().bg(background_color));

    let area = bottom_right_rect(50, 50, area);
    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Min(5)].as_ref())
        .horizontal_margin(1)
        .split(area);

    let list_area = Rect::new(
        vchunks[1].x,
        vchunks[1].y,
        vchunks[1].width,
        vchunks[1].height.saturating_sub(1),
    );

    frame.render_widget(Clear, area); // This clears out the background

    let background = Block::default().style(Style::default().bg(background_color));
    frame.render_widget(background, area);

    if view == View::GroupingPreset {
        let max_len = vchunks[0].width.saturating_sub(1) as usize;
        app.set_max_input_len(max_len);

        let input = vec![
            Line::from(""), // Hidden underneath border
            Line::from(Span::styled(
                app.input.as_str(),
                Style::default()
                    .fg(Color::Reset)
                    .add_modifier(Modifier::BOLD),
            )),
        ];
        let input = Paragraph::new(input).style(Style::default().fg(ROON_BRAND_COLOR));

        frame.render_widget(input, vchunks[0]);

        // Make the cursor visible and ask ratatui to put it at the specified coordinates after
        // rendering
        frame.set_cursor(
            // Draw the cursor at the current position in the input field.
            // This position can be controlled via the left and right arrow key
            vchunks[0].x + app.cursor_position.clamp(0, max_len) as u16,
            // Move one line down, from the border to the input line
            vchunks[0].y + 1,
        );
    } else {
        let ouput_ids = app.get_included_output_ids(app.grouping.items.as_ref()?);
        let zone_name = if ouput_ids.len() <= 1 {
            app.selected_zone.as_ref()?.display_name.as_str()
        } else if !app.input.is_empty() {
            app.input.as_str()
        } else if app.matched_draft_preset.is_some() {
            app.matched_draft_preset.as_deref()?
        } else {
            app.selected_zone.as_ref()?.display_name.as_str()
        };
        let zone_name = vec![
            Line::from(""), // Hidden underneath border
            Line::from(Span::styled(
                zone_name,
                Style::default()
                    .fg(Color::Reset)
                    .add_modifier(Modifier::BOLD),
            )),
        ];
        let page_lines = list_area.height as usize;

        app.grouping.prepare_paging(page_lines, |_| 1);

        frame.render_widget(Paragraph::new(zone_name), vchunks[0]);
    }

    let grouping = app.grouping.items.as_ref()?;
    let checked_symbol = if app.no_unicode_symbols {
        CHECKED_SYMBOL
    } else {
        UNI_CHECKED_SYMBOL
    };
    let unchecked_symbol = if app.no_unicode_symbols {
        UNCHECKED_SYMBOL
    } else {
        UNI_UNCHECKED_SYMBOL
    };
    let items: Vec<ListItem> = grouping
        .iter()
        .map(|(_, name, included)| {
            let state = if *included {
                checked_symbol
            } else {
                unchecked_symbol
            };
            let line = Span::styled(
                format!("{}  {}", state, name),
                get_text_view_style(&app, Some(&View::Grouping)),
            );

            ListItem::new(Line::from(line)).style(Style::default())
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let list = List::new(items).block(Block::default()).highlight_style(
        Style::default()
            .bg(get_theme_color("THEME_ZONES_HIGHLIGHT", ROON_BRAND_COLOR))
            .add_modifier(Modifier::BOLD),
    );

    // We can now render the widgets
    frame.render_stateful_widget(list, list_area, &mut app.grouping.state);
    frame.render_widget(block, area);

    Some(())
}

fn draw_help_view(frame: &mut Frame, area: Rect, app: &mut App) {
    let view = Some(&View::Help);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(get_border_view_style(
            &app,
            view,
            "THEME_HELP_BORDER",
            "THEME_HELP_BORDER",
        ))
        .style(Style::default().bg(get_theme_color("THEME_HELP_BG", Color::Reset))) // Apply background color to main block;
        .title(Span::styled(
            " Help ",
            get_text_view_style(&app, view)
                .fg(get_theme_color("THEME_HELP_TITLE_FG", CUSTOM_GRAY))
                .bg(get_theme_color("THEME_HELP_TITLE_BG", Color::Reset)),
        ))
        .title_alignment(Alignment::Left);
    let chunk = Layout::default()
        .direction(Direction::Horizontal)
        .horizontal_margin(2)
        .vertical_margin(1)
        .constraints([Constraint::Percentage(100)])
        .split(area);
    let hor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .horizontal_margin(2)
        .constraints(
            [
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ]
            .as_ref(),
        )
        .split(chunk[0]);
    let max_entries: usize = (hor_chunks[0].height as usize).saturating_sub(2);
    let text = [
        "__Global__",
        "Tab     Next view",
        "Sh-Tab  Previous view",
        "Ctrl-z  Select zone",
        "Ctrl-g  Group zones",
        "Ctrl-Sp Play/Pause",
        "Ctrl-p  Play/Pause",
        "Ctrl-e  Pause at end",
        "Ctrl-Up Volume up",
        "Ctrl-Dn Volume down",
        "Ctrl-Ri Next track",
        "Ctrl-Le Previous track",
        "Ctrl-q  Queue mode",
        "Ctrl-a  Append queue",
        "Ctrl-h  This help page",
        "Ctrl-c  Quit",
        "",
        "__List Controls__",
        "Up      Move up",
        "Down    Move down",
        "Home    Move to top",
        "End     Move to bottom",
        "Page-Up Move page up",
        "Page-Dn Move page down",
        "",
        "__Browse View__",
        "Enter   Select",
        "Esc     Move level up",
        "Ctrl-Hm Browse home",
        "F5      Refresh",
        "a..z    Char jump",
        "Backsp  Prev char jump",
        "",
        "__Queue View__",
        "Enter   Play from here",
        "",
        "__Now Playing View__",
        "m       Mute",
        "u       Unmute",
        "+       Volume up",
        "-       Volume down",
        "r       Toggle Repeat",
        "s       Toggle shuffle",
        "",
        "__Zone Select Popup__",
        "Enter   Select zone",
        "Esc     Back to view",
        "Delete  Delete preset",
        "",
        "__Zone Grouping Popup__",
        "Space   Toggle output",
        "Enter   Activate group",
        "s       Save as preset",
        "Esc     Back to view",
        "",
        "__Text Input__",
        "Enter   Confirm input",
        "Esc     Cancel input",
    ];

    frame.render_widget(Clear, chunk[0]); // This clears out the background

    for column in 0..hor_chunks.len() {
        let start = column * max_entries;
        let end = (start + max_entries).clamp(start, text.len());

        frame.render_widget(create_paragraph(&text[start..end]), hor_chunks[column]);

        if end == text.len() {
            break;
        }
    }

    frame.render_widget(block, chunk[0]);
}

fn create_paragraph<'a>(text: &'a [&str]) -> Paragraph<'a> {
    let block = Block::default().padding(Padding {
        left: 1,
        right: 1,
        top: 1,
        bottom: 0,
    });
    let style = Style::default().fg(get_theme_color("THEME_TEXT_FG_SELECTED", Color::Reset));
    let mut lines = Vec::new();

    for line in text {
        if line.starts_with("__") && line.ends_with("__") {
            let bold_line = &line[2..line.len().saturating_sub(2)];
            let bold_style = style.add_modifier(Modifier::BOLD);

            lines.push(Line::from(Span::styled(bold_line, bold_style)))
        } else {
            lines.push(Line::from(Span::styled(*line, style)))
        }
    }

    Paragraph::new(lines).block(block)
}

fn get_border_view_style(
    app: &App,
    view: Option<&View>,
    default_theme_key: &str,
    selected_theme_key: &str,
) -> Style {
    let mut style = Style::default();

    // set default theme color if default_theme_key is provided
    // otherwise use CUSTOM_GRAY
    let border_color = if default_theme_key.is_empty() {
        CUSTOM_GRAY
    } else {
        get_theme_color(default_theme_key, CUSTOM_GRAY)
    };

    let border_selected_color = if selected_theme_key.is_empty() {
        ROON_BRAND_COLOR
    } else {
        get_theme_color(selected_theme_key, ROON_BRAND_COLOR)
    };

    if let Some(selected_view) = app.get_selected_view() {
        if let Some(view) = view {
            if *selected_view == *view {
                style = style.fg(border_selected_color);
            } else {
                style = style.fg(border_color);
            }
        }
    } else if view.is_none() {
        style = style.fg(border_color);
    } else {
        style = style.fg(border_color);
    }

    if view.is_none() {
        style = style.fg(border_color);
    }

    style
}

fn is_selected_view(app: &App, view: Option<&View>) -> bool {
    if let Some(selected_view) = app.get_selected_view() {
        if let Some(view) = view {
            return *selected_view == *view;
        }
    }

    false
}

fn get_browse_title_style(app: &App) -> Style {
    let mut style = Style::default();

    if is_selected_view(&app, Some(&View::Browse)) {
        style = style
            .fg(get_theme_color(
                "THEME_BROWSE_TITLE_FG_SELECTED",
                Color::Reset,
            ))
            .bg(get_theme_color(
                "THEME_BROWSE_TITLE_BG_SELECTED",
                Color::Reset,
            ))
            .add_modifier(Modifier::BOLD);
    } else {
        style = style
            .fg(get_theme_color("THEME_BROWSE_TITLE_FG", CUSTOM_GRAY))
            .bg(get_theme_color("THEME_BROWSE_TITLE_BG", Color::Reset));
    }

    style
}

fn get_queue_title_style(app: &App) -> Style {
    let mut style = Style::default();

    if is_selected_view(&app, Some(&View::Queue)) {
        style = style
            .fg(get_theme_color(
                "THEME_QUEUE_TITLE_FG_SELECTED",
                Color::Reset,
            ))
            .bg(get_theme_color(
                "THEME_QUEUE_TITLE_BG_SELECTED",
                Color::Reset,
            ))
            .add_modifier(Modifier::BOLD);
    } else {
        style = style
            .fg(get_theme_color("THEME_QUEUE_TITLE_FG", CUSTOM_GRAY))
            .bg(get_theme_color("THEME_QUEUE_TITLE_BG", Color::Reset));
    }

    style
}

fn get_playing_title_style(app: &App) -> Style {
    let mut style = Style::default();

    if is_selected_view(&app, Some(&View::NowPlaying)) {
        style = style
            .fg(get_theme_color(
                "THEME_NOW_PLAYING_TITLE_FG_SELECTED",
                Color::Reset,
            ))
            .bg(get_theme_color(
                "THEME_NOW_PLAYING_TITLE_BG_SELECTED",
                Color::Reset,
            ))
            .add_modifier(Modifier::BOLD);
    } else {
        style = style
            .fg(get_theme_color("THEME_NOW_PLAYING_TITLE_FG", CUSTOM_GRAY))
            .bg(get_theme_color("THEME_NOW_PLAYING_TITLE_BG", Color::Reset));
    }

    style
}

fn get_text_view_style(app: &App, view: Option<&View>) -> Style {
    let mut style = Style::default().fg(get_theme_color("THEME_TEXT_FG", CUSTOM_GRAY));

    if let Some(selected_view) = app.get_selected_view() {
        if let Some(view) = view {
            if *selected_view == *view {
                // style = style.fg(Color::Reset).add_modifier(Modifier::BOLD);
                style = style
                    .fg(get_theme_color("THEME_TEXT_FG_SELECTED", Color::Reset))
                    .add_modifier(Modifier::BOLD);
            }
        }
    } else if view.is_none() {
        style = style
            .fg(get_theme_color("THEME_TEXT_FG", CUSTOM_GRAY))
            .add_modifier(Modifier::BOLD);
    } else {
        style = style.fg(get_theme_color("THEME_TEXT_FG", CUSTOM_GRAY));
    }

    if view.is_none() {
        style = style
            .fg(get_theme_color("THEME_TITLE_FG", CUSTOM_GRAY))
            .bg(get_theme_color("THEME_TITLE_BG", Color::Reset));
    }

    style
}

fn get_gauge_view_style(app: &App, view: Option<&View>) -> Style {
    let mut style = Style::default().bg(get_theme_color(
        "THEME_GAUGE_BG",
        Color::Rgb(0x30, 0x30, 0x30),
    ));

    if let Some(selected_view) = app.get_selected_view() {
        if let Some(view) = view {
            if *selected_view == *view {
                style = style.fg(get_theme_color("THEME_GAUGE_FG_SELECTED", ROON_BRAND_COLOR));
            } else {
                style = style.fg(get_theme_color("THEME_GAUGE_FG", CUSTOM_GRAY));
            }
        }
    } else if view.is_some() {
        style = style.fg(Color::Rgb(0x30, 0x30, 0x30));
    }

    style
}

fn upper_bar(rect: Rect) -> Rect {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)].as_ref())
        .split(rect)[0]
}

fn bottom_right_rect(percent_x: u16, percent_y: u16, rect: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(100 - percent_y),
                Constraint::Percentage(percent_y),
            ]
            .as_ref(),
        )
        .split(rect);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(100 - percent_x),
                Constraint::Percentage(percent_x),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
