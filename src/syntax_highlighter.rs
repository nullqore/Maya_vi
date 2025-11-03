use eframe::egui;
use egui::text::LayoutJob;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

lazy_static::lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

pub struct CodeTheme {
    theme: &'static Theme,
}

impl Default for CodeTheme {
    fn default() -> Self {
        Self {
            theme: &THEME_SET.themes["base16-ocean.dark"],
        }
    }
}

impl CodeTheme {
    pub fn highlight(&self, _ui: &egui::Ui, lang: &str, code: &str) -> LayoutJob {
        let syntax = SYNTAX_SET.find_syntax_by_extension(lang).unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, self.theme);
        let mut job = LayoutJob::default();

        for line in LinesWithEndings::from(code) {
            let ranges = h.highlight_line(line, &SYNTAX_SET).unwrap();
            for (style, text) in ranges {
                let color = egui::Color32::from_rgb(
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                );
                job.append(text, 0.0, egui::TextFormat { font_id: egui::FontId::monospace(14.0), color, ..Default::default() });
            }
        }
        job
    }
}
