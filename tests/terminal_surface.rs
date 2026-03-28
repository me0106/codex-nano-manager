use codex_nano_manager::config::ProviderConfig;
use codex_nano_manager::provider_templates::TemplateChoice;
use codex_nano_manager::selector::{ProviderSelector, RatatuiSelector, select_template_choice};
use codex_nano_manager::ui::render::render_provider_table;

fn provider() -> ProviderConfig {
    ProviderConfig {
        name: "router-a".into(),
        base_url: "https://router.example.com/v1".into(),
        env_key: "CXCM_KEY_12345678".into(),
        api_key: "secret".into(),
        model: Some("gpt-5.4".into()),
        last_used_at: None,
        notes: Some("internal relay".into()),
    }
}

#[test]
fn selector_surface_exports_ratatui_selector() {
    let selector = RatatuiSelector;
    let providers = vec![provider()];
    let _trait_object: &dyn ProviderSelector = &selector;
    assert_eq!(providers.len(), 1);
}

#[test]
fn template_selector_surface_is_available() {
    let picker: fn(&[TemplateChoice]) -> Result<TemplateChoice, codex_nano_manager::error::AppError> =
        select_template_choice;
    let _ = picker;
}

#[test]
fn render_provider_table_surface_is_available() {
    let output = render_provider_table(&[provider()], Some(0), 120, 6);
    assert!(output.contains("router-a"));
}
