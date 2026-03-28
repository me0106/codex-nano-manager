use ratatui::style::Color;

pub struct UiPalette {
    pub border: Color,
    pub title: Color,
    pub table_title_fg: Color,
    pub table_item_fg: Color,
    pub header_fg: Color,
    pub body_fg: Color,
    pub label_fg: Color,
    pub value_fg: Color,
    pub selected_fg: Color,
    pub selected_bg: Color,
    pub help_fg: Color,
    pub custom_accent: Color,
    pub error_fg: Color,
    pub keycap_fg: Color,
    pub keycap_bg: Color,
}

pub fn ui_palette() -> UiPalette {
    UiPalette {
        border: Color::Rgb(76, 96, 99),
        title: Color::Rgb(184, 214, 209),
        table_title_fg: Color::Rgb(204, 226, 222),
        table_item_fg: Color::Rgb(168, 181, 178),
        header_fg: Color::Rgb(218, 226, 224),
        body_fg: Color::Rgb(186, 198, 196),
        label_fg: Color::Rgb(136, 150, 147),
        value_fg: Color::Rgb(229, 236, 234),
        selected_fg: Color::Rgb(240, 246, 244),
        selected_bg: Color::Rgb(58, 84, 80),
        help_fg: Color::Rgb(126, 142, 140),
        custom_accent: Color::Rgb(166, 150, 119),
        error_fg: Color::Rgb(210, 177, 124),
        keycap_fg: Color::Rgb(184, 214, 209),
        keycap_bg: Color::Reset,
    }
}
