pub mod dialogs;
pub mod window;

use std::path::PathBuf;

use shadow_batch::paths;

pub fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("style.css"));
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

pub fn register_icons() {
    gtk::Window::set_default_icon_name(paths::APP_ICON);
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let theme = gtk::IconTheme::for_display(&display);
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join("data/icons");
        if p.join("hicolor/scalable/apps/shadow-batch-processor.svg").is_file() {
            theme.add_search_path(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            theme.add_search_path(dir.join("../share/icons"));
        }
    }
}

pub fn icon_paintable() -> Option<gtk::gdk::Texture> {
    const PNG: &[u8] =
        include_bytes!("../../data/icons/hicolor/256x256/apps/shadow-batch-processor.png");
    gtk::gdk::Texture::from_bytes(&gtk::glib::Bytes::from_static(PNG)).ok()
}

pub fn apply_theme(theme: shadow_batch::Theme) {
    let scheme = match theme {
        shadow_batch::Theme::System => adw::ColorScheme::Default,
        shadow_batch::Theme::Light => adw::ColorScheme::ForceLight,
        shadow_batch::Theme::Dark => adw::ColorScheme::ForceDark,
    };
    adw::StyleManager::default().set_color_scheme(scheme);
}

#[allow(dead_code)]
fn _icon_paths() -> Vec<PathBuf> {
    Vec::new()
}
