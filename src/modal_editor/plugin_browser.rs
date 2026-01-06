//! Plugin Browser Panel
//!
//! Provides a UI panel for browsing available plugins and managing instances.
//! Accessible via Alt+P in the modal editor.

use crate::plugin_host::{PluginCategory, PluginFormat, PluginInfo, PluginInstanceManager};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Plugin browser view mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserView {
    /// Show available plugins
    Available,
    /// Show active instances
    Instances,
}

/// Plugin browser state
pub struct PluginBrowser {
    /// Whether the browser is visible
    visible: bool,
    /// Current view mode
    view: BrowserView,
    /// Selected index in the current list
    selected_index: usize,
    /// Filter text for searching
    filter: String,
    /// Whether in name input mode (for creating instances)
    naming_mode: bool,
    /// Name input buffer
    name_input: String,
    /// Status message
    status: String,
}

impl Default for PluginBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginBrowser {
    /// Create a new plugin browser
    pub fn new() -> Self {
        Self {
            visible: false,
            view: BrowserView::Available,
            selected_index: 0,
            filter: String::new(),
            naming_mode: false,
            name_input: String::new(),
            status: String::new(),
        }
    }

    /// Toggle browser visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.selected_index = 0;
            self.filter.clear();
            self.naming_mode = false;
            self.status.clear();
        }
    }

    /// Check if browser is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Hide the browser
    pub fn hide(&mut self) {
        self.visible = false;
        self.naming_mode = false;
    }

    /// Switch to available plugins view
    pub fn show_available(&mut self) {
        self.view = BrowserView::Available;
        self.selected_index = 0;
    }

    /// Switch to instances view
    pub fn show_instances(&mut self) {
        self.view = BrowserView::Instances;
        self.selected_index = 0;
    }

    /// Toggle between views
    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            BrowserView::Available => BrowserView::Instances,
            BrowserView::Instances => BrowserView::Available,
        };
        self.selected_index = 0;
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self, max_items: usize) {
        if self.selected_index + 1 < max_items {
            self.selected_index += 1;
        }
    }

    /// Add character to filter or name input
    pub fn add_char(&mut self, c: char) {
        if self.naming_mode {
            self.name_input.push(c);
        } else {
            self.filter.push(c);
            self.selected_index = 0;
        }
    }

    /// Delete character from filter or name input
    pub fn delete_char(&mut self) {
        if self.naming_mode {
            self.name_input.pop();
        } else {
            self.filter.pop();
            self.selected_index = 0;
        }
    }

    /// Check if in naming mode
    pub fn is_naming(&self) -> bool {
        self.naming_mode
    }

    /// Enter naming mode for creating an instance
    pub fn start_naming(&mut self, suggested_name: &str) {
        self.naming_mode = true;
        self.name_input = suggested_name.to_string();
        self.status = "Enter instance name (Enter to confirm, Esc to cancel)".to_string();
    }

    /// Cancel naming mode
    pub fn cancel_naming(&mut self) {
        self.naming_mode = false;
        self.name_input.clear();
        self.status.clear();
    }

    /// Confirm naming and get the name
    pub fn confirm_naming(&mut self) -> Option<String> {
        if self.naming_mode && !self.name_input.is_empty() {
            let name = self.name_input.clone();
            self.naming_mode = false;
            self.name_input.clear();
            self.status = format!("Created instance: ~{}", name);
            Some(name)
        } else {
            None
        }
    }

    /// Get current view
    pub fn current_view(&self) -> BrowserView {
        self.view
    }

    /// Get selected index
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Get current filter
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Set status message
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    /// Render the plugin browser
    pub fn render(&self, f: &mut Frame, area: Rect, manager: &PluginInstanceManager) {
        // Main container block
        let block = Block::default()
            .title("Plugin Browser [Tab: switch view, Enter: create, Esc: close]")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Split into view tabs, content, and status
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // View tabs
                Constraint::Min(5),    // Content
                Constraint::Length(3), // Status/filter
            ])
            .split(inner);

        // View tabs
        let tabs_block = Block::default().borders(Borders::BOTTOM);
        let available_style = if self.view == BrowserView::Available {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        let instances_style = if self.view == BrowserView::Instances {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        let plugins = manager.list_plugins();
        let instances = manager.list_instances();

        let tabs_text = Line::from(vec![
            Span::styled(
                format!(" Available ({}) ", plugins.len()),
                available_style,
            ),
            Span::raw(" | "),
            Span::styled(
                format!(" Instances ({}) ", instances.len()),
                instances_style,
            ),
        ]);
        let tabs = Paragraph::new(tabs_text).block(tabs_block);
        f.render_widget(tabs, chunks[0]);

        // Content area
        match self.view {
            BrowserView::Available => {
                self.render_available_plugins(f, chunks[1], manager);
            }
            BrowserView::Instances => {
                self.render_instances(f, chunks[1], manager);
            }
        }

        // Status/filter bar
        let status_block = Block::default().borders(Borders::TOP);
        let status_text = if self.naming_mode {
            format!("Name: {}|", self.name_input)
        } else if !self.filter.is_empty() {
            format!("Filter: {}", self.filter)
        } else if !self.status.is_empty() {
            self.status.clone()
        } else {
            "Type to filter, Enter to create instance".to_string()
        };
        let status = Paragraph::new(status_text)
            .block(status_block)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(status, chunks[2]);
    }

    /// Render available plugins list
    fn render_available_plugins(
        &self,
        f: &mut Frame,
        area: Rect,
        manager: &PluginInstanceManager,
    ) {
        let plugins = manager.list_plugins();

        // Filter plugins
        let filter_lower = self.filter.to_lowercase();
        let filtered: Vec<&PluginInfo> = plugins
            .iter()
            .filter(|p| {
                self.filter.is_empty() || p.id.name.to_lowercase().contains(&filter_lower)
            })
            .copied()
            .collect();

        if filtered.is_empty() {
            let msg = if plugins.is_empty() {
                "No plugins found. Run 'phonon plugins scan' to scan for plugins."
            } else {
                "No plugins match the filter."
            };
            let para = Paragraph::new(msg)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            f.render_widget(para, area);
            return;
        }

        // Create list items
        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(i, plugin)| {
                let format_str = match plugin.id.format {
                    PluginFormat::Vst3 => "VST3",
                    PluginFormat::AudioUnit => "AU",
                    PluginFormat::Clap => "CLAP",
                    PluginFormat::Lv2 => "LV2",
                };
                let category_str = match plugin.category {
                    PluginCategory::Instrument => "Synth",
                    PluginCategory::Effect => "FX",
                    PluginCategory::MidiEffect => "MIDI",
                    PluginCategory::Unknown => "?",
                };

                let is_selected = i == self.selected_index;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let prefix = if is_selected { ">" } else { " " };
                let text = format!(
                    "{} {} [{}] ({})",
                    prefix, plugin.id.name, format_str, category_str
                );
                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, area);
    }

    /// Render active instances list
    fn render_instances(&self, f: &mut Frame, area: Rect, manager: &PluginInstanceManager) {
        let instances = manager.list_instances();

        if instances.is_empty() {
            let para = Paragraph::new("No active instances. Select a plugin and press Enter to create one.")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(para, area);
            return;
        }

        // Create list items
        let items: Vec<ListItem> = instances
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let is_selected = i == self.selected_index;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let prefix = if is_selected { ">" } else { " " };
                // Get instance info if possible
                let text = if let Some(instance) = manager.get_instance(name) {
                    if let Ok(handle) = instance.lock() {
                        let info = handle.info();
                        format!("{} ~{} ({})", prefix, name, info.id.name)
                    } else {
                        format!("{} ~{}", prefix, name)
                    }
                } else {
                    format!("{} ~{}", prefix, name)
                };
                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, area);
    }

    /// Get the selected plugin (if in Available view)
    pub fn selected_plugin<'a>(&self, manager: &'a PluginInstanceManager) -> Option<&'a PluginInfo> {
        if self.view != BrowserView::Available {
            return None;
        }

        let plugins = manager.list_plugins();
        let filter_lower = self.filter.to_lowercase();
        let filtered: Vec<&PluginInfo> = plugins
            .iter()
            .filter(|p| {
                self.filter.is_empty() || p.id.name.to_lowercase().contains(&filter_lower)
            })
            .copied()
            .collect();

        filtered.get(self.selected_index).copied()
    }

    /// Get the selected instance name (if in Instances view)
    pub fn selected_instance_name(&self, manager: &PluginInstanceManager) -> Option<String> {
        if self.view != BrowserView::Instances {
            return None;
        }

        let instances = manager.list_instances();
        instances.get(self.selected_index).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_toggle() {
        let mut browser = PluginBrowser::new();
        assert!(!browser.is_visible());

        browser.toggle();
        assert!(browser.is_visible());

        browser.toggle();
        assert!(!browser.is_visible());
    }

    #[test]
    fn test_view_switching() {
        let mut browser = PluginBrowser::new();
        assert_eq!(browser.current_view(), BrowserView::Available);

        browser.toggle_view();
        assert_eq!(browser.current_view(), BrowserView::Instances);

        browser.toggle_view();
        assert_eq!(browser.current_view(), BrowserView::Available);
    }

    #[test]
    fn test_selection() {
        let mut browser = PluginBrowser::new();
        assert_eq!(browser.selected_index(), 0);

        browser.select_next(5);
        assert_eq!(browser.selected_index(), 1);

        browser.select_next(5);
        browser.select_next(5);
        browser.select_next(5);
        assert_eq!(browser.selected_index(), 4);

        // Can't go past end
        browser.select_next(5);
        assert_eq!(browser.selected_index(), 4);

        browser.select_prev();
        assert_eq!(browser.selected_index(), 3);

        // Can't go below 0
        browser.selected_index = 0;
        browser.select_prev();
        assert_eq!(browser.selected_index(), 0);
    }

    #[test]
    fn test_filter() {
        let mut browser = PluginBrowser::new();
        assert!(browser.filter().is_empty());

        browser.add_char('o');
        browser.add_char('s');
        browser.add_char('i');
        assert_eq!(browser.filter(), "osi");

        browser.delete_char();
        assert_eq!(browser.filter(), "os");
    }

    #[test]
    fn test_naming_mode() {
        let mut browser = PluginBrowser::new();
        assert!(!browser.is_naming());

        browser.start_naming("osirus:1");
        assert!(browser.is_naming());

        browser.add_char('a');
        // In naming mode, typing adds to name_input, not filter
        assert_eq!(browser.filter(), "");

        browser.cancel_naming();
        assert!(!browser.is_naming());
    }
}
