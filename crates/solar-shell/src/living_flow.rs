// ==============================================================================
// ColorOS 17 "Living Flow Design" System for Blaze Universe & SolarUI
// Three Core Pillars:
//   1. Luminous Effect  — Liquid-acrylic translucent cards, Contour Glow borders,
//                         crisp high-contrast AMOLED typography, zero white leakage.
//   2. Fluid Motion     — Smooth cubic-bezier & GTK4 Stack slide/crossfade transitions.
//   3. Flexible Feedback— Capsule/pill tactile buttons with elastic press physics
//                         and live Fluid Cloud contextual system reaction capsule.
// ==============================================================================

use gdk_pixbuf::PixbufLoader;
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

        let dot_lbl = Label::new(Some("✦"));
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

    /// Triggers a live ColorOS 17 Fluid Cloud morphing reaction in the window topbar,
    /// plays subtle tactile acoustic feedback, and optionally dispatches an OSD toast.
    pub fn pulse(&self, message: &str, tone: FluidCloudTone, send_osd: bool) {
        let _ = crate::delight::play_acoustic_feedback("click");

        for cls in ["cloud-info", "cloud-ok", "cloud-warn", "cloud-danger", "cloud-pulse"] {
            self.pill_box.remove_css_class(cls);
        }

        let (tone_cls, symbol) = match tone {
            FluidCloudTone::Info => ("cloud-info", "✦"),
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
        glib::timeout_add_local_once(std::time::Duration::from_millis(420), move || {
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
                        "Blaze Universe • Fluid Cloud",
                        &msg_owned,
                    ])
                    .spawn();
            });
        }
    }
}

pub const LIVING_FLOW_CORE_CSS: &str = r#"
    @keyframes lf-glow-pulse {
        0%   { transform: scale(1.0); }
        45%  { transform: scale(1.035); }
        100% { transform: scale(1.0); }
    }

    window,
    window.background,
    window.csd {
        background-color: #0b0f1a;
        color: #f8fafc;
    }

    headerbar {
        background: linear-gradient(180deg, rgba(15, 23, 42, 0.96), rgba(11, 15, 26, 0.96));
        background-image: none;
        color: #f8fafc;
        border-bottom: 1px solid rgba(255, 255, 255, 0.10);
        box-shadow: none;
    }

    headerbar title,
    headerbar .title {
        color: #f8fafc;
        font-weight: 700;
    }

    .lf-window-shell {
        background: linear-gradient(155deg, #090d16 0%, #0f172a 52%, #131c31 100%);
        border: 1px solid rgba(255, 255, 255, 0.14);
        border-radius: 20px;
    }

    .lf-topbar {
        background: linear-gradient(180deg, rgba(22, 31, 50, 0.88) 0%, rgba(15, 22, 38, 0.82) 100%);
        border-bottom: 1px solid rgba(255, 255, 255, 0.09);
        border-top-left-radius: 20px;
        border-top-right-radius: 20px;
        padding: 12px 18px;
    }

    .lf-topbar-title {
        color: #f8fafc;
        font-size: 16px;
        font-weight: 800;
    }

    .lf-topbar-subtitle {
        color: #7dd3fc;
        font-size: 11.5px;
        font-weight: 600;
    }

    /* Fluid Cloud Live Dynamic Capsule */
    .lf-fluid-cloud {
        background-color: rgba(15, 23, 42, 0.90);
        border: 1px solid rgba(56, 189, 248, 0.38);
        border-radius: 999px;
        padding: 5px 14px;
        box-shadow: 0 4px 14px rgba(2, 6, 23, 0.45);
        transition: all 240ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-fluid-cloud.cloud-pulse {
        animation: lf-glow-pulse 360ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-fluid-cloud.cloud-info {
        border-color: rgba(56, 189, 248, 0.45);
        background-color: rgba(14, 116, 144, 0.22);
    }
    .lf-fluid-cloud.cloud-info .lf-cloud-dot { color: #38bdf8; }

    .lf-fluid-cloud.cloud-ok {
        border-color: rgba(52, 211, 153, 0.50);
        background-color: rgba(6, 95, 70, 0.28);
    }
    .lf-fluid-cloud.cloud-ok .lf-cloud-dot { color: #34d399; }

    .lf-fluid-cloud.cloud-warn {
        border-color: rgba(251, 191, 36, 0.52);
        background-color: rgba(146, 64, 14, 0.28);
    }
    .lf-fluid-cloud.cloud-warn .lf-cloud-dot { color: #fbbf24; }

    .lf-fluid-cloud.cloud-danger {
        border-color: rgba(248, 113, 113, 0.55);
        background-color: rgba(153, 27, 27, 0.30);
    }
    .lf-fluid-cloud.cloud-danger .lf-cloud-dot { color: #f87171; }

    .lf-cloud-dot {
        font-size: 12px;
        font-weight: 800;
    }

    .lf-cloud-text {
        color: #e2e8f0;
        font-size: 12px;
        font-weight: 600;
    }

    /* Window Controls (Minimize / Close Capsules) */
    .lf-win-btn {
        background-color: rgba(255, 255, 255, 0.08);
        background-image: none;
        color: #cbd5e1;
        border: 1px solid rgba(255, 255, 255, 0.14);
        border-radius: 999px;
        min-width: 30px;
        min-height: 30px;
        padding: 0 10px;
        font-size: 12px;
        font-weight: 700;
        transition: all 180ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-win-btn:hover {
        background-color: rgba(255, 255, 255, 0.18);
        background-image: none;
        color: #ffffff;
        transform: scale(1.05);
    }

    .lf-win-btn:active {
        transform: scale(0.93);
    }

    .lf-win-close:hover {
        background-color: rgba(239, 68, 68, 0.85);
        background-image: none;
        border-color: rgba(252, 165, 165, 0.65);
        color: #ffffff;
    }

    /* Floating Luminous Cards (Liquid Acrylic + Contour Glow) */
    .lf-card,
    .section-box,
    .installer-card {
        background: linear-gradient(180deg, rgba(30, 41, 64, 0.80) 0%, rgba(19, 27, 45, 0.86) 100%);
        border: 1px solid rgba(255, 255, 255, 0.12);
        border-radius: 20px;
        padding: 18px 20px;
        margin-bottom: 14px;
        box-shadow: 0 10px 26px rgba(2, 6, 23, 0.34);
        transition: border-color 220ms cubic-bezier(0.22, 1, 0.36, 1),
                    background-color 220ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-card:hover,
    .section-box:hover,
    .installer-card:hover {
        border-color: rgba(125, 211, 252, 0.30);
    }

    /* Typography Hierarchy (Strict High-Contrast Legibility) */
    .lf-section-title,
    .section-title,
    .section-heading {
        color: #f8fafc;
        font-size: 15.5px;
        font-weight: 700;
        margin-bottom: 6px;
    }

    .lf-label,
    .setting-label {
        color: #f1f5f9;
        font-size: 13.5px;
        font-weight: 600;
    }

    .lf-subtext,
    .setting-subtext,
    .section-desc {
        color: #cbd5e1;
        font-size: 12px;
        font-weight: 400;
    }

    /* Reset GTK4 Adwaita default white button gradients globally */
    button {
        background-image: none;
        text-shadow: none;
        -gtk-icon-shadow: none;
    }

    /* ColorOS 17 Capsule Buttons with Elastic Feedback */
    .lf-btn-primary,
    .action-btn,
    .start-btn {
        background: linear-gradient(135deg, #0ea5e9 0%, #2563eb 100%);
        color: #ffffff;
        border: 1px solid rgba(186, 230, 253, 0.40);
        border-radius: 999px;
        padding: 9px 22px;
        font-size: 13px;
        font-weight: 700;
        box-shadow: 0 6px 18px rgba(14, 165, 233, 0.30);
        transition: all 190ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-btn-primary:hover,
    .action-btn:hover,
    .start-btn:hover {
        background: linear-gradient(135deg, #38bdf8 0%, #3b82f6 100%);
        border-color: rgba(224, 242, 254, 0.68);
        box-shadow: 0 8px 22px rgba(56, 189, 248, 0.42);
        transform: scale(1.02);
    }

    .lf-btn-primary:active,
    .action-btn:active,
    .start-btn:active {
        transform: scale(0.96);
    }

    .lf-btn-flame,
    .primary-btn {
        background: linear-gradient(135deg, #f97316 0%, #ea580c 100%);
        color: #ffffff;
        border: 1px solid rgba(254, 215, 170, 0.42);
        border-radius: 999px;
        padding: 10px 24px;
        font-size: 13.5px;
        font-weight: 800;
        box-shadow: 0 6px 18px rgba(249, 115, 22, 0.32);
        transition: all 190ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-btn-flame:hover,
    .primary-btn:hover {
        background: linear-gradient(135deg, #fb923c 0%, #f97316 100%);
        border-color: rgba(255, 237, 213, 0.72);
        box-shadow: 0 8px 24px rgba(251, 146, 60, 0.46);
        transform: scale(1.02);
    }

    .lf-btn-flame:active,
    .primary-btn:active {
        transform: scale(0.96);
    }

    .lf-btn-emerald,
    .install-btn {
        background: linear-gradient(135deg, #10b981 0%, #059669 100%);
        color: #ffffff;
        border: 1px solid rgba(167, 243, 208, 0.40);
        border-radius: 999px;
        padding: 9px 22px;
        font-size: 13px;
        font-weight: 700;
        box-shadow: 0 6px 18px rgba(16, 185, 129, 0.28);
        transition: all 190ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-btn-emerald:hover,
    .install-btn:hover {
        background: linear-gradient(135deg, #34d399 0%, #10b981 100%);
        transform: scale(1.02);
    }

    .lf-btn-emerald:active,
    .install-btn:active {
        transform: scale(0.96);
    }

    .lf-btn-glass,
    .secondary-btn {
        background-color: rgba(30, 41, 59, 0.88);
        background-image: none;
        color: #f8fafc;
        border: 1px solid rgba(148, 163, 184, 0.34);
        border-radius: 999px;
        padding: 9px 18px;
        font-size: 13px;
        font-weight: 600;
        transition: all 190ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-btn-glass:hover,
    .secondary-btn:hover {
        background-color: rgba(51, 65, 85, 0.95);
        background-image: none;
        color: #ffffff;
        border-color: rgba(125, 211, 252, 0.55);
        transform: scale(1.02);
    }

    .lf-btn-glass:active,
    .secondary-btn:active {
        transform: scale(0.96);
    }

    .lf-btn-danger,
    .danger-btn {
        background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%);
        color: #ffffff;
        border: 1px solid rgba(252, 165, 165, 0.42);
        border-radius: 999px;
        padding: 9px 22px;
        font-size: 13px;
        font-weight: 700;
        box-shadow: 0 6px 18px rgba(239, 68, 68, 0.30);
        transition: all 190ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-btn-danger:hover,
    .danger-btn:hover {
        background: linear-gradient(135deg, #f87171 0%, #ef4444 100%);
        transform: scale(1.02);
    }

    .lf-btn-danger:active,
    .danger-btn:active {
        transform: scale(0.96);
    }

    /* Sidebar Navigation Pills & Stepper Capsules */
    .lf-sidebar {
        background-color: rgba(15, 23, 42, 0.62);
        border-right: 1px solid rgba(255, 255, 255, 0.08);
        padding: 14px 10px;
    }

    .lf-nav-pill {
        background-color: transparent;
        background-image: none;
        color: #cbd5e1;
        border: 1px solid transparent;
        border-radius: 14px;
        padding: 10px 14px;
        margin-bottom: 4px;
        font-size: 13px;
        font-weight: 600;
        transition: all 190ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-nav-pill:hover {
        background-color: rgba(255, 255, 255, 0.07);
        color: #f8fafc;
    }

    .lf-nav-pill.active {
        background: linear-gradient(135deg, rgba(14, 165, 233, 0.26) 0%, rgba(37, 99, 235, 0.32) 100%);
        color: #ffffff;
        border: 1px solid rgba(125, 211, 252, 0.45);
        box-shadow: 0 4px 14px rgba(14, 165, 233, 0.18);
    }

    .lf-step-pill {
        background-color: rgba(30, 41, 59, 0.68);
        background-image: none;
        color: #94a3b8;
        border: 1px solid rgba(255, 255, 255, 0.10);
        border-radius: 999px;
        padding: 8px 18px;
        font-size: 13px;
        font-weight: 700;
        transition: all 200ms cubic-bezier(0.22, 1, 0.36, 1);
    }

    .lf-step-pill:hover {
        background-color: rgba(51, 65, 85, 0.78);
        color: #f1f5f9;
    }

    .lf-step-pill.active {
        background: linear-gradient(135deg, rgba(249, 115, 22, 0.28) 0%, rgba(234, 88, 12, 0.36) 100%);
        color: #ffedd5;
        border-color: rgba(251, 146, 60, 0.62);
        box-shadow: 0 4px 16px rgba(249, 115, 22, 0.24);
    }

    /* Status Badges */
    .status-badge-ok,
    .disk-badge-ok {
        color: #6ee7b7;
        background-color: rgba(16, 185, 129, 0.18);
        border: 1px solid rgba(52, 211, 153, 0.38);
        border-radius: 999px;
        padding: 4px 12px;
        font-size: 11.5px;
        font-weight: 700;
    }

    .status-badge-info {
        color: #7dd3fc;
        background-color: rgba(14, 165, 233, 0.18);
        border: 1px solid rgba(56, 189, 248, 0.38);
        border-radius: 999px;
        padding: 4px 12px;
        font-size: 11.5px;
        font-weight: 700;
    }

    .disk-badge-warn {
        color: #fde68a;
        background-color: rgba(245, 158, 11, 0.18);
        border: 1px solid rgba(251, 191, 36, 0.40);
        border-radius: 14px;
        padding: 8px 14px;
        font-size: 12px;
        font-weight: 600;
    }

    /* Dark Liquid-Acrylic Form Widgets (DropDown, ComboBox, Entry, Popover, CheckButton) */
    entry,
    passwordentry,
    passwordentry > text,
    spinbutton {
        background-color: rgba(15, 23, 42, 0.85);
        background-image: none;
        color: #f8fafc;
        border: 1px solid rgba(148, 163, 184, 0.28);
        border-radius: 14px;
        padding: 8px 14px;
        min-height: 22px;
        caret-color: #38bdf8;
        transition: all 180ms ease;
    }

    entry:focus,
    passwordentry:focus-within {
        border-color: #38bdf8;
        box-shadow: 0 0 0 2px rgba(56, 189, 248, 0.22);
    }

    dropdown > button,
    combobox > box > button {
        background-color: rgba(15, 23, 42, 0.88);
        background-image: none;
        color: #f8fafc;
        border: 1px solid rgba(148, 163, 184, 0.32);
        border-radius: 14px;
        padding: 8px 14px;
        font-weight: 600;
        transition: all 180ms ease;
    }

    dropdown > button:hover,
    combobox > box > button:hover {
        background-color: rgba(30, 41, 59, 0.96);
        border-color: rgba(125, 211, 252, 0.52);
    }

    popover,
    popover > contents {
        background-color: rgba(15, 23, 42, 0.97);
        color: #f8fafc;
        border: 1px solid rgba(148, 163, 184, 0.32);
        border-radius: 16px;
        padding: 6px;
        box-shadow: 0 16px 38px rgba(2, 6, 23, 0.72);
    }

    popover listview,
    popover box {
        background-color: transparent;
        color: #f8fafc;
    }

    popover row,
    popover modelbutton {
        border-radius: 10px;
        padding: 7px 12px;
        color: #e2e8f0;
    }

    popover row:selected,
    popover row:hover,
    popover modelbutton:hover {
        background-color: rgba(14, 165, 233, 0.28);
        color: #ffffff;
    }

    checkbutton {
        color: #e2e8f0;
        font-size: 13px;
        font-weight: 500;
    }

    checkbutton > label {
        color: #e2e8f0;
    }

    checkbutton > check {
        background-color: rgba(15, 23, 42, 0.90);
        background-image: none;
        border: 1.5px solid rgba(148, 163, 184, 0.48);
        border-radius: 6px;
        min-width: 18px;
        min-height: 18px;
        margin-right: 8px;
        color: #ffffff;
    }

    checkbutton:checked > check {
        background: linear-gradient(135deg, #0ea5e9, #2563eb);
        border-color: #7dd3fc;
        color: #ffffff;
    }

    progressbar > text {
        color: #cbd5e1;
        font-size: 12px;
        font-weight: 600;
        margin-bottom: 4px;
    }

    progressbar > trough {
        background-color: rgba(15, 23, 42, 0.85);
        border: 1px solid rgba(255, 255, 255, 0.10);
        border-radius: 999px;
        min-height: 12px;
    }

    progressbar > trough > progress {
        background: linear-gradient(90deg, #0ea5e9 0%, #38bdf8 50%, #34d399 100%);
        border-radius: 999px;
        min-height: 12px;
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
        background-color: rgba(148, 163, 184, 0.28);
        border-radius: 999px;
        min-width: 6px;
        min-height: 28px;
    }

    scrollbar slider:hover {
        background-color: rgba(125, 211, 252, 0.52);
    }

    separator {
        background-color: rgba(255, 255, 255, 0.10);
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

pub fn load_png_emblem(png_bytes: &[u8], size: i32) -> Image {
    let loader = match PixbufLoader::with_type("png") {
        Ok(l) => l,
        Err(_) => return Image::new(),
    };
    loader.set_size(size, size);
    if loader.write(png_bytes).is_ok() && loader.close().is_ok() {
        if let Some(pixbuf) = loader.pixbuf() {
            let texture = gdk4::Texture::for_pixbuf(&pixbuf);
            let img = Image::from_paintable(Some(&texture));
            img.set_pixel_size(size);
            return img;
        }
    }
    Image::new()
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
