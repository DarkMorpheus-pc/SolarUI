use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, Grid, Label, Orientation, ScrolledWindow, Window,
};
use solar_common::SolarConfig;

use crate::living_flow::{
    apply_living_flow_theme, build_living_flow_topbar, load_png_emblem, FluidCloud, FluidCloudTone,
};

const WELCOME_EMBLEM_PNG: &[u8] =
    include_bytes!("../../../data/icons/living-flow/solar-welcome-64.png");
const LOGO_PNG: &[u8] = include_bytes!("../../../data/icons/solar-logo-256.png");

const WELCOME_EXTRA_CSS: &str = r#"
    .welcome-body {
        padding: 18px 22px 20px 22px;
    }

    .welcome-hero {
        background: linear-gradient(135deg, rgba(38, 40, 48, 0.92) 0%, rgba(26, 27, 33, 0.96) 100%);
        border: 1px solid rgba(255, 255, 255, 0.09);
        border-radius: 18px;
        padding: 14px 20px;
        margin-bottom: 12px;
    }

    .welcome-title {
        color: #f5f5f7;
        font-size: 19px;
        font-weight: 700;
        letter-spacing: -0.2px;
    }

    .welcome-subtitle {
        color: #a1a1aa;
        font-size: 12.2px;
        font-weight: 500;
    }

    .shortcut-grid-card {
        background: linear-gradient(180deg, rgba(30, 32, 38, 0.90) 0%, rgba(23, 25, 30, 0.94) 100%);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 18px;
        padding: 14px 18px;
        margin-bottom: 14px;
    }

    .lf-keycap {
        background-color: rgba(255, 255, 255, 0.08);
        color: #f4f4f6;
        border: 1px solid rgba(255, 255, 255, 0.13);
        border-radius: 999px;
        padding: 4px 12px;
        font-size: 11.5px;
        font-weight: 700;
        box-shadow: 0 2px 6px rgba(0, 0, 0, 0.28);
    }

    .shortcut-desc {
        color: #d4d4d8;
        font-size: 12.6px;
        font-weight: 500;
        margin-left: 8px;
    }
"#;

pub fn launch_welcome_window() {
    glib::set_prgname(Some("solar-welcome"));
    glib::set_application_name("SolarUI Welcome");

    if let Err(err) = gtk4::init() {
        eprintln!("Failed to initialize GTK4: {}", err);
        return;
    }

    let main_loop = glib::MainLoop::new(None, false);
    build_welcome_ui(main_loop.clone());
    main_loop.run();
}

fn build_welcome_ui(main_loop: glib::MainLoop) {
    apply_living_flow_theme(WELCOME_EXTRA_CSS);

    let window = Window::builder()
        .title("SolarUI Desktop — Karşılama ve Kısayollar")
        .decorated(false)
        .default_width(750)
        .default_height(590)
        .resizable(false)
        .build();

    let loop_quit = main_loop.clone();
    window.connect_close_request(move |_| {
        loop_quit.quit();
        glib::Propagation::Proceed
    });

    let key_controller = gtk4::EventControllerKey::new();
    let win_weak_key = window.downgrade();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape || key == gdk::Key::Return || key == gdk::Key::KP_Enter {
            if let Some(win) = win_weak_key.upgrade() {
                win.close();
            }
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);

    let shell_box = GtkBox::new(Orientation::Vertical, 0);
    shell_box.add_css_class("lf-window-shell");

    let fluid_cloud = FluidCloud::new("Living Flow • Sistem Hazır");
    let top_emblem = load_png_emblem(WELCOME_EMBLEM_PNG, 34);
    let loop_top = main_loop.clone();
    let topbar = build_living_flow_topbar(
        &window,
        Some(top_emblem),
        "SolarUI Masaüstü Rehberi",
        "Blaze Universe • Living Flow Design",
        &fluid_cloud,
        Some(Box::new(move || loop_top.quit())),
    );
    shell_box.append(&topbar);

    let body = GtkBox::new(Orientation::Vertical, 0);
    body.add_css_class("welcome-body");
    body.set_vexpand(true);

    // Luminous Hero Banner
    let hero = GtkBox::new(Orientation::Horizontal, 14);
    hero.add_css_class("welcome-hero");
    let logo = load_png_emblem(LOGO_PNG, 48);
    logo.set_valign(gtk4::Align::Center);
    hero.append(&logo);

    let hero_txt = GtkBox::new(Orientation::Vertical, 2);
    hero_txt.set_valign(gtk4::Align::Center);
    let title = Label::new(Some("Blaze Universe & SolarUI Masaüstüne Hoş Geldiniz"));
    title.add_css_class("welcome-title");
    title.set_halign(gtk4::Align::Start);
    hero_txt.append(&title);

    let subtitle = Label::new(Some(
        "Luminous Akış • Elastik Pencere Fiziği • Portadora & Flathub Hibrit Ekosistem",
    ));
    subtitle.add_css_class("welcome-subtitle");
    subtitle.set_halign(gtk4::Align::Start);
    hero_txt.append(&subtitle);
    hero.append(&hero_txt);
    body.append(&hero);

    // Scrollable Shortcut Grid inside Liquid-Acrylic Card
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .build();
    scroll.add_css_class("shortcut-grid-card");

    let grid = Grid::new();
    grid.set_column_spacing(20);
    grid.set_row_spacing(7);

    let shortcuts = [
        ("Mod + Space", "Solar Omnibar — Uygulama, Matematik, Kur & Sistem Arama"),
        ("Mod + G", "Blaze GameZone — Tam Ekran Konsol & Proton-GE Kabuğu"),
        ("Mod + Shift + S", "SolarUI Masaüstü, Kabuk Motoru ve Görev Çubuğu Ayarları"),
        ("Mod + S", "Kontrol Merkezi, Ses Mikseri ve Ağ Paneli"),
        ("Mod + Enter", "Alacritty GPU Hızlandırmalı Uçbirim"),
        ("Mod + T / F", "Serbest Yüzen Pencere (Floating) Moduna Geçiş"),
        ("Mod + Sol / Sağ Tık", "Pencereyi Taşı / Boyutlandır (Elastik Yay Fiziği)"),
        ("Mod + Y / Ctrl+Aşağı", "Pencereyi Görev Çubuğuna Küçült (Minimize)"),
        ("Alt + F4 / Mod + Q", "Aktif Pencereyi Kapat"),
        ("Mod + Alt + K", "SolarEvolution Retro Modu (Konami Kodu Sürprizi)"),
        ("Mod + Shift + Q", "Oturumu Kapat / Güç Menüsü"),
    ];

    for (i, (key, desc)) in shortcuts.iter().enumerate() {
        let row = i as i32;
        let key_label = Label::new(Some(key));
        key_label.add_css_class("lf-keycap");
        key_label.set_halign(gtk4::Align::Start);

        let desc_label = Label::new(Some(desc));
        desc_label.add_css_class("shortcut-desc");
        desc_label.set_halign(gtk4::Align::Start);
        desc_label.set_hexpand(true);
        desc_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        grid.attach(&key_label, 0, row, 1, 1);
        grid.attach(&desc_label, 1, row, 1, 1);
    }
    scroll.set_child(Some(&grid));
    body.append(&scroll);

    // Bottom Capsule Action Row
    let bottom_box = GtkBox::new(Orientation::Horizontal, 10);
    bottom_box.set_valign(gtk4::Align::Center);

    let cfg = SolarConfig::load();
    let check = CheckButton::with_label("Açılışta göster");
    check.set_active(cfg.shortcuts_hud.show_at_startup);
    check.set_hexpand(true);
    let cloud_chk = fluid_cloud.clone();
    check.connect_toggled(move |c| {
        let mut updated = SolarConfig::load();
        updated.shortcuts_hud.show_at_startup = c.is_active();
        let _ = updated.save();
        if c.is_active() {
            cloud_chk.pulse("Açılış karşılama kartı etkin", FluidCloudTone::Success, false);
        } else {
            cloud_chk.pulse("Açılış karşılama kartı gizlendi", FluidCloudTone::Info, false);
        }
    });
    bottom_box.append(&check);

    let has_solar_installer = std::path::Path::new("/usr/bin/solar-installer").exists();
    if has_solar_installer {
        let inst_btn = Button::with_label("Sistemi Kur");
        inst_btn.add_css_class("lf-btn-glass");
        let cloud_inst = fluid_cloud.clone();
        inst_btn.connect_clicked(move |_| {
            cloud_inst.pulse("Solar Kurulum Sihirbazı açılıyor...", FluidCloudTone::Info, false);
            let _ = std::process::Command::new("solar-shell")
                .arg("installer")
                .spawn();
        });
        bottom_box.append(&inst_btn);
    }

    let store_btn = Button::with_label("FireHub Mağaza");
    store_btn.add_css_class("lf-btn-emerald");
    let cloud_store = fluid_cloud.clone();
    store_btn.connect_clicked(move |_| {
        cloud_store.pulse("FireHub Mağaza başlatıldı", FluidCloudTone::Success, false);
        let _ = std::process::Command::new("firehub").spawn();
    });
    bottom_box.append(&store_btn);

    let start_btn = Button::with_label("Masaüstüne Başla →");
    start_btn.add_css_class("lf-btn-primary");
    let win_weak = window.downgrade();
    let loop_clone = main_loop.clone();
    start_btn.connect_clicked(move |_| {
        if let Some(win) = win_weak.upgrade() {
            win.close();
        }
        loop_clone.quit();
    });
    bottom_box.append(&start_btn);

    body.append(&bottom_box);
    shell_box.append(&body);

    window.set_child(Some(&shell_box));
    start_btn.set_can_focus(true);
    window.present();
    start_btn.grab_focus();
}
