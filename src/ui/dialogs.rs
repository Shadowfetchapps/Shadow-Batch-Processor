use adw::prelude::*;
use gtk::gio;
use gtk::glib;

use shadow_batch::paths;
use shadow_batch::settings::{Settings, Theme};
use shadow_batch::Error;

use crate::ui;

pub fn show_error(parent: &impl IsA<gtk::Window>, err: &Error) {
    let dialog = adw::MessageDialog::new(Some(parent), Some("Batch problem"), Some(&err.human_message()));
    dialog.add_response("ok", "OK");
    if let Some(details) = err.technical_details() {
        dialog.add_response("details", "Show Technical Details");
        dialog.set_response_appearance("details", adw::ResponseAppearance::Suggested);
        let parent = parent.as_ref().clone();
        dialog.connect_response(None, move |dlg, response| {
            if response == "details" {
                let tech = adw::MessageDialog::new(Some(&parent), Some("Technical details"), Some(&details));
                tech.add_response("ok", "OK");
                tech.present();
            }
            dlg.close();
        });
    }
    dialog.present();
}

pub fn show_about(parent: &impl IsA<gtk::Window>) {
    adw::AboutWindow::builder()
        .transient_for(parent)
        .modal(true)
        .application_name(paths::APP_NAME)
        .application_icon(paths::APP_ICON)
        .developer_name("Shadowfetch")
        .version(paths::APP_VERSION)
        .comments("Run local media pipelines on many files. Copies by default. No account or telemetry.")
        .license_type(gtk::License::MitX11)
        .website(paths::APP_WEBSITE)
        .issue_url("https://github.com/Shadowfetchapps/Shadow-Batch-Processor/issues")
        .copyright("© 2026 Shadow Batch Processor contributors")
        .build()
        .present();
}

pub fn show_settings(parent: &impl IsA<gtk::Window>, settings: &Settings, on_save: impl Fn(Settings) + 'static) {
    let window = adw::PreferencesWindow::builder()
        .transient_for(parent)
        .modal(true)
        .title("Settings")
        .search_enabled(false)
        .build();
    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::new();
    group.set_title("Batch");
    let copies = adw::SwitchRow::new();
    copies.set_title("Process copies (recommended)");
    copies.set_subtitle("Never write over the originals.");
    copies.set_active(settings.process_copies);
    group.add(&copies);
    let theme = adw::ComboRow::new();
    theme.set_title("Theme");
    theme.set_model(Some(&gtk::StringList::new(&["System", "Light", "Dark"])));
    theme.set_selected(settings.theme.index());
    group.add(&theme);
    page.add(&group);
    window.add(&page);
    let current = std::rc::Rc::new(std::cell::RefCell::new(settings.clone()));
    let c = current.clone();
    copies.connect_active_notify(move |row| c.borrow_mut().process_copies = row.is_active());
    let c = current.clone();
    theme.connect_selected_notify(move |row| {
        let t = Theme::from_index(row.selected());
        c.borrow_mut().theme = t;
        ui::apply_theme(t);
    });
    window.connect_close_request(move |_| {
        let snapshot = current.borrow().clone();
        let _ = snapshot.save();
        on_save(snapshot);
        glib::Propagation::Proceed
    });
    window.present();
}

pub fn confirm_destructive(parent: &impl IsA<gtk::Window>, on_yes: impl Fn() + 'static) {
    let dialog = adw::MessageDialog::new(
        Some(parent),
        Some("Write over original files?"),
        Some("Process Copies is off. This can replace the files you selected. This cannot be undone from inside the app."),
    );
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("destroy", "Replace originals");
    dialog.set_response_appearance("destroy", adw::ResponseAppearance::Destructive);
    dialog.connect_response(None, move |dlg, response| {
        if response == "destroy" {
            on_yes();
        }
        dlg.close();
    });
    dialog.present();
}

pub fn pick_folder(parent: &impl IsA<gtk::Window>, on_pick: impl Fn(std::path::PathBuf) + 'static) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title("Output folder");
    let parent = parent.as_ref().clone();
    dialog.select_folder(Some(&parent), gio::Cancellable::NONE, move |result| {
        if let Ok(file) = result {
            if let Some(path) = file.path() {
                on_pick(path);
            }
        }
    });
}
