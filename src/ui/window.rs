use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use adw::prelude::*;
use gtk::gdk;
use gtk::gio;
use gtk::glib;

use shadow_batch::files;
use shadow_batch::paths;
use shadow_batch::pipeline::{preview, NumberStyle, Pipeline, Step};
use shadow_batch::presets;
use shadow_batch::runner::{self, BatchReport};
use shadow_batch::settings::Settings;
use shadow_batch::Error;

use crate::ui::{self, dialogs};

struct State {
    settings: RefCell<Settings>,
    files: RefCell<Vec<PathBuf>>,
    pipeline: RefCell<Pipeline>,
    busy: Cell<bool>,
    cancel: RefCell<Option<Arc<AtomicBool>>>,
    pause: RefCell<Option<Arc<AtomicBool>>>,
}

struct Widgets {
    window: adw::ApplicationWindow,
    toast: adw::ToastOverlay,
    files_box: gtk::ListBox,
    steps_box: gtk::ListBox,
    preview_box: gtk::ListBox,
    output_label: gtk::Label,
    copies: gtk::Switch,
    progress: gtk::ProgressBar,
    status: gtk::Label,
    run_btn: gtk::Button,
    pause_btn: gtk::Button,
    cancel_btn: gtk::Button,
    drop_zone: gtk::Box,
    report_box: gtk::Box,
}

pub fn present(app: &adw::Application, initial: Vec<PathBuf>) {
    ui::load_css();
    let settings = Settings::load();
    ui::apply_theme(settings.theme);
    if let Some(win) = app.active_window() {
        win.present();
        return;
    }

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(paths::APP_NAME)
        .default_width(980)
        .default_height(860)
        .build();
    window.set_icon_name(Some(paths::APP_ICON));

    let toast = adw::ToastOverlay::new();
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let add_btn = gtk::Button::from_icon_name("list-add-symbolic");
    add_btn.set_tooltip_text(Some("Add files"));
    let settings_btn = gtk::Button::from_icon_name("emblem-system-symbolic");
    settings_btn.set_tooltip_text(Some("Settings"));
    let about_btn = gtk::Button::from_icon_name("help-about-symbolic");
    about_btn.set_tooltip_text(Some("About"));
    header.pack_start(&add_btn);
    header.pack_end(&settings_btn);
    header.pack_end(&about_btn);
    toolbar.add_top_bar(&header);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_start(18);
    root.set_margin_end(18);
    root.set_margin_top(8);
    root.set_margin_bottom(18);
    root.append(&brand());

    let drop_zone = gtk::Box::new(gtk::Orientation::Vertical, 6);
    drop_zone.add_css_class("drop-zone");
    let drop_l = gtk::Label::new(Some("Drop many files here"));
    drop_l.add_css_class("title-4");
    drop_l.set_halign(gtk::Align::Center);
    let drop_h = gtk::Label::new(Some("Folders, empty files, and unreadable paths are skipped with an explanation."));
    drop_h.add_css_class("dim-label");
    drop_h.set_wrap(true);
    drop_h.set_halign(gtk::Align::Center);
    drop_zone.append(&drop_l);
    drop_zone.append(&drop_h);
    root.append(&drop_zone);

    let cols = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    cols.set_homogeneous(true);
    cols.set_vexpand(true);

    let files_box = gtk::ListBox::new();
    files_box.add_css_class("boxed-list");
    files_box.set_selection_mode(gtk::SelectionMode::None);
    let files_scroll = gtk::ScrolledWindow::new();
    files_scroll.set_child(Some(&files_box));
    files_scroll.set_vexpand(true);
    files_scroll.set_min_content_height(220);
    let left = gtk::Box::new(gtk::Orientation::Vertical, 6);
    left.append(&heading("Files"));
    left.append(&files_scroll);
    let remove = gtk::Button::with_label("Remove last");
    remove.add_css_class("flat");
    left.append(&remove);

    let steps_box = gtk::ListBox::new();
    steps_box.add_css_class("boxed-list");
    steps_box.set_selection_mode(gtk::SelectionMode::None);
    let steps_scroll = gtk::ScrolledWindow::new();
    steps_scroll.set_child(Some(&steps_box));
    steps_scroll.set_vexpand(true);
    let mid = gtk::Box::new(gtk::Orientation::Vertical, 6);
    mid.append(&heading("Pipeline"));
    let preset_labels = preset_names();
    let preset_refs: Vec<&str> = preset_labels.iter().map(String::as_str).collect();
    let preset = gtk::DropDown::from_strings(&preset_refs);
    mid.append(&preset);
    mid.append(&steps_scroll);
    let step_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add_step = gtk::DropDown::from_strings(&[
        "Add step…",
        "Rename / number",
        "Convert images",
        "Convert video",
        "Convert audio",
        "Extract audio",
        "Remove metadata",
    ]);
    add_step.set_hexpand(true);
    let up = gtk::Button::from_icon_name("go-up-symbolic");
    up.set_tooltip_text(Some("Move last step up"));
    let del_step = gtk::Button::from_icon_name("user-trash-symbolic");
    del_step.set_tooltip_text(Some("Remove last step"));
    step_row.append(&add_step);
    step_row.append(&up);
    step_row.append(&del_step);
    mid.append(&step_row);

    cols.append(&left);
    cols.append(&mid);
    root.append(&cols);

    root.append(&heading("Preview (source → copy)"));
    let preview_box = gtk::ListBox::new();
    preview_box.add_css_class("boxed-list");
    preview_box.set_selection_mode(gtk::SelectionMode::None);
    let preview_scroll = gtk::ScrolledWindow::new();
    preview_scroll.set_child(Some(&preview_box));
    preview_scroll.set_min_content_height(140);
    root.append(&preview_scroll);

    let out_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let output_label = gtk::Label::new(None);
    output_label.set_xalign(0.0);
    output_label.set_hexpand(true);
    output_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    let out_btn = gtk::Button::with_label("Output folder");
    out_btn.set_tooltip_text(Some("Choose where copies are written"));
    out_row.append(&output_label);
    out_row.append(&out_btn);
    root.append(&out_row);

    let copies = gtk::Switch::new();
    copies.set_active(settings.process_copies);
    copies.set_valign(gtk::Align::Center);
    let copies_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let copies_l = gtk::Label::new(Some("Process copies"));
    copies_l.set_hexpand(true);
    copies_l.set_xalign(0.0);
    copies_row.append(&copies_l);
    copies_row.append(&copies);
    root.append(&copies_row);

    let progress = gtk::ProgressBar::new();
    progress.set_show_text(true);
    let status = gtk::Label::new(Some("Add files, choose a preset, preview, then run."));
    status.add_css_class("dim-label");
    status.set_xalign(0.0);
    status.set_wrap(true);
    root.append(&progress);
    root.append(&status);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let run_btn = gtk::Button::with_label("Run pipeline");
    run_btn.add_css_class("suggested-action");
    run_btn.add_css_class("run-button");
    run_btn.add_css_class("pill");
    let pause_btn = gtk::Button::with_label("Pause");
    pause_btn.add_css_class("pill");
    pause_btn.set_sensitive(false);
    let cancel_btn = gtk::Button::with_label("Cancel");
    cancel_btn.add_css_class("destructive-action");
    cancel_btn.add_css_class("pill");
    cancel_btn.set_sensitive(false);
    buttons.append(&run_btn);
    buttons.append(&pause_btn);
    buttons.append(&cancel_btn);
    root.append(&buttons);

    let report_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    root.append(&heading("Report"));
    root.append(&report_box);

    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&root));
    toolbar.set_content(Some(&scroll));
    toast.set_child(Some(&toolbar));
    window.set_content(Some(&toast));

    let widgets = Rc::new(Widgets {
        window: window.clone(),
        toast,
        files_box,
        steps_box,
        preview_box,
        output_label,
        copies,
        progress,
        status,
        run_btn,
        pause_btn,
        cancel_btn,
        drop_zone,
        report_box,
    });
    let pipeline = settings.last_pipeline.clone();
    let state = Rc::new(State {
        settings: RefCell::new(settings),
        files: RefCell::new(Vec::new()),
        pipeline: RefCell::new(if pipeline.steps.is_empty() {
            presets::builtins()[0].clone()
        } else {
            pipeline
        }),
        busy: Cell::new(false),
        cancel: RefCell::new(None),
        pause: RefCell::new(None),
    });

    refresh_all(&widgets, &state);

    let w = widgets.clone();
    let s = state.clone();
    add_btn.connect_clicked(move |_| choose_files(&w, &s));
    let s = state.clone();
    let w = widgets.clone();
    remove.connect_clicked(move |_| {
        s.files.borrow_mut().pop();
        refresh_all(&w, &s);
    });
    let s = state.clone();
    let w = widgets.clone();
    preset.connect_selected_notify(move |drop| {
        let names = preset_names();
        if let Some(name) = names.get(drop.selected() as usize) {
            if let Some(p) = presets::all_presets().into_iter().find(|p| p.name == *name) {
                s.pipeline.replace(p);
                refresh_all(&w, &s);
            }
        }
    });
    let s = state.clone();
    let w = widgets.clone();
    add_step.connect_selected_notify(move |drop| {
        if let Some(step) = step_from_index(drop.selected()) {
            s.pipeline.borrow_mut().steps.push(step);
            refresh_all(&w, &s);
        }
        drop.set_selected(0);
    });
    let s = state.clone();
    let w = widgets.clone();
    up.connect_clicked(move |_| {
        let mut p = s.pipeline.borrow_mut();
        let n = p.steps.len();
        if n >= 2 {
            p.steps.swap(n - 1, n - 2);
        }
        drop(p);
        refresh_all(&w, &s);
    });
    let s = state.clone();
    let w = widgets.clone();
    del_step.connect_clicked(move |_| {
        s.pipeline.borrow_mut().steps.pop();
        refresh_all(&w, &s);
    });
    let s = state.clone();
    let w = widgets.clone();
    let win = widgets.window.clone();
    out_btn.connect_clicked(move |_| {
        let s = s.clone();
        let w = w.clone();
        dialogs::pick_folder(&win, move |path| {
            s.settings.borrow_mut().output_dir = path;
            let _ = s.settings.borrow().save();
            refresh_all(&w, &s);
        });
    });
    let w = widgets.clone();
    let s = state.clone();
    widgets.run_btn.connect_clicked(move |_| start_run(&w, &s));
    let s = state.clone();
    let w = widgets.clone();
    widgets.pause_btn.connect_clicked(move |btn| {
        if let Some(flag) = s.pause.borrow().as_ref() {
            let next = !flag.load(Ordering::SeqCst);
            flag.store(next, Ordering::SeqCst);
            btn.set_label(if next { "Resume" } else { "Pause" });
            w.status.set_text(if next { "Paused between files." } else { "Running…" });
        }
    });
    let s = state.clone();
    widgets.cancel_btn.connect_clicked(move |_| {
        if let Some(flag) = s.cancel.borrow().as_ref() {
            flag.store(true, Ordering::SeqCst);
        }
    });
    let s = state.clone();
    let w = widgets.clone();
    settings_btn.connect_clicked(move |_| {
        let cur = s.settings.borrow().clone();
        let s_cb = s.clone();
        let w_cb = w.clone();
        let window = w.window.clone();
        dialogs::show_settings(&window, &cur, move |updated| {
            s_cb.settings.replace(updated);
            refresh_all(&w_cb, &s_cb);
        });
    });
    let win = widgets.window.clone();
    about_btn.connect_clicked(move |_| dialogs::show_about(&win));

    setup_drop(&widgets, &state);
    add_paths(&widgets, &state, initial);
    window.present();
}

fn brand() -> gtk::Box {
    let kicker = gtk::Label::new(Some("SHADOW BATCH PROCESSOR"));
    kicker.add_css_class("brand-kicker");
    kicker.set_xalign(0.0);
    let title = gtk::Label::new(Some("Many files, one pipeline"));
    title.add_css_class("brand-title");
    title.set_xalign(0.0);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.append(&kicker);
    text.append(&title);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    if let Some(tex) = ui::icon_paintable() {
        let image = gtk::Image::from_paintable(Some(&tex));
        image.set_pixel_size(48);
        row.append(&image);
    }
    row.append(&text);
    row
}

fn heading(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class("heading");
    l.set_xalign(0.0);
    l
}

fn preset_names() -> Vec<String> {
    presets::all_presets().into_iter().map(|p| p.name).collect()
}

fn step_from_index(i: u32) -> Option<Step> {
    match i {
        1 => Some(Step::Rename {
            prefix: String::new(),
            suffix: String::new(),
            numbering: NumberStyle::ZeroPad3,
            extension: None,
        }),
        2 => Some(Step::ConvertImage {
            format: "jpeg".into(),
            max_width: Some(1920),
            quality: 85,
        }),
        3 => Some(Step::ConvertVideo {
            container: "mp4".into(),
            height: Some(1080),
            compress: false,
        }),
        4 => Some(Step::ConvertAudio {
            format: "mp3".into(),
            normalize: false,
        }),
        5 => Some(Step::ExtractAudio {
            format: "mp3".into(),
        }),
        6 => Some(Step::RemoveMetadata),
        _ => None,
    }
}

fn setup_drop(widgets: &Rc<Widgets>, state: &Rc<State>) {
    let target = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    let zone = widgets.drop_zone.clone();
    target.connect_enter(move |_, _, _| {
        zone.add_css_class("drop-hover");
        gdk::DragAction::COPY
    });
    let zone = widgets.drop_zone.clone();
    target.connect_leave(move |_| zone.remove_css_class("drop-hover"));
    let widgets_d = widgets.clone();
    let state_d = state.clone();
    target.connect_drop(move |_, value, _, _| {
        widgets_d.drop_zone.remove_css_class("drop-hover");
        if let Ok(list) = value.get::<gdk::FileList>() {
            let paths: Vec<PathBuf> = list.files().iter().filter_map(|f| f.path()).collect();
            add_paths(&widgets_d, &state_d, paths);
            return true;
        }
        false
    });
    widgets.drop_zone.add_controller(target);
}

fn choose_files(widgets: &Rc<Widgets>, state: &Rc<State>) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title("Add files");
    let widgets_cb = widgets.clone();
    let state_cb = state.clone();
    let window = widgets.window.clone();
    dialog.open_multiple(Some(&window), gio::Cancellable::NONE, move |result| {
        if let Ok(model) = result {
            let mut paths = Vec::new();
            for i in 0..model.n_items() {
                if let Some(file) = model.item(i).and_downcast::<gio::File>() {
                    if let Some(path) = file.path() {
                        paths.push(path);
                    }
                }
            }
            add_paths(&widgets_cb, &state_cb, paths);
        }
    });
}

fn add_paths(widgets: &Widgets, state: &State, incoming: Vec<PathBuf>) {
    if incoming.is_empty() {
        return;
    }
    let result = files::collect_paths(incoming);
    if let Some(err) = files::explain_skips(&result) {
        dialogs::show_error(&widgets.window, &err);
    }
    state.files.borrow_mut().extend(result.added);
    refresh_all(widgets, state);
}

fn refresh_all(widgets: &Widgets, state: &State) {
    while let Some(c) = widgets.files_box.first_child() {
        widgets.files_box.remove(&c);
    }
    for (i, path) in state.files.borrow().iter().enumerate() {
        let row = adw::ActionRow::new();
        row.set_title(&format!("{}. {}", i + 1, path.file_name().and_then(|n| n.to_str()).unwrap_or("file")));
        row.set_subtitle(&paths::display_home_path(path));
        widgets.files_box.append(&row);
    }
    while let Some(c) = widgets.steps_box.first_child() {
        widgets.steps_box.remove(&c);
    }
    for (i, step) in state.pipeline.borrow().steps.iter().enumerate() {
        let row = adw::ActionRow::new();
        row.set_title(&format!("{}. {}", i + 1, step.label()));
        widgets.steps_box.append(&row);
    }
    while let Some(c) = widgets.preview_box.first_child() {
        widgets.preview_box.remove(&c);
    }
    let out = state.settings.borrow().output_dir.clone();
    widgets
        .output_label
        .set_text(&format!("Output: {}", paths::display_home_path(&out)));
    for map in preview(&state.files.borrow(), &state.pipeline.borrow(), &out)
        .into_iter()
        .take(80)
    {
        let row = adw::ActionRow::new();
        row.set_title(
            &map.source
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
        row.set_subtitle(&format!(
            "→ {}",
            map.dest
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        ));
        widgets.preview_box.append(&row);
    }
}

fn start_run(widgets: &Rc<Widgets>, state: &Rc<State>) {
    if state.busy.get() {
        return;
    }
    if state.files.borrow().is_empty() {
        dialogs::show_error(
            &widgets.window,
            &Error::user("Add at least one file before running the pipeline."),
        );
        return;
    }
    if state.pipeline.borrow().steps.is_empty() {
        dialogs::show_error(
            &widgets.window,
            &Error::user("Add at least one pipeline step, or pick a preset."),
        );
        return;
    }
    let copies = widgets.copies.is_active();
    if !copies {
        let w = widgets.clone();
        let s = state.clone();
        dialogs::confirm_destructive(&widgets.window, move || run_now(&w, &s, false));
        return;
    }
    run_now(widgets, state, true);
}

fn run_now(widgets: &Rc<Widgets>, state: &Rc<State>, copies: bool) {
    state.busy.set(true);
    let cancel = Arc::new(AtomicBool::new(false));
    let pause = Arc::new(AtomicBool::new(false));
    state.cancel.replace(Some(cancel.clone()));
    state.pause.replace(Some(pause.clone()));
    widgets.run_btn.set_sensitive(false);
    widgets.pause_btn.set_sensitive(true);
    widgets.cancel_btn.set_sensitive(true);
    widgets.pause_btn.set_label("Pause");
    widgets.progress.set_fraction(0.0);
    widgets.status.set_text("Running…");
    while let Some(c) = widgets.report_box.first_child() {
        widgets.report_box.remove(&c);
    }

    let files = state.files.borrow().clone();
    let pipeline = state.pipeline.borrow().clone();
    let output = state.settings.borrow().output_dir.clone();
    state.settings.borrow_mut().last_pipeline = pipeline.clone();
    state.settings.borrow_mut().process_copies = copies;
    let _ = state.settings.borrow().save();

    let (tx, rx) = async_channel::unbounded::<UiMsg>();
    thread::spawn(move || {
        let report = runner::run_batch(
            &files,
            &pipeline,
            &output,
            copies,
            cancel,
            pause,
            |p| {
                let _ = tx.send_blocking(UiMsg::Progress(p));
            },
        );
        let _ = tx.send_blocking(UiMsg::Done(report));
    });

    let widgets = widgets.clone();
    let state = state.clone();
    glib::spawn_future_local(async move {
        while let Ok(msg) = rx.recv().await {
            match msg {
                UiMsg::Progress(p) => {
                    widgets.progress.set_fraction(p.fraction);
                    widgets.progress.set_text(Some(&format!("{}/{}", p.done, p.total)));
                    if let Some(cur) = p.current {
                        widgets.status.set_text(&format!(
                            "Working on {}",
                            cur.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
                        ));
                    }
                }
                UiMsg::Done(report) => {
                    finish_run(&widgets, &state, report);
                    break;
                }
            }
        }
    });
}

enum UiMsg {
    Progress(runner::BatchProgress),
    Done(BatchReport),
}

fn finish_run(widgets: &Widgets, state: &State, report: BatchReport) {
    state.busy.set(false);
    widgets.run_btn.set_sensitive(true);
    widgets.pause_btn.set_sensitive(false);
    widgets.cancel_btn.set_sensitive(false);
    let ok = report.results.iter().filter(|r| r.ok).count();
    let fail = report.results.len() - ok;
    widgets.progress.set_fraction(1.0);
    widgets.status.set_text(&format!(
        "{} finished, {} failed{}",
        ok,
        fail,
        if report.cancelled { " (cancelled)" } else { "" }
    ));
    for r in &report.results {
        let row = gtk::Label::new(Some(&format!(
            "{} — {}",
            r.source.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
            r.message
        )));
        row.set_xalign(0.0);
        row.set_wrap(true);
        if !r.ok {
            row.add_css_class("error");
        }
        widgets.report_box.append(&row);
    }
    if fail > 0 {
        if let Some(first) = report.results.iter().find(|r| !r.ok) {
            let err = if let Some(t) = &first.technical {
                Error::detailed(first.message.clone(), t.clone())
            } else {
                Error::user(format!("{fail} file(s) failed. See the report."))
            };
            dialogs::show_error(&widgets.window, &err);
        }
    } else {
        widgets.toast.add_toast(adw::Toast::new("Batch finished."));
    }
}
