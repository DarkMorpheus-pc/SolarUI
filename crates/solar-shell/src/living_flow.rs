// ==============================================================================
// ColorOS 17 "Living Flow Design" System for Blaze Universe & SolarUI
// Authentic Dark-Mode Architecture: Obsidian Smoked Glass, Condensed Light &
// Physics-Driven Flexible Feedback (Zero Neon Glows, Pure Industrial Precision)
// ==============================================================================

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CssProvider, IconTheme, Image, Label, Orientation, Window, WindowHandle,
};
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FluidCloudTone {
    Info,
    Success,
    Warning,
    Danger,
}

#[derive(Clone)]
pub struct FluidCloud {
    pill_box: GtkBox,
    dot_lbl: Label,
    text_lbl: Label,
    generation: Rc<RefCell<u64>>,
}

impl FluidCloud {
    pub fn new(initial_text: &str) -> Self {
        let pill_box = GtkBox::new(Orientation::Horizontal, 8);
        pill_box.add_css_class("lf-fluid-cloud");
        pill_box.add_css_class("cloud-info");
        pill_box.set_valign(gtk4::Align::Center);

        let dot_lbl = Label::new(Some("●"));
        dot_lbl.add_css_class("lf-cloud-dot");
        pill_box.append(&dot_lbl);

        let text_lbl = Label::new(Some(initial_text));
        text_lbl.add_css_class("lf-cloud-text");
        text_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        text_lbl.set_max_width_chars(38);
        pill_box.append(&text_lbl);

        Self {
            pill_box,
            dot_lbl,
            text_lbl,
            generation: Rc::new(RefCell::new(0)),
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.pill_box
    }

    /// Triggers a live ColorOS 17 Aqua Dynamics / Fluid Cloud morphing reaction
    /// in the window topbar with tactile spring compression/expansion.
    pub fn pulse(&self, message: &str, tone: FluidCloudTone, send_osd: bool) {
        let _ = crate::delight::play_acoustic_feedback("click");

        for cls in ["cloud-info", "cloud-ok", "cloud-warn", "cloud-danger", "cloud-pulse"] {
            self.pill_box.remove_css_class(cls);
        }

        let (tone_cls, symbol) = match tone {
            FluidCloudTone::Info => ("cloud-info", "●"),
            FluidCloudTone::Success => ("cloud-ok", "●"),
            FluidCloudTone::Warning => ("cloud-warn", "▲"),
            FluidCloudTone::Danger => ("cloud-danger", "◆"),
        };

        self.dot_lbl.set_text(symbol);
        self.text_lbl.set_text(message);
        self.pill_box.add_css_class(tone_cls);
        self.pill_box.add_css_class("cloud-pulse");

        let next_gen = {
            let mut g = self.generation.borrow_mut();
            *g += 1;
            *g
        };

        let pill_ref = self.pill_box.clone();
        let gen_ref = self.generation.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
            if *gen_ref.borrow() == next_gen {
                pill_ref.remove_css_class("cloud-pulse");
            }
        });

        if send_osd {
            let msg_owned = message.to_string();
            std::thread::spawn(move || {
                let _ = Command::new("notify-send")
                    .args([
                        "-a",
                        "SolarUI Living Flow",
                        "-i",
                        "solar-settings",
                        "Blaze Universe",
                        &msg_owned,
                    ])
                    .spawn();
            });
        }
    }
}

pub const LIVING_FLOW_CORE_CSS: &str = r#"
    @keyframes lf-elastic-morph {
        0%   { transform: scale(1.0); }
        40%  { transform: scale(1.028); }
        100% { transform: scale(1.0); }
    }

    window,
    window.background,
    window.csd {
        background-color: #121316;
        color: #f4f4f6;
    }

    *,
    *:focus,
    *:focus-visible,
    *:focus-within {
        outline-color: transparent;
        outline-width: 0px;
        outline-style: none;
    }

    headerbar {
        background: #17181d;
        background-image: none;
        color: #f4f4f6;
        border-bottom: 1px solid rgba(255, 255, 255, 0.07);
        box-shadow: none;
    }

    headerbar title,
    headerbar .title {
        color: #f4f4f6;
        font-weight: 700;
    }

    /* ColorOS 17 Obsidian Smoked Glass Window Shell + Condensed Light Rim */
    .lf-window-shell {
        background: linear-gradient(180deg, #17181d 0%, #111216 100%);
        border: 1px solid rgba(255, 255, 255, 0.09);
        border-radius: 20px;
    }

    .lf-topbar {
        background: linear-gradient(180deg, rgba(255, 255, 255, 0.032) 0%, rgba(255, 255, 255, 0.012) 100%);
        border-bottom: 1px solid rgba(255, 255, 255, 0.065);
        border-top-left-radius: 20px;
        border-top-right-radius: 20px;
        padding: 13px 20px;
    }

    .lf-topbar-title {
        color: #f5f5f7;
        font-size: 15.5px;
        font-weight: 700;
        letter-spacing: -0.2px;
    }

    .lf-topbar-subtitle {
        color: #9ca3af;
        font-size: 11.5px;
        font-weight: 500;
    }

    /* Authentic ColorOS 17 Aqua Dynamics / Fluid Cloud Capsule (Smoked Onyx, Zero Neon) */
    .lf-fluid-cloud {
        background-color: rgba(255, 255, 255, 0.055);
        border: 1px solid rgba(255, 255, 255, 0.10);
        border-radius: 999px;
        padding: 5px 14px;
        box-shadow: 0 3px 10px rgba(0, 0, 0, 0.32);
        transition: all 220ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-fluid-cloud.cloud-pulse {
        animation: lf-elastic-morph 340ms cubic-bezier(0.22, 1, 0.36, 1);
        background-color: rgba(255, 255, 255, 0.09);
    }

    .lf-fluid-cloud.cloud-info .lf-cloud-dot   { color: #e04828; }
    .lf-fluid-cloud.cloud-ok .lf-cloud-dot     { color: #34d399; }
    .lf-fluid-cloud.cloud-warn .lf-cloud-dot   { color: #f59e0b; }
    .lf-fluid-cloud.cloud-danger .lf-cloud-dot { color: #ef4444; }

    .lf-cloud-dot {
        font-size: 10px;
        font-weight: 800;
    }

    .lf-cloud-text {
        color: #e4e4e7;
        font-size: 12px;
        font-weight: 600;
    }

    /* Window Controls (Tactile Smoked Glass Pills) */
    .lf-win-btn {
        background-color: rgba(255, 255, 255, 0.065);
        background-image: none;
        color: #d4d4d8;
        border: 1px solid rgba(255, 255, 255, 0.10);
        border-radius: 999px;
        min-width: 30px;
        min-height: 30px;
        padding: 0 10px;
        font-size: 12px;
        font-weight: 700;
        transition: all 180ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-win-btn:hover {
        background-color: rgba(255, 255, 255, 0.14);
        background-image: none;
        color: #ffffff;
        transform: scale(1.04);
    }

    .lf-win-btn:active {
        transform: scale(0.94);
    }

    .lf-win-close:hover {
        background-color: #c83b22;
        background-image: none;
        border-color: rgba(255, 255, 255, 0.25);
        color: #ffffff;
    }

    /* Frosted Smoked-Titanium Cards with Top Condensed Light Highlight */
    .lf-card,
    .section-box,
    .installer-card {
        background: linear-gradient(180deg, rgba(32, 34, 40, 0.88) 0%, rgba(25, 27, 32, 0.92) 100%);
        border-top: 1px solid rgba(255, 255, 255, 0.10);
        border-left: 1px solid rgba(255, 255, 255, 0.07);
        border-right: 1px solid rgba(255, 255, 255, 0.07);
        border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        border-radius: 18px;
        padding: 18px 20px;
        margin-bottom: 14px;
        box-shadow: 0 8px 22px rgba(0, 0, 0, 0.30);
        transition: border-color 200ms cubic-bezier(0.22, 1, 0.36, 1),
                    background-color 200ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-card:hover,
    .section-box:hover,
    .installer-card:hover {
        border-color: rgba(255, 255, 255, 0.14);
    }

    /* Typography Hierarchy */
    .lf-section-title,
    .section-title,
    .section-heading {
        color: #f5f5f7;
        font-size: 15px;
        font-weight: 700;
        margin-bottom: 6px;
        letter-spacing: -0.15px;
    }

    .lf-label,
    .setting-label {
        color: #e4e4e7;
        font-size: 13.2px;
        font-weight: 600;
    }

    .lf-subtext,
    .setting-subtext,
    .section-desc {
        color: #a1a1aa;
        font-size: 12px;
        font-weight: 400;
    }

    button {
        background-image: none;
        text-shadow: none;
        -gtk-icon-shadow: none;
    }

    /* ColorOS 17 Tactile Buttons — Zero Neon, Warm Alabaster & Satin Vermilion */
    .lf-btn-primary,
    .action-btn,
    .start-btn {
        background: #f2f3f5;
        color: #111215;
        border: 1px solid rgba(255, 255, 255, 0.92);
        border-radius: 999px;
        padding: 9px 22px;
        font-size: 13px;
        font-weight: 700;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
        transition: all 180ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-btn-primary:hover,
    .action-btn:hover,
    .start-btn:hover {
        background: #ffffff;
        color: #09090b;
        box-shadow: 0 6px 16px rgba(0, 0, 0, 0.44);
        transform: scale(1.015);
    }

    .lf-btn-primary:active,
    .action-btn:active,
    .start-btn:active {
        transform: scale(0.965);
    }

    .lf-btn-flame,
    .primary-btn {
        background: linear-gradient(180deg, #cf3f24 0%, #b03018 100%);
        color: #ffffff;
        border: 1px solid rgba(255, 255, 255, 0.16);
        border-radius: 999px;
        padding: 9px 24px;
        font-size: 13.2px;
        font-weight: 700;
        box-shadow: 0 4px 14px rgba(0, 0, 0, 0.38);
        transition: all 180ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-btn-flame:hover,
    .primary-btn:hover {
        background: linear-gradient(180deg, #dc472b 0%, #be361d 100%);
        border-color: rgba(255, 255, 255, 0.26);
        box-shadow: 0 6px 18px rgba(0, 0, 0, 0.46);
        transform: scale(1.015);
    }

    .lf-btn-flame:active,
    .primary-btn:active {
        transform: scale(0.965);
    }

    .lf-btn-emerald,
    .install-btn {
        background-color: rgba(255, 255, 255, 0.11);
        color: #f4f4f6;
        border: 1px solid rgba(255, 255, 255, 0.16);
        border-radius: 999px;
        padding: 9px 20px;
        font-size: 13px;
        font-weight: 600;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.28);
        transition: all 180ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-btn-emerald:hover,
    .install-btn:hover {
        background-color: rgba(255, 255, 255, 0.17);
        border-color: rgba(255, 255, 255, 0.25);
        transform: scale(1.015);
    }

    .lf-btn-emerald:active,
    .install-btn:active {
        transform: scale(0.965);
    }

    .lf-btn-glass,
    .secondary-btn {
        background-color: rgba(255, 255, 255, 0.065);
        background-image: none;
        color: #e4e4e7;
        border: 1px solid rgba(255, 255, 255, 0.11);
        border-radius: 999px;
        padding: 8px 18px;
        font-size: 12.8px;
        font-weight: 600;
        transition: all 180ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-btn-glass:hover,
    .secondary-btn:hover {
        background-color: rgba(255, 255, 255, 0.125);
        background-image: none;
        color: #ffffff;
        border-color: rgba(255, 255, 255, 0.20);
        transform: scale(1.015);
    }

    .lf-btn-glass:active,
    .secondary-btn:active {
        transform: scale(0.965);
    }

    .lf-btn-danger,
    .danger-btn {
        background: linear-gradient(180deg, #b91c1c 0%, #991b1b 100%);
        color: #ffffff;
        border: 1px solid rgba(255, 255, 255, 0.16);
        border-radius: 999px;
        padding: 9px 22px;
        font-size: 13px;
        font-weight: 700;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.36);
        transition: all 180ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-btn-danger:hover,
    .danger-btn:hover {
        background: linear-gradient(180deg, #dc2626 0%, #b91c1c 100%);
        transform: scale(1.015);
    }

    .lf-btn-danger:active,
    .danger-btn:active {
        transform: scale(0.965);
    }

    /* Two-Pane Control Center Sidebar & Squircle Navigation Rows */
    .lf-sidebar {
        background-color: rgba(18, 19, 23, 0.72);
        border-right: 1px solid rgba(255, 255, 255, 0.065);
        padding: 12px 10px;
    }

    .lf-nav-pill {
        background-color: transparent;
        background-image: none;
        color: #a1a1aa;
        border: 1px solid transparent;
        border-radius: 13px;
        padding: 8px 11px;
        margin-bottom: 3px;
        font-size: 12.8px;
        font-weight: 600;
        transition: all 180ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-nav-pill:hover {
        background-color: rgba(255, 255, 255, 0.055);
        color: #f4f4f6;
    }

    .lf-nav-pill.active {
        background-color: rgba(255, 255, 255, 0.105);
        color: #ffffff;
        border: 1px solid rgba(255, 255, 255, 0.13);
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.26);
    }

    .lf-step-pill {
        background-color: rgba(255, 255, 255, 0.045);
        background-image: none;
        color: #a1a1aa;
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 999px;
        padding: 8px 18px;
        font-size: 12.8px;
        font-weight: 600;
        transition: all 190ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-step-pill:hover {
        background-color: rgba(255, 255, 255, 0.09);
        color: #f4f4f6;
    }

    .lf-step-pill.active {
        background-color: rgba(255, 255, 255, 0.12);
        color: #ffffff;
        border-color: rgba(255, 255, 255, 0.22);
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.32);
    }

    /* Status Badges */
    .status-badge-ok,
    .disk-badge-ok {
        color: #86efac;
        background-color: rgba(34, 197, 94, 0.12);
        border: 1px solid rgba(74, 222, 128, 0.25);
        border-radius: 999px;
        padding: 4px 12px;
        font-size: 11.5px;
        font-weight: 600;
    }

    .status-badge-info {
        color: #e4e4e7;
        background-color: rgba(255, 255, 255, 0.08);
        border: 1px solid rgba(255, 255, 255, 0.14);
        border-radius: 999px;
        padding: 4px 12px;
        font-size: 11.5px;
        font-weight: 600;
    }

    .disk-badge-warn {
        color: #fde68a;
        background-color: rgba(245, 158, 11, 0.11);
        border: 1px solid rgba(245, 158, 11, 0.26);
        border-radius: 14px;
        padding: 9px 14px;
        font-size: 12px;
        font-weight: 500;
    }

    /* Smoked-Titanium Form Widgets (DropDown, ComboBox, Entry, Popover, CheckButton) */
    entry,
    passwordentry,
    passwordentry > text,
    spinbutton {
        background-color: rgba(39, 41, 48, 0.94);
        background-image: none;
        color: #f4f4f6;
        border: 1px solid rgba(255, 255, 255, 0.10);
        border-radius: 13px;
        padding: 8px 14px;
        min-height: 22px;
        caret-color: #f4f4f6;
        transition: all 160ms ease;
    }

    entry:focus,
    passwordentry:focus-within {
        border-color: rgba(255, 255, 255, 0.28);
        background-color: rgba(45, 48, 56, 0.98);
        box-shadow: none;
    }

    dropdown > button,
    combobox > box > button {
        background-color: rgba(39, 41, 48, 0.94);
        background-image: none;
        color: #f4f4f6;
        border: 1px solid rgba(255, 255, 255, 0.10);
        border-radius: 13px;
        padding: 8px 14px;
        font-weight: 600;
        transition: all 160ms ease;
    }

    dropdown > button:hover,
    combobox > box > button:hover {
        background-color: rgba(48, 51, 59, 0.98);
        border-color: rgba(255, 255, 255, 0.20);
    }

    popover,
    popover > contents {
        background-color: #1c1e24;
        color: #f4f4f6;
        border: 1px solid rgba(255, 255, 255, 0.12);
        border-radius: 16px;
        padding: 6px;
        box-shadow: 0 16px 38px rgba(0, 0, 0, 0.65);
    }

    popover listview,
    popover box {
        background-color: transparent;
        color: #f4f4f6;
    }

    popover row,
    popover modelbutton {
        border-radius: 10px;
        padding: 7px 12px;
        color: #e4e4e7;
    }

    popover row:selected,
    popover row:hover,
    popover modelbutton:hover {
        background-color: rgba(255, 255, 255, 0.11);
        color: #ffffff;
    }

    checkbutton {
        color: #e4e4e7;
        font-size: 13px;
        font-weight: 500;
    }

    checkbutton > label {
        color: #e4e4e7;
    }

    checkbutton > check {
        background-color: rgba(39, 41, 48, 0.92);
        background-image: none;
        border: 1.5px solid rgba(255, 255, 255, 0.24);
        border-radius: 6px;
        min-width: 18px;
        min-height: 18px;
        margin-right: 8px;
        color: #ffffff;
    }

    checkbutton:checked > check {
        background: #c83b22;
        border-color: #e05338;
        color: #ffffff;
    }

    progressbar > text {
        color: #d4d4d8;
        font-size: 12px;
        font-weight: 600;
        margin-bottom: 4px;
    }

    progressbar > trough {
        background-color: rgba(39, 41, 48, 0.90);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 999px;
        min-height: 10px;
    }

    progressbar > trough > progress {
        background: linear-gradient(90deg, #c83b22 0%, #e05338 100%);
        border-radius: 999px;
        min-height: 10px;
    }

    notebook,
    notebook > stack {
        background-color: transparent;
    }

    scrolledwindow,
    viewport {
        background-color: transparent;
    }

    scrollbar {
        background-color: transparent;
    }

    scrollbar slider {
        background-color: rgba(255, 255, 255, 0.18);
        border-radius: 999px;
        min-width: 6px;
        min-height: 28px;
    }

    scrollbar slider:hover {
        background-color: rgba(255, 255, 255, 0.32);
    }

    separator {
        background-color: rgba(255, 255, 255, 0.08);
        min-height: 1px;
    }
"#;

pub fn apply_living_flow_theme(extra_css: &str) {
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(true);
    }

    if let Some(display) = gdk::Display::default() {
        let icon_theme = IconTheme::for_display(&display);
        icon_theme.add_search_path("/usr/share/icons/hicolor");
        icon_theme.add_search_path("/usr/share/pixmaps");
        icon_theme.add_search_path("/usr/share/solarui/icons");
        icon_theme.add_search_path("/home/darkmorpheus/SolarUI/data/icons/living-flow");

        let combined = format!("{}\n{}", LIVING_FLOW_CORE_CSS, extra_css);
        let provider = CssProvider::new();
        provider.load_from_string(&combined);
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
    }
}

/// Decodes embedded PNG bytes directly via GTK4's native `gdk::Texture::from_bytes`
/// (never relies on external `gdk-pixbuf` loader plugins or `loaders.cache`).
pub fn load_png_emblem(png_bytes: &[u8], size: i32) -> Image {
    let bytes = glib::Bytes::from(png_bytes);
    if let Ok(texture) = gdk::Texture::from_bytes(&bytes) {
        let img = Image::from_paintable(Some(&texture));
        img.set_pixel_size(size);
        img.set_size_request(size, size);
        return img;
    }
    let fallback = Image::from_icon_name("blazeos");
    fallback.set_pixel_size(size);
    fallback
}

/// Decodes wide/non-square official brand logos with guaranteed `ContentFit::Contain`
/// aspect-ratio preservation (never stretches or squishes horizontal logos).
pub fn load_png_wide_logo(png_bytes: &[u8], width: i32, height: i32) -> gtk4::Picture {
    let pic = gtk4::Picture::new();
    pic.set_can_shrink(true);
    pic.set_content_fit(gtk4::ContentFit::Contain);
    pic.set_size_request(width, height);
    pic.set_halign(gtk4::Align::Center);
    pic.set_valign(gtk4::Align::Center);
    let bytes = glib::Bytes::from(png_bytes);
    if let Ok(texture) = gdk::Texture::from_bytes(&bytes) {
        pic.set_paintable(Some(&texture));
    }
    pic
}

/// Builds a draggable ColorOS 17 Living Flow top bar with app emblem, title, subtitle,
/// live Fluid Cloud dynamic capsule, and tactile minimize/close controls.
pub fn build_living_flow_topbar(
    window: &Window,
    emblem: Option<Image>,
    title_text: &str,
    subtitle_text: &str,
    fluid_cloud: &FluidCloud,
    on_close: Option<Box<dyn Fn()>>,
) -> WindowHandle {
    let handle = WindowHandle::new();
    let bar = GtkBox::new(Orientation::Horizontal, 14);
    bar.add_css_class("lf-topbar");

    if let Some(img) = emblem {
        img.set_valign(gtk4::Align::Center);
        bar.append(&img);
    }

    let titles = GtkBox::new(Orientation::Vertical, 1);
    titles.set_valign(gtk4::Align::Center);
    titles.set_hexpand(true);

    let t_lbl = Label::new(Some(title_text));
    t_lbl.add_css_class("lf-topbar-title");
    t_lbl.set_halign(gtk4::Align::Start);
    t_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    titles.append(&t_lbl);

    let s_lbl = Label::new(Some(subtitle_text));
    s_lbl.add_css_class("lf-topbar-subtitle");
    s_lbl.set_halign(gtk4::Align::Start);
    s_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    titles.append(&s_lbl);

    bar.append(&titles);
    bar.append(fluid_cloud.widget());

    let controls = GtkBox::new(Orientation::Horizontal, 6);
    controls.set_valign(gtk4::Align::Center);

    let min_btn = Button::with_label("—");
    min_btn.add_css_class("lf-win-btn");
    min_btn.set_tooltip_text(Some("Arka Plana Küçült"));
    let win_min = window.downgrade();
    min_btn.connect_clicked(move |_| {
        if let Some(w) = win_min.upgrade() {
            w.minimize();
        }
        let _ = Command::new("solar-shell").arg("minimize").spawn();
    });
    controls.append(&min_btn);

    let close_btn = Button::with_label("✕");
    close_btn.add_css_class("lf-win-btn");
    close_btn.add_css_class("lf-win-close");
    close_btn.set_tooltip_text(Some("Pencereyi Kapat"));
    let win_close = window.downgrade();
    let on_close_rc = on_close.map(Rc::new);
    close_btn.connect_clicked(move |_| {
        if let Some(cb) = &on_close_rc {
            cb();
        }
        if let Some(w) = win_close.upgrade() {
            w.close();
        }
    });
    controls.append(&close_btn);

    bar.append(&controls);
    handle.set_child(Some(&bar));
    handle
}
