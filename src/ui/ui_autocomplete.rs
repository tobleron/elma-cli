//! @efficiency-role: ui-component
//!
//! Slash command and emoji autocomplete dropdowns.
//!
//! Renders a list of matching commands/emojis as styled lines
//! positioned above the input line in the footer.

use crate::ui_colors::*;
use crate::ui_theme::*;
use crate::ui_wrap::display_width;

/// A single autocomplete suggestion.
#[derive(Clone, Debug)]
pub(crate) struct AutocompleteSuggestion {
    /// The command/emoji text to insert (e.g. "/help", ":smile:").
    pub label: String,
    /// A short description shown next to the label.
    pub description: String,
}

/// State for the autocomplete dropdown.
#[derive(Clone, Debug, Default)]
pub(crate) struct AutocompleteState {
    /// Whether the dropdown is currently visible.
    pub active: bool,
    /// The prefix being matched (e.g. "/he" or ":sm").
    pub prefix: String,
    /// The filtered list of matching suggestions.
    pub matches: Vec<AutocompleteSuggestion>,
    /// Index of the currently selected suggestion.
    pub selected: usize,
    /// Whether this is an emoji picker (true) or slash commands (false).
    pub is_emoji: bool,
}

impl AutocompleteState {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Check if input starts a slash command prefix and update matches.
    pub(crate) fn update_slash(&mut self, input: &str) {
        if input.starts_with('/') {
            self.active = true;
            self.is_emoji = false;
            self.prefix = input.to_string();
            self.matches = filter_slash_commands(input);
            self.selected = 0;
        } else {
            self.active = false;
        }
    }

    /// Check if input starts an emoji prefix and update matches.
    pub(crate) fn update_emoji(&mut self, input: &str) {
        if input.starts_with(':') {
            self.active = true;
            self.is_emoji = true;
            self.prefix = input.to_string();
            self.matches = filter_emojis(input);
            self.selected = 0;
        } else {
            self.active = false;
        }
    }

    /// Deactivate the dropdown.
    pub(crate) fn deactivate(&mut self) {
        self.active = false;
    }

    /// Move selection down.
    pub(crate) fn select_down(&mut self) {
        if self.selected + 1 < self.matches.len() {
            self.selected += 1;
        }
    }

    /// Move selection up.
    pub(crate) fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Get the selected suggestion's label, if any.
    pub(crate) fn selected_label(&self) -> Option<String> {
        self.matches.get(self.selected).map(|s| s.label.clone())
    }
}

// ============================================================================
// Built-in slash commands
// ============================================================================

const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/help", "Show help screen"),
    ("/models", "Switch model/provider"),
    ("/usage", "Token and cost stats"),
    ("/sessions", "Session manager"),
    ("/approve", "Tool approval policy"),
    ("/compact", "Compact context now"),
    ("/reset", "Clear history and reset state"),
    ("/snapshot", "Create workspace snapshot"),
    ("/tune", "Run model tuning"),
    ("/goals", "Show current goal state"),
    ("/reset-goals", "Clear goals"),
    ("/tools", "Discover available tools"),
    ("/verbose", "Toggle verbose mode"),
    ("/reasoning", "Toggle reasoning visibility"),
    ("/exit", "Exit Elma"),
    ("/quit", "Exit Elma"),
];

fn filter_slash_commands(prefix: &str) -> Vec<AutocompleteSuggestion> {
    let prefix_lower = prefix.to_lowercase();
    SLASH_COMMANDS
        .iter()
        .filter(|(cmd, _)| cmd.to_lowercase().starts_with(&prefix_lower))
        .map(|(cmd, desc)| AutocompleteSuggestion {
            label: cmd.to_string(),
            description: desc.to_string(),
        })
        .collect()
}

// ============================================================================
// Emoji shortcodes (subset of most common ~200)
// ============================================================================

const EMOJIS: &[(&str, &str)] = &[
    (":smile:", "😄"),
    (":laughing:", "😆"),
    (":blush:", "😊"),
    (":smiley:", "😃"),
    (":heart:", "❤️"),
    (":thumbsup:", "👍"),
    (":thumbsdown:", "👎"),
    (":fire:", "🔥"),
    (":star:", "⭐"),
    (":rocket:", "🚀"),
    (":check:", "✅"),
    (":cross:", "❌"),
    (":warning:", "⚠️"),
    (":bulb:", "💡"),
    (":book:", "📖"),
    (":memo:", "📝"),
    (":eyes:", "👀"),
    (":wave:", "👋"),
    (":clap:", "👏"),
    (":pray:", "🙏"),
    (":thinking:", "🤔"),
    (":ok_hand:", "👌"),
    (":point_up:", "☝️"),
    (":point_down:", "👇"),
    (":tada:", "🎉"),
    (":gift:", "🎁"),
    (":birthday:", "🎂"),
    (":coffee:", "☕"),
    (":beer:", "🍺"),
    (":pizza:", "🍕"),
    (":apple:", "🍎"),
    (":sun:", "☀️"),
    (":moon:", "🌙"),
    (":cloud:", "☁️"),
    (":rainbow:", "🌈"),
    (":snowflake:", "❄️"),
    (":dog:", "🐶"),
    (":cat:", "🐱"),
    (":bird:", "🐦"),
    (":fish:", "🐟"),
    (":bug:", "🐛"),
    (":100:", "💯"),
    (":zap:", "⚡"),
    (":boom:", "💥"),
    (":trophy:", "🏆"),
    (":medal:", "🏅"),
    (":dart:", "🎯"),
    (":key:", "🔑"),
    (":lock:", "🔒"),
    (":unlock:", "🔓"),
    (":hammer:", "🔨"),
    (":wrench:", "🔧"),
    (":computer:", "💻"),
    (":phone:", "📱"),
    (":email:", "📧"),
    (":inbox:", "📥"),
    (":link:", "🔗"),
    (":chart:", "📊"),
    (":package:", "📦"),
    (":clock:", "🕐"),
    (":alarm:", "⏰"),
    (":hourglass:", "⏳"),
    (":stopwatch:", "⏱️"),
    (":flag:", "🚩"),
    (":construction:", "🚧"),
    (":recycle:", "♻️"),
    (":gem:", "💎"),
    (":crystal:", "💎"),
    (":shield:", "🛡️"),
    (":anchor:", "⚓"),
    (":globe:", "🌍"),
    (":earth:", "🌍"),
    (":sunrise:", "🌅"),
    (":sunset:", "🌇"),
    (":mountain:", "⛰️"),
    (":tree:", "🌳"),
    (":seedling:", "🌱"),
    (":flower:", "🌸"),
    (":rose:", "🌹"),
    (":cherry:", "🍒"),
    (":lemon:", "🍋"),
    (":banana:", "🍌"),
    (":grapes:", "🍇"),
    (":watermelon:", "🍉"),
    (":cookie:", "🍪"),
    (":chocolate:", "🍫"),
    (":candy:", "🍬"),
    (":cake:", "🍰"),
    (":soccer:", "⚽"),
    (":basketball:", "🏀"),
    (":football:", "🏈"),
    (":baseball:", "⚾"),
    (":tennis:", "🎾"),
    (":bowling:", "🎳"),
    (":golf:", "⛳"),
    (":dart_board:", "🎯"),
    (":gamepad:", "🎮"),
    (":joystick:", "🕹️"),
    (":dice:", "🎲"),
    (":puzzle:", "🧩"),
    (":muscle:", "💪"),
    (":brain:", "🧠"),
    (":handshake:", "🤝"),
    (":peace:", "✌️"),
    (":victory:", "✌️"),
    (":middle_finger:", "🖕"),
    (":rock:", "🤘"),
    (":call:", "🤙"),
    (":writing:", "✍️"),
    (":nail_care:", "💅"),
    (":selfie:", "🤳"),
    (":dancer:", "💃"),
    (":runner:", "🏃"),
    (":walking:", "🚶"),
    (":cyclist:", "🚴"),
    (":swimmer:", "🏊"),
    (":surfer:", "🏄"),
    (":bath:", "🛁"),
    (":bed:", "🛏️"),
    (":couch:", "🛋️"),
    (":toilet:", "🚽"),
    (":shower:", "🚿"),
    (":bathtub:", "🛁"),
    (":razor:", "🪒"),
    (":syringe:", "💉"),
    (":pill:", "💊"),
    (":stethoscope:", "🩺"),
    (":microscope:", "🔬"),
    (":telescope:", "🔭"),
    (":satellite:", "🛰️"),
    (":candle:", "🕯️"),
    (":scroll:", "📜"),
    (":calendar:", "📅"),
    (":card:", "📇"),
    (":clipboard:", "📋"),
    (":file:", "📄"),
    (":folder:", "📁"),
    (":newspaper:", "📰"),
    (":notebook:", "📓"),
    (":ledger:", "📒"),
    (":receipt:", "🧾"),
    (":bank:", "🏦"),
    (":hospital:", "🏥"),
    (":hotel:", "🏨"),
    (":store:", "🏪"),
    (":school:", "🏫"),
    (":factory:", "🏭"),
    (":tower:", "🗼"),
    (":castle:", "🏰"),
    (":church:", "⛪"),
    (":mosque:", "🕌"),
    (":synagogue:", "🕍"),
    (":shinto:", "⛩️"),
    (":kaaba:", "🕋"),
    (":fountain:", "⛲"),
    (":tent:", "⛺"),
    (":foggy:", "🌁"),
    (":night:", "🌃"),
    (":bridge:", "🌉"),
    (":hotsprings:", "♨️"),
    (":carousel:", "🎠"),
    (":ferris_wheel:", "🎡"),
    (":roller_coaster:", "🎢"),
    (":circus:", "🎪"),
    (":art:", "🎨"),
    (":slot_machine:", "🎰"),
    (":steam:", "🛁"),
];

fn filter_emojis(prefix: &str) -> Vec<AutocompleteSuggestion> {
    let prefix_lower = prefix.to_lowercase();
    // Strip leading ':' for matching.
    let search = prefix_lower.strip_prefix(':').unwrap_or(&prefix_lower);
    EMOJIS
        .iter()
        .filter(|(code, _)| {
            let code_name = code
                .strip_prefix(':')
                .and_then(|s| s.strip_suffix(':'))
                .unwrap_or(code);
            code_name.starts_with(search)
        })
        .take(20) // Limit dropdown size.
        .map(|(code, emoji)| AutocompleteSuggestion {
            label: code.to_string(),
            description: emoji.to_string(),
        })
        .collect()
}

// ============================================================================
// Dropdown rendering
// ============================================================================

/// Render the autocomplete dropdown as a list of display lines.
/// Returns (lines, dropdown_height).
/// The dropdown is positioned above the input line.
pub(crate) fn render_autocomplete(
    state: &AutocompleteState,
    max_width: usize,
    max_height: usize,
) -> Vec<String> {
    if !state.active || state.matches.is_empty() {
        return vec![];
    }

    let max_items = max_height.min(10);
    let items = &state.matches[..state.matches.len().min(max_items)];
    let inner_width = max_width.saturating_sub(4); // 2 for borders + 2 for padding

    let mut lines: Vec<String> = Vec::new();

    // Header
    let header_text = if state.is_emoji {
        format!(" {} ", fg_bold(AQUA.0, AQUA.1, AQUA.2, "Emoji"))
    } else {
        format!(" {} ", fg_bold(AQUA.0, AQUA.1, AQUA.2, "Commands"))
    };
    lines.push(header_text);
    lines.push(String::new());

    for (i, suggestion) in items.iter().enumerate() {
        let is_selected = i == state.selected;
        let prefix = if is_selected {
            fg_bold(AQUA.0, AQUA.1, AQUA.2, "▸")
        } else {
            dim(" ")
        };

        let label = if is_selected {
            fg_bold(AQUA.0, AQUA.1, AQUA.2, &suggestion.label)
        } else {
            dim(&suggestion.label)
        };

        let entry = format!("  {} {}  {}", prefix, label, dim(&suggestion.description));
        let wrapped = crate::ui_wrap::wrap_ansi(&entry, max_width.saturating_sub(2));
        lines.extend(wrapped);
    }

    lines.push(String::new());
    let hint = format!(
        "  {} select · {} navigate · {} close",
        dim("Enter"),
        dim("↑↓"),
        dim("Esc"),
    );
    lines.push(hint);

    lines
}
