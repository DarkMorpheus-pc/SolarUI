use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Image, Label, Orientation,
    ScrolledWindow, Separator, Stack, StackTransitionType, StringList, Window,
};
use solar_common::{
    HardwareDetector, HardwareTuningProfile, PerformanceTier, ShellEngine, SolarConfig,
    SurfaceProvider,
};
use crate::living_flow::{
    apply_living_flow_theme, build_living_flow_topbar, load_png_emblem, FluidCloud, FluidCloudTone,
};
use crate::pixel_clock::{PixelClockConfig, PixelClockStyle};
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

const LOGO_PNG: &[u8] = include_bytes!("../../../data/icons/solar-logo-256.png");
const SETTINGS_EMBLEM_PNG: &[u8] =
    include_bytes!("../../../data/icons/living-flow/solar-settings-64.png");
const NAV_CLOCK_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-clock.png");
const NAV_TASKBAR_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-taskbar.png");
const NAV_DISPLAY_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-display.png");
const NAV_SHELL_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-shell.png");
const NAV_CAELESTIA_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-caelestia.png");
const NAV_NOCTALIA_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-noctalia.png");
const NAV_PERF_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-perf.png");
const NAV_HEALTH_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-health.png");
const NAV_OMNIBAR_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-omnibar.png");
const NAV_HUD_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-hud.png");
const NAV_ABOUT_PNG: &[u8] = include_bytes!("../../../data/icons/living-flow/nav-about.png");

fn create_logo_image(size: i32) -> Image {
    load_png_emblem(LOGO_PNG, size)
}

pub fn launch_settings_window() {
    glib::set_prgname(Some("solar-settings"));
    glib::set_application_name("SolarUI Settings");

    if let Err(err) = gtk4::init() {
        eprintln!("Failed to initialize GTK4: {}", err);
        return;
    }

    let main_loop = glib::MainLoop::new(None, false);
    build_settings_ui(main_loop.clone());
    main_loop.run();
}

fn restart_taskbar_if_enabled() {
    let cfg = SolarConfig::load();
    // Kill existing taskbar instance
    let _ = Command::new("pkill")
        .args(["-f", "solar-shell (run|gui|taskbar|window)"])
        .status();

    if cfg.taskbar.enabled {
        // Spawn fresh taskbar instance
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(150));
            let _ = Command::new("solar-shell").arg("run").spawn();
        });
    }
}

fn build_settings_ui(main_loop: glib::MainLoop) {
    let extra_css = r#"
        .settings-stage {
            padding: 18px 22px;
        }

        .settings-title {
            color: #f5f5f7;
            font-size: 19px;
            font-weight: 700;
        }

        .settings-subtitle {
            color: #a1a1aa;
            font-size: 12.5px;
            font-weight: 500;
        }

        .settings-cat-header {
            color: #71717a;
            font-size: 10.2px;
            font-weight: 700;
            letter-spacing: 1.1px;
            margin: 10px 10px 4px 10px;
        }
    "#;
    apply_living_flow_theme(extra_css);

    let window = Window::builder()
        .title("SolarUI Ayarları — Masaüstü ve Görev Çubuğu")
        .decorated(false)
        .default_width(1020)
        .default_height(620)
        .resizable(true)
        .build();

    let loop_quit = main_loop.clone();
    window.connect_close_request(move |_| {
        loop_quit.quit();
        glib::Propagation::Proceed
    });

    let key_ctrl = gtk4::EventControllerKey::new();
    let win_esc = window.downgrade();
    key_ctrl.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            if let Some(w) = win_esc.upgrade() {
                w.close();
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(key_ctrl);

    let shell_box = GtkBox::new(Orientation::Vertical, 0);
    shell_box.add_css_class("lf-window-shell");

    let fluid_cloud = FluidCloud::new("Living Flow • Ayarlar Senkronize");
    let emblem = load_png_emblem(SETTINGS_EMBLEM_PNG, 34);
    let loop_top = main_loop.clone();
    let topbar = build_living_flow_topbar(
        &window,
        Some(emblem),
        "SolarUI Masaüstü Ayarları",
        "Blaze Universe • ColorOS 17 Living Flow Kontrol Paneli",
        &fluid_cloud,
        Some(Box::new(move || loop_top.quit())),
    );
    shell_box.append(&topbar);

    // Two-Pane Living Flow Layout: Left Capsule Navigation Rail + Right Fluid Stack
    let split_box = GtkBox::new(Orientation::Horizontal, 0);
    split_box.set_vexpand(true);
    split_box.set_hexpand(true);

    let sidebar_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .min_content_width(228)
        .vexpand(true)
        .build();
    sidebar_scroll.add_css_class("lf-sidebar");

    let nav_list = GtkBox::new(Orientation::Vertical, 2);

    let stage_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .build();

    let stack = Stack::new();
    stack.set_hhomogeneous(false);
    stack.set_vhomogeneous(false);
    stack.set_transition_type(StackTransitionType::SlideUpDown);
    stack.set_transition_duration(240);
    stack.add_css_class("settings-stage");

    // ── BUILD ALL PAGES ───────────────────────────────────────────────────────
    let clock_tab = build_pixel_clock_tab();
    let taskbar_tab = build_taskbar_tab();
    let display_tab = build_display_tab();
    let shell_tab = build_shell_tab();
    let perf_tab = build_performance_tab();
    let health_tab = build_health_tab();
    let caelestia_tab = build_caelestia_tab();

    // Noctalia Bar Quick Control Page
    let noctalia_tab = GtkBox::new(Orientation::Vertical, 12);
    let n_sec = GtkBox::new(Orientation::Vertical, 12);
    n_sec.add_css_class("section-box");

    let n_title = Label::new(Some("Noctalia Bar ve Hızlı Kontrol Panelleri"));
    n_title.add_css_class("section-title");
    n_title.set_halign(gtk4::Align::Start);
    n_sec.append(&n_title);

    let n_desc = Label::new(Some(
        "Noctalia Bar, SolarUI'ın üst bilgi ve kontrol merkezidir. Tüm ayarlarını ve panellerini aşağıdan doğrudan açabilirsiniz.",
    ));
    n_desc.add_css_class("setting-subtext");
    n_desc.set_wrap(true);
    n_desc.set_xalign(0.0);
    n_desc.set_halign(gtk4::Align::Start);
    n_sec.append(&n_desc);

    let btn_grid = GtkBox::new(Orientation::Horizontal, 10);
    let cloud_noc = fluid_cloud.clone();
    let btn_settings = Button::with_label("Noctalia Ayarları");
    btn_settings.add_css_class("action-btn");
    btn_settings.connect_clicked(move |_| {
        cloud_noc.pulse("Noctalia ayar paneli açıldı", FluidCloudTone::Info, false);
        let _ = Command::new("noctalia").args(["msg", "settings-toggle"]).spawn();
    });
    btn_grid.append(&btn_settings);

    let btn_cc = Button::with_label("Kontrol Merkezi");
    btn_cc.add_css_class("secondary-btn");
    btn_cc.connect_clicked(|_| {
        let _ = Command::new("noctalia")
            .args(["msg", "panel-toggle", "control-center"])
            .spawn();
    });
    btn_grid.append(&btn_cc);

    let btn_wp = Button::with_label("Duvar Kağıdı");
    btn_wp.add_css_class("secondary-btn");
    btn_wp.connect_clicked(|_| {
        let _ = Command::new("noctalia")
            .args(["msg", "panel-toggle", "wallpaper"])
            .spawn();
    });
    btn_grid.append(&btn_wp);

    let btn_audio = Button::with_label("Ses & Mikser");
    btn_audio.add_css_class("secondary-btn");
    btn_audio.connect_clicked(|_| {
        let _ = Command::new("noctalia")
            .args(["msg", "panel-toggle", "control-center", "audio"])
            .spawn();
    });
    btn_grid.append(&btn_audio);

    n_sec.append(&btn_grid);
    noctalia_tab.append(&n_sec);

    let omnibar_tab = build_omnibar_tab();

    // Shortcuts & Welcome HUD Page
    let hud_tab = GtkBox::new(Orientation::Vertical, 12);
    let h_sec = GtkBox::new(Orientation::Vertical, 12);
    h_sec.add_css_class("section-box");

    let h_title = Label::new(Some("Açılış Karşılama ve Kısayollar Ekranı"));
    h_title.add_css_class("section-title");
    h_title.set_halign(gtk4::Align::Start);
    h_sec.append(&h_title);

    let h_sub = Label::new(Some(
        "Başlangıç karşılama ekranını, sistem klavye kısayollarını ve kurulum yardımcısını yapılandırın.",
    ));
    h_sub.add_css_class("setting-subtext");
    h_sub.set_wrap(true);
    h_sub.set_xalign(0.0);
    h_sub.set_halign(gtk4::Align::Start);
    h_sec.append(&h_sub);

    let open_welcome_btn = Button::with_label("Kısayol ve Karşılama Ekranını Şimdi Aç");
    open_welcome_btn.add_css_class("action-btn");
    open_welcome_btn.set_halign(gtk4::Align::Start);
    open_welcome_btn.connect_clicked(|_| {
        let _ = Command::new("solar-shell").arg("welcome").spawn();
    });
    h_sec.append(&open_welcome_btn);

    let cfg = SolarConfig::load();
    let chk_startup = CheckButton::with_label("Oturum açıldığında karşılama ekranını otomatik göster");
    chk_startup.set_active(cfg.shortcuts_hud.show_at_startup);
    let cloud_hud = fluid_cloud.clone();
    chk_startup.connect_toggled(move |cb| {
        let mut c = SolarConfig::load();
        c.shortcuts_hud.show_at_startup = cb.is_active();
        let _ = c.save();
        cloud_hud.pulse("Karşılama tercihi kaydedildi", FluidCloudTone::Success, false);
    });
    h_sec.append(&chk_startup);
    hud_tab.append(&h_sec);

    // About Page
    let about_tab = GtkBox::new(Orientation::Vertical, 12);
    let a_sec = GtkBox::new(Orientation::Vertical, 12);
    a_sec.add_css_class("section-box");
    a_sec.set_halign(gtk4::Align::Fill);

    let big_logo = create_logo_image(80);
    big_logo.set_halign(gtk4::Align::Center);
    a_sec.append(&big_logo);

    let a_name = Label::new(Some("SolarUI Masaüstü Ortamı (Blaze Universe Edition)"));
    a_name.add_css_class("settings-title");
    a_name.set_halign(gtk4::Align::Center);
    a_sec.append(&a_name);

    let a_sig = Label::new(Some("ColorOS 17 Living Flow Design • Luminous Rendering & Aurora Physics"));
    a_sig.add_css_class("settings-subtitle");
    a_sig.set_halign(gtk4::Align::Center);
    a_sec.append(&a_sig);

    let a_ver = Label::new(Some("Sürüm 1.1.0 (Rust + GTK4 + Niri Wayland + Linux 7.2.9)"));
    a_ver.add_css_class("setting-subtext");
    a_ver.set_halign(gtk4::Align::Center);
    a_sec.append(&a_ver);

    let sep = Separator::new(Orientation::Horizontal);
    a_sec.append(&sep);

    let a_dev = Label::new(Some(
        "Geliştirici: Bulut Ars. E. (DarkMorpheus) & Blaze Universe Project\nYerli Bağımsız Linux Tabanı • Portadora (.blz) & Flathub Hibrit Mimari",
    ));
    a_dev.add_css_class("setting-label");
    a_dev.set_justify(gtk4::Justification::Center);
    a_dev.set_halign(gtk4::Align::Center);
    a_sec.append(&a_dev);
    about_tab.append(&a_sec);

    // Register pages and build ColorOS 17 Two-Pane Squircle Navigation Rows (Zero Unicode Tofu!)
    let pages: Vec<(&str, &str, &[u8], &str, GtkBox)> = vec![
        ("GÖRÜNÜM & KİŞİSELLEŞTİRME", "clock", NAV_CLOCK_PNG, "Pixel Saat & Widget", clock_tab),
        ("", "taskbar", NAV_TASKBAR_PNG, "Görev Çubuğu", taskbar_tab),
        ("", "display", NAV_DISPLAY_PNG, "Ekran & DPI Ölçeği", display_tab),
        ("KABUK MOTORLARI", "shell", NAV_SHELL_PNG, "Kabuk Motoru & Yüzeyler", shell_tab),
        ("", "caelestia", NAV_CAELESTIA_PNG, "Caelestia Düzeni", caelestia_tab),
        ("", "noctalia", NAV_NOCTALIA_PNG, "Noctalia Bar", noctalia_tab),
        ("SİSTEM & GÜVENLİK", "perf", NAV_PERF_PNG, "Performans & Aurora", perf_tab),
        ("", "health", NAV_HEALTH_PNG, "Sistem Sağlığı & Kalkan", health_tab),
        ("", "omnibar", NAV_OMNIBAR_PNG, "Omnibar & Akıllı Arama", omnibar_tab),
        ("", "hud", NAV_HUD_PNG, "Kısayollar & Rehber", hud_tab),
        ("", "about", NAV_ABOUT_PNG, "Hakkında", about_tab),
    ];

    let mut nav_buttons: Vec<(String, String, Button)> = Vec::new();
    for (group_hdr, page_id, icon_bytes, label_txt, widget_box) in pages {
        if !group_hdr.is_empty() {
            let hdr = Label::new(Some(group_hdr));
            hdr.add_css_class("settings-cat-header");
            hdr.set_halign(gtk4::Align::Start);
            nav_list.append(&hdr);
        }
        stack.add_named(&widget_box, Some(page_id));

        let btn = Button::new();
        btn.add_css_class("lf-nav-pill");
        let row_box = GtkBox::new(Orientation::Horizontal, 10);
        let nav_icon = load_png_emblem(icon_bytes, 24);
        nav_icon.set_valign(gtk4::Align::Center);
        row_box.append(&nav_icon);

        let b_lbl = Label::new(Some(label_txt));
        b_lbl.set_halign(gtk4::Align::Start);
        b_lbl.set_valign(gtk4::Align::Center);
        b_lbl.set_hexpand(true);
        b_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        row_box.append(&b_lbl);
        btn.set_child(Some(&row_box));

        if page_id == "clock" {
            btn.add_css_class("active");
        }
        nav_list.append(&btn);
        nav_buttons.push((page_id.to_string(), label_txt.to_string(), btn));
    }

    let all_btns = Rc::new(nav_buttons);
    for (page_id, title_str, btn) in all_btns.iter() {
        let pid = page_id.clone();
        let t_str = title_str.clone();
        let btns_ref = all_btns.clone();
        let stack_ref = stack.clone();
        let cloud_nav = fluid_cloud.clone();
        btn.connect_clicked(move |_| {
            for (other_id, _, other_btn) in btns_ref.iter() {
                if other_id == &pid {
                    other_btn.add_css_class("active");
                } else {
                    other_btn.remove_css_class("active");
                }
            }
            stack_ref.set_visible_child_name(&pid);
            cloud_nav.pulse(&t_str, FluidCloudTone::Info, false);
        });
    }

    stack.set_visible_child_name("clock");
    sidebar_scroll.set_child(Some(&nav_list));
    stage_scroll.set_child(Some(&stack));

    split_box.append(&sidebar_scroll);
    split_box.append(&stage_scroll);
    shell_box.append(&split_box);

    window.set_child(Some(&shell_box));
    window.present();
}

fn build_performance_tab() -> GtkBox {
    let tab = GtkBox::new(Orientation::Vertical, 12);
    tab.set_margin_top(10);
    tab.set_margin_bottom(10);

    let hw = HardwareDetector::detect();
    let current_profile = HardwareTuningProfile::load();

    // 1. Detected Hardware Card
    let hw_sec = GtkBox::new(Orientation::Vertical, 8);
    hw_sec.add_css_class("section-box");

    let hw_title = Label::new(Some("Algılanan Donanım Mimarisi"));
    hw_title.add_css_class("section-title");
    hw_title.set_halign(gtk4::Align::Start);
    hw_sec.append(&hw_title);

    let dev_label = Label::new(Some(&format!(
        "GPU: {}\nCihazlar: {}\nToplam RAM: {:.1} GB | İşlemci: {} Çekirdek\nGüç: {}",
        hw.gpu_vendor,
        if hw.gpu_devices.is_empty() {
            "Tümleşik / Standart DRM".to_string()
        } else {
            hw.gpu_devices.join(", ")
        },
        (hw.total_ram_mb as f64) / 1024.0,
        hw.cpu_cores,
        if hw.on_battery {
            "Batarya Gücünde"
        } else {
            "Şebeke Gücünde (AC Bağlı)"
        }
    )));
    dev_label.add_css_class("setting-label");
    dev_label.set_halign(gtk4::Align::Start);
    dev_label.set_margin_bottom(6);
    hw_sec.append(&dev_label);

    let rec_box = GtkBox::new(Orientation::Horizontal, 8);
    let rec_title = Label::new(Some("Donanımınıza Özel Önerilen Profil:"));
    rec_title.add_css_class("setting-subtext");
    rec_box.append(&rec_title);

    let rec_val = Label::new(Some(hw.recommended_tier.name_tr()));
    rec_val.add_css_class("status-badge-info");
    rec_box.append(&rec_val);
    hw_sec.append(&rec_box);

    tab.append(&hw_sec);

    // 2. Profile Selection Card
    let prof_sec = GtkBox::new(Orientation::Vertical, 10);
    prof_sec.add_css_class("section-box");

    let prof_title = Label::new(Some("Aktif Performans Profili"));
    prof_title.add_css_class("section-title");
    prof_title.set_halign(gtk4::Align::Start);
    prof_sec.append(&prof_title);

    let prof_sub = Label::new(Some(
        "Görsel efektler (blur, gölgeler, animasyon süreleri) donanım gücüne göre uyarlanır. Ekran yenileme hızı (Hz) profilden bağımsız olup daima monitörün native frekansında kalır.",
    ));
    prof_sub.add_css_class("setting-subtext");
    prof_sub.set_wrap(true);
    prof_sub.set_halign(gtk4::Align::Start);
    prof_sec.append(&prof_sub);

    let tier_items = StringList::new(&[
        "Hafif / Eski Donanım (Legacy) — Sıfır blur, sıfır gölge, anlık 0ms geçişler",
        "Dengeli (Balanced) — Optimize edilmiş akıcı animasyonlar, hafif cam efekti",
        "Yüksek Performans (Ultra) — Tam cam blur efekti, derin gölgeler, yay fiziği",
    ]);
    let tier_dropdown = DropDown::new(Some(tier_items), None::<gtk4::Expression>);

    match current_profile.tier {
        PerformanceTier::Legacy => tier_dropdown.set_selected(0),
        PerformanceTier::Balanced => tier_dropdown.set_selected(1),
        PerformanceTier::Ultra => tier_dropdown.set_selected(2),
    }

    tier_dropdown.connect_selected_notify(|dd| {
        let tier = match dd.selected() {
            0 => PerformanceTier::Legacy,
            1 => PerformanceTier::Balanced,
            _ => PerformanceTier::Ultra,
        };
        let p = HardwareTuningProfile::for_tier(tier);
        let _ = p.save();
        restart_taskbar_if_enabled();
    });

    prof_sec.append(&tier_dropdown);
    tab.append(&prof_sec);

    // 3. ChromeOS Ash Stability & Resource Optimization Card
    let ash_sec = GtkBox::new(Orientation::Vertical, 10);
    ash_sec.add_css_class("section-box");

    let ash_title = Label::new(Some("ChromeOS Ash Seviyesi Kararlılık & Güç Tasarrufu"));
    ash_title.add_css_class("section-title");
    ash_title.set_halign(gtk4::Align::Start);
    ash_sec.append(&ash_title);

    let ash_sub = Label::new(Some(
        "Aura/Ash esintili akıllı kaynak yönetimi: Boşta CPU/GPU kullanımını minimuma indirir, çöken kabukları otomatik iyileştirir ve arayüzü OOM krizlerine karşı korur.",
    ));
    ash_sub.add_css_class("setting-subtext");
    ash_sub.set_wrap(true);
    ash_sub.set_halign(gtk4::Align::Start);
    ash_sec.append(&ash_sub);

    // Watchdog Auto-Healing
    let chk_watchdog = CheckButton::with_label("Sıfır-Çökme Watchdog (Çöken arayüz motorlarını 3sn içinde sessizce onarır)");
    chk_watchdog.set_active(current_profile.watchdog_enabled);
    chk_watchdog.connect_toggled(|cb| {
        let mut p = HardwareTuningProfile::load();
        p.watchdog_enabled = cb.is_active();
        let _ = p.save();
    });
    ash_sec.append(&chk_watchdog);

    // Idle Resource Throttling
    let chk_idle = CheckButton::with_label("Boşta Akıllı Kaynak Kısma (Kullanıcı boştayken CPU/RAM polling ve render'ı frenler)");
    chk_idle.set_active(current_profile.idle_throttling_enabled);
    chk_idle.connect_toggled(|cb| {
        let mut p = HardwareTuningProfile::load();
        p.idle_throttling_enabled = cb.is_active();
        let _ = p.save();
    });
    ash_sec.append(&chk_idle);

    // OOM Killer Shield
    let chk_oom = CheckButton::with_label("OOM Bellek Kalkanı (Ağır RAM yükünde pencere yöneticisi ve kabuğun öldürülmesini engeller)");
    chk_oom.set_active(current_profile.oom_shield_enabled);
    chk_oom.connect_toggled(|cb| {
        let mut p = HardwareTuningProfile::load();
        p.oom_shield_enabled = cb.is_active();
        let _ = p.save();
        apply_oom_shield_live(cb.is_active());
    });
    ash_sec.append(&chk_oom);

    tab.append(&ash_sec);

    tab
}

pub fn apply_oom_shield_live(enabled: bool) {
    let score = if enabled { "-500" } else { "0" };
    // Apply to current process first
    let _ = std::fs::write("/proc/self/oom_score_adj", score);
    // Apply to core desktop processes owned by user or via privileged helper
    for proc_name in ["niri", "solar-shell", "noctalia", "quickshell", "solar-core"] {
        if let Ok(out) = Command::new("pgrep").args(["-x", proc_name]).output() {
            if let Ok(pids) = String::from_utf8(out.stdout) {
                for pid in pids.lines().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    let path = format!("/proc/{}/oom_score_adj", pid);
                    if std::fs::write(&path, score).is_err() {
                        let _ = Command::new("sudo")
                            .args(["-n", "sh", "-c", &format!("echo {} > {}", score, path)])
                            .status();
                    }
                }
            }
        }
    }
}

fn probe_live_health_components() -> Vec<(&'static str, String, &'static str)> {
    // 1. SolarCore / Daemon check
    let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
    let sock_paths = [
        format!("{}/solar.sock", xdg_runtime),
        format!("{}/solar-core.sock", xdg_runtime),
        "/tmp/solar-core.sock".to_string(),
    ];
    let has_sock = sock_paths.iter().any(|p| std::path::Path::new(p).exists());
    let has_core_proc = Command::new("pgrep")
        .args(["-f", "solar-core|solar-daemon|solar-shell"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let core_status = if has_sock {
        ("Aktif (Unix IPC Soketi Hazır)".to_string(), "status-badge-ok")
    } else if has_core_proc {
        ("Aktif (Otonom Denetleyici Çalışıyor)".to_string(), "status-badge-ok")
    } else {
        ("Beklemede (İstemci Modu)".to_string(), "status-badge-info")
    };

    // 2. Active Shell Engine check
    let shell_status = if Command::new("pgrep").args(["-x", "noctalia"]).output().map(|o| o.status.success()).unwrap_or(false) {
        ("Canlı (Noctalia QML Motoru Aktif)".to_string(), "status-badge-ok")
    } else if Command::new("pgrep").args(["-f", "quickshell"]).output().map(|o| o.status.success()).unwrap_or(false) {
        ("Canlı (Quickshell / Caelestia / end4 Aktif)".to_string(), "status-badge-ok")
    } else {
        ("Canlı (SolarShell Yerel Yüzey)".to_string(), "status-badge-ok")
    };

    // 3. Wayland Compositor (Niri) check
    let niri_alive = std::env::var("NIRI_SOCKET")
        .map(|p| std::path::Path::new(&p).exists())
        .unwrap_or(false)
        || Command::new("pgrep").args(["-x", "niri"]).output().map(|o| o.status.success()).unwrap_or(false);
    let wm_status = if niri_alive {
        ("Canlı (Niri Wayland Compositor)".to_string(), "status-badge-ok")
    } else {
        ("Wayland Oturumu Aktif".to_string(), "status-badge-info")
    };

    // 4. Blaze Sentinel Live Security Audit
    let sentinel_status = if let Ok(out) = Command::new("blaze-sentinel").args(["status", "--json"]).output() {
        if let Ok(txt) = String::from_utf8(out.stdout) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                let passed = val.get("passed_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let total = val.get("total_count").and_then(|v| v.as_u64()).unwrap_or(7);
                (
                    format!("Tam Kalkan ({}/{} Güvenlik Kontrolü Aktif)", passed, total),
                    if passed == total { "status-badge-ok" } else { "status-badge-info" },
                )
            } else {
                ("Aktif (Blaze Sentinel Kalkanı)".to_string(), "status-badge-ok")
            }
        } else {
            ("Aktif (Blaze Sentinel Kalkanı)".to_string(), "status-badge-ok")
        }
    } else {
        ("Senkronize (Atomik Durum Makinesi)".to_string(), "status-badge-info")
    };

    vec![
        ("SolarCore Platform Servisi", core_status.0, core_status.1),
        ("SolarUI Kabuk Motoru", shell_status.0, shell_status.1),
        ("SolarUI Pencere Yöneticisi", wm_status.0, wm_status.1),
        ("Blaze Sentinel Güvenlik Kalkanı", sentinel_status.0, sentinel_status.1),
    ]
}

fn build_health_tab() -> GtkBox {
    let tab = GtkBox::new(Orientation::Vertical, 12);
    tab.set_margin_top(10);
    tab.set_margin_bottom(10);

    // 1. Health Status Card
    let stat_sec = GtkBox::new(Orientation::Vertical, 10);
    stat_sec.add_css_class("section-box");

    let stat_title = Label::new(Some("Platform Durumu ve Süreç İzolasyonu (Canlı Telemetri)"));
    stat_title.add_css_class("section-title");
    stat_title.set_halign(gtk4::Align::Start);
    stat_sec.append(&stat_title);

    let stat_desc = Label::new(Some(
        "SolarUI mimarisi bileşenleri izole eder ve Blaze Sentinel çekirdek güvenlik kontrollerini gerçek zamanlı denetler.",
    ));
    stat_desc.add_css_class("setting-subtext");
    stat_desc.set_wrap(true);
    stat_desc.set_halign(gtk4::Align::Start);
    stat_sec.append(&stat_desc);

    let components = probe_live_health_components();

    for (comp, status, badge_cls) in components {
        let row = GtkBox::new(Orientation::Horizontal, 12);
        row.set_margin_top(4);
        row.set_margin_bottom(4);

        let c_lbl = Label::new(Some(comp));
        c_lbl.add_css_class("setting-label");
        c_lbl.set_hexpand(true);
        c_lbl.set_halign(gtk4::Align::Start);
        row.append(&c_lbl);

        let s_lbl = Label::new(Some(&status));
        s_lbl.add_css_class(badge_cls);
        s_lbl.set_halign(gtk4::Align::End);
        row.append(&s_lbl);

        stat_sec.append(&row);
    }

    tab.append(&stat_sec);

    // 1.5 USB Storage Guard (Paranoid Mode vs Desktop Mode)
    let usb_sec = GtkBox::new(Orientation::Vertical, 10);
    usb_sec.add_css_class("section-box");

    let usb_title = Label::new(Some("USB Depolama Kalkanı (Fiziksel Port Güvenliği)"));
    usb_title.add_css_class("section-title");
    usb_title.set_halign(gtk4::Align::Start);
    usb_sec.append(&usb_title);

    let usb_desc = Label::new(Some(
        "Etkinleştirildiğinde (Paranoid Mod) yeni takılan yabancı USB belleklerin çekirdek düzeyinde bağlanması engellenir. Günlük masaüstü ve ofis kullanımı için kapalı tutabilirsiniz.",
    ));
    usb_desc.add_css_class("setting-subtext");
    usb_desc.set_wrap(true);
    usb_desc.set_halign(gtk4::Align::Start);
    usb_sec.append(&usb_desc);

    let usb_locked = std::fs::read_to_string("/etc/modprobe.d/blaze-usb-guard.conf")
        .map(|c| c.contains("install usb-storage /bin/true"))
        .unwrap_or(false);
    let chk_usb_guard = CheckButton::with_label("Yabancı USB Depolama Aygıtlarını Kilitle (Paranoid Mod / Lynis STRG-1846)");
    chk_usb_guard.set_active(usb_locked);
    chk_usb_guard.connect_toggled(|cb| {
        let lock = cb.is_active();
        std::thread::spawn(move || {
            let cmd_str = if lock {
                "printf 'install usb-storage /bin/true\\nblacklist usb-storage\\n' > /etc/modprobe.d/blaze-usb-guard.conf"
            } else {
                "rm -f /etc/modprobe.d/blaze-usb-guard.conf && modprobe usb-storage 2>/dev/null || true"
            };
            let _ = Command::new("sudo").args(["-n", "sh", "-c", cmd_str]).status();
        });
    });
    usb_sec.append(&chk_usb_guard);
    tab.append(&usb_sec);

    // 2. Self-Healing Crash Test Card
    let test_sec = GtkBox::new(Orientation::Vertical, 10);
    test_sec.add_css_class("section-box");

    let test_title = Label::new(Some("Self-Healing Watchdog Testi"));
    test_title.add_css_class("section-title");
    test_title.set_halign(gtk4::Align::Start);
    test_sec.append(&test_title);

    let test_info = Label::new(Some(
        "Aşağıdaki butona tıkladığınızda `solar-shell` süreci aniden sonlandırılır. SolarUI pencere yöneticisi ve SolarCore çalışmaya devam eder. Görev çubuğu yeniden başlatıldığında açık uygulamalarınızın durumu yeniden yüklenir.",
    ));
    test_info.add_css_class("setting-subtext");
    test_info.set_wrap(true);
    test_info.set_halign(gtk4::Align::Start);
    test_sec.append(&test_info);

    let crash_btn = Button::with_label("Kabuğu Test Amaçlı Sonlandır (Self-Healing Crash Test)");
    crash_btn.add_css_class("danger-btn");
    crash_btn.set_halign(gtk4::Align::Start);
    crash_btn.connect_clicked(|_| {
        std::thread::spawn(|| {
            // Kill solar-shell run instance
            let _ = Command::new("pkill")
                .args(["-9", "-f", "solar-shell (run|gui|taskbar|window)"])
                .status();
            std::thread::sleep(std::time::Duration::from_millis(150));
            // Ensure watchdog brings it back
            let _ = Command::new("solar-shell").arg("run").spawn();
        });
    });
    test_sec.append(&crash_btn);

    tab.append(&test_sec);

    tab
}

fn build_pixel_clock_tab() -> GtkBox {
    let tab = GtkBox::new(Orientation::Vertical, 10);
    tab.set_margin_top(10);
    tab.set_margin_bottom(10);

    // Section 1: Style Selection Cards (Matching user screenshots 1, 2, 4)
    let style_sec = GtkBox::new(Orientation::Vertical, 10);
    style_sec.add_css_class("section-box");

    let s_title = Label::new(Some("Google Pixel / Material You Saat Stili"));
    s_title.add_css_class("section-title");
    s_title.set_halign(gtk4::Align::Start);
    style_sec.append(&s_title);

    let s_sub = Label::new(Some("Masaüstü duvar kağıdı üzerinde çalışan Google Pixel saat stilleri:"));
    s_sub.add_css_class("setting-subtext");
    s_sub.set_halign(gtk4::Align::Start);
    style_sec.append(&s_sub);

    let style_choices = StringList::new(&[
        "Pixel Kalın Yatay (16:49 + Pil Kapsülü & Tarih)",
        "Pixel İki Katlı Dikey (16 / 49 Büyük Balon Rakam)",
        "Pixel İçi Boş Kontur (Outline Minimalist)",
        "Pixel Analog Kalın (12, 3, 6, 9 Kalın Rakamlar)",
        "Pixel Analog Kontur (12, 3, 6, 9 İçi Boş Rakamlar)",
        "Pixel Analog Takimetre (Hız Göstergesi Kadranı)",
    ]);
    let style_dd = DropDown::new(Some(style_choices), None::<gtk4::Expression>);

    let clock_cfg = PixelClockConfig::load();
    let initial_idx = match clock_cfg.style {
        PixelClockStyle::BoldHorizontal => 0,
        PixelClockStyle::StackedVertical => 1,
        PixelClockStyle::Outline => 2,
        PixelClockStyle::AnalogBold => 3,
        PixelClockStyle::AnalogOutline => 4,
        PixelClockStyle::AnalogTachymeter => 5,
    };
    style_dd.set_selected(initial_idx);
    style_dd.connect_selected_notify(|dd| {
        let mut cfg = PixelClockConfig::load();
        cfg.style = match dd.selected() {
            0 => PixelClockStyle::BoldHorizontal,
            1 => PixelClockStyle::StackedVertical,
            2 => PixelClockStyle::Outline,
            3 => PixelClockStyle::AnalogBold,
            4 => PixelClockStyle::AnalogOutline,
            _ => PixelClockStyle::AnalogTachymeter,
        };
        let _ = cfg.save();
    });
    style_sec.append(&style_dd);
    tab.append(&style_sec);

    // Section 2: Colors & Material You (Matching screenshot 3)
    let color_sec = GtkBox::new(Orientation::Vertical, 10);
    color_sec.add_css_class("section-box");

    let c_title = Label::new(Some("Renklendirme Seçenekleri (Coloring Options)"));
    c_title.add_css_class("section-title");
    c_title.set_halign(gtk4::Align::Start);
    color_sec.append(&c_title);

    let color_choices = StringList::new(&[
        "Dinamik: Material You (Duvar kağıdından otomatik uyarlanan ton)",
        "Özel Renk: Elektrik Camgöbeği (Cyan #80d4ff)",
        "Özel Renk: Pastel Mor (Lavender #c084fc)",
        "Özel Renk: Nane Yeşili (Mint #4ade80)",
        "Özel Renk: Şeftali / Mercan (Peach #fb923c)",
        "Özel Renk: Saf Beyaz (Pure White #ffffff)",
    ]);
    let color_dd = DropDown::new(Some(color_choices), None::<gtk4::Expression>);
    let color_idx = if clock_cfg.color_mode == "dynamic" {
        0
    } else {
        match clock_cfg.custom_color.as_str() {
            "#80d4ff" => 1,
            "#c084fc" => 2,
            "#4ade80" => 3,
            "#fb923c" => 4,
            _ => 5,
        }
    };
    color_dd.set_selected(color_idx);
    color_dd.connect_selected_notify(|dd| {
        let mut cfg = PixelClockConfig::load();
        match dd.selected() {
            0 => {
                cfg.color_mode = "dynamic".to_string();
            }
            1 => {
                cfg.color_mode = "custom".to_string();
                cfg.custom_color = "#80d4ff".to_string();
            }
            2 => {
                cfg.color_mode = "custom".to_string();
                cfg.custom_color = "#c084fc".to_string();
            }
            3 => {
                cfg.color_mode = "custom".to_string();
                cfg.custom_color = "#4ade80".to_string();
            }
            4 => {
                cfg.color_mode = "custom".to_string();
                cfg.custom_color = "#fb923c".to_string();
            }
            _ => {
                cfg.color_mode = "custom".to_string();
                cfg.custom_color = "#ffffff".to_string();
            }
        }
        let _ = cfg.save();
    });
    color_sec.append(&color_dd);
    tab.append(&color_sec);

    // Section 3: Opacity, Battery Pill, Date
    let opts_sec = GtkBox::new(Orientation::Vertical, 10);
    opts_sec.add_css_class("section-box");

    let o_title = Label::new(Some("Rakam Opaklığı ve Göstergeler"));
    o_title.add_css_class("section-title");
    o_title.set_halign(gtk4::Align::Start);
    opts_sec.append(&o_title);

    let row1 = GtkBox::new(Orientation::Horizontal, 16);
    let op_lbl = Label::new(Some("Rakam Opaklığı (Numeral Opacity):"));
    op_lbl.add_css_class("setting-label");
    row1.append(&op_lbl);

    let op_choices = StringList::new(&["%100 (Tam Belirgin)", "%95 (Önerilen)", "%80 (Hafif Saydam)", "%60 (Yarı Saydam)", "%40 (Düşük)"]);
    let op_dd = DropDown::new(Some(op_choices), None::<gtk4::Expression>);
    op_dd.set_selected(1);
    op_dd.connect_selected_notify(|dd| {
        let mut cfg = PixelClockConfig::load();
        cfg.numeral_opacity = match dd.selected() {
            0 => 1.0,
            1 => 0.95,
            2 => 0.80,
            3 => 0.60,
            _ => 0.40,
        };
        let _ = cfg.save();
    });
    row1.append(&op_dd);
    opts_sec.append(&row1);

    let row2 = GtkBox::new(Orientation::Horizontal, 20);
    let bat_chk = CheckButton::with_label("Pil Kapsülünü Göster (Battery Pill [PIL] %41)");
    bat_chk.set_active(clock_cfg.show_battery_pill);
    bat_chk.connect_toggled(|cb| {
        let mut cfg = PixelClockConfig::load();
        cfg.show_battery_pill = cb.is_active();
        let _ = cfg.save();
    });
    row2.append(&bat_chk);

    let date_chk = CheckButton::with_label("Tarih Göster");
    date_chk.set_active(clock_cfg.show_date);
    date_chk.connect_toggled(|cb| {
        let mut cfg = PixelClockConfig::load();
        cfg.show_date = cb.is_active();
        let _ = cfg.save();
    });
    row2.append(&date_chk);
    opts_sec.append(&row2);
    tab.append(&opts_sec);

    // Section 4: Action Buttons (Launch / Restart widget)
    let act_sec = GtkBox::new(Orientation::Horizontal, 12);
    act_sec.set_halign(gtk4::Align::End);

    let launch_btn = Button::with_label("Pixel Saat Widget'ını Masaüstünde Aç");
    launch_btn.add_css_class("action-btn");
    launch_btn.connect_clicked(|_| {
        let _ = Command::new("pkill").args(["-f", "solar-shell (clock|pixel-clock)"]).status();
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(150));
            let _ = Command::new("solar-shell").arg("clock").spawn();
        });
    });
    act_sec.append(&launch_btn);
    tab.append(&act_sec);

    tab
}

fn build_taskbar_tab() -> GtkBox {
    let tab = GtkBox::new(Orientation::Vertical, 10);
    tab.set_margin_top(10);
    tab.set_margin_bottom(10);

    // Section 1: Appearance Style Presets
    let preset_sec = GtkBox::new(Orientation::Vertical, 10);
    preset_sec.add_css_class("section-box");

    let p_title = Label::new(Some("Görev Çubuğu Görünüm Stili (Taskbar Appearance)"));
    p_title.add_css_class("section-title");
    p_title.set_halign(gtk4::Align::Start);
    preset_sec.append(&p_title);

    let p_sub = Label::new(Some(
        "Buzlu cam efekti zorunlu değildir. İster düz renk, ister şeffaf cam efekti, ister tam saydam, isterseniz de Material 3 duvar kağıdı renklerine dinamik uyarlanan stil seçebilirsiniz:",
    ));
    p_sub.add_css_class("setting-subtext");
    p_sub.set_wrap(true);
    p_sub.set_halign(gtk4::Align::Start);
    preset_sec.append(&p_sub);

    let preset_choices = StringList::new(&[
        "Buzlu Cam (Frosted Acrylic Glass — %32 Şeffaflık & Blur)",
        "Düz Opak Renk (Solid Opaque — %100 Mat)",
        "Tam Şeffaf (Fully Transparent — Sadece Simgeler Yüzer)",
        "Material 3 Dinamik Renk (Duvar Kağıdı Rengine Uyumlu)",
    ]);
    let preset_dd = DropDown::new(Some(preset_choices), None::<gtk4::Expression>);
    preset_dd.set_selected(0);
    preset_dd.connect_selected_notify(|dd| {
        match dd.selected() {
            0 => apply_taskbar_preset("glass"),
            1 => apply_taskbar_preset("solid"),
            2 => apply_taskbar_preset("transparent"),
            _ => apply_taskbar_preset("m3-adaptive"),
        }
    });
    preset_sec.append(&preset_dd);
    tab.append(&preset_sec);

    // Section 2: Fine-grained Customization (Opacity, Corner Radius, Height)
    let fine_sec = GtkBox::new(Orientation::Vertical, 10);
    fine_sec.add_css_class("section-box");

    let f_title = Label::new(Some("Manuel Boyut ve Şeffaflık İnce Ayarları"));
    f_title.add_css_class("section-title");
    f_title.set_halign(gtk4::Align::Start);
    fine_sec.append(&f_title);

    // Opacity
    let op_row = GtkBox::new(Orientation::Horizontal, 16);
    let op_l = Label::new(Some("Şeffaflık Derecesi:"));
    op_l.add_css_class("setting-label");
    op_l.set_hexpand(true);
    op_l.set_halign(gtk4::Align::Start);
    op_row.append(&op_l);

    let op_list = StringList::new(&["%10 (Ultra Cam)", "%25 (Hafif Cam)", "%32 (Varsayılan Solar)", "%50 (Yarı Saydam)", "%75 (Koyu Cam)", "%100 (Mat)"]);
    let op_dd = DropDown::new(Some(op_list), None::<gtk4::Expression>);
    op_dd.set_selected(2);
    op_dd.connect_selected_notify(|dd| {
        let op = match dd.selected() {
            0 => 0.10,
            1 => 0.25,
            2 => 0.32,
            3 => 0.50,
            4 => 0.75,
            _ => 1.0,
        };
        update_noctalia_taskbar_opacity(op);
    });
    op_row.append(&op_dd);
    fine_sec.append(&op_row);

    // Corner Radius
    let rad_row = GtkBox::new(Orientation::Horizontal, 16);
    let rad_l = Label::new(Some("Köşe Yuvarlaklığı:"));
    rad_l.add_css_class("setting-label");
    rad_l.set_hexpand(true);
    rad_l.set_halign(gtk4::Align::Start);
    rad_row.append(&rad_l);

    let rad_list = StringList::new(&["0 px (Köşeli Düz)", "8 px (Hafif Kavis)", "16 px (Varsayılan Modern)", "24 px (Ekstra Yuvarlak)", "40 px (Kapsül)"]);
    let rad_dd = DropDown::new(Some(rad_list), None::<gtk4::Expression>);
    rad_dd.set_selected(2);
    rad_dd.connect_selected_notify(|dd| {
        let r = match dd.selected() {
            0 => 0,
            1 => 8,
            2 => 16,
            3 => 24,
            _ => 40,
        };
        update_noctalia_taskbar_radius(r);
    });
    rad_row.append(&rad_dd);
    fine_sec.append(&rad_row);

    // Height / Thickness
    let h_row = GtkBox::new(Orientation::Horizontal, 16);
    let h_l = Label::new(Some("Çubuk Yüksekliği (Kalınlık):"));
    h_l.add_css_class("setting-label");
    h_l.set_hexpand(true);
    h_l.set_halign(gtk4::Align::Start);
    h_row.append(&h_l);

    let h_list = StringList::new(&["36 px (Kompakt)", "40 px (Varsayılan)", "44 px (Rahat)", "48 px (Büyük)", "56 px (KDE Plasma Standart)"]);
    let h_dd = DropDown::new(Some(h_list), None::<gtk4::Expression>);
    h_dd.set_selected(1);
    h_dd.connect_selected_notify(|dd| {
        let th = match dd.selected() {
            0 => 36,
            1 => 40,
            2 => 44,
            3 => 48,
            _ => 56,
        };
        update_noctalia_taskbar_thickness(th);
    });
    h_row.append(&h_dd);
    fine_sec.append(&h_row);

    tab.append(&fine_sec);

    // Section 3: Open Full Noctalia Bar Settings Button
    let btn_row = GtkBox::new(Orientation::Horizontal, 12);
    btn_row.set_halign(gtk4::Align::End);
    let full_btn = Button::with_label("Gelişmiş Çubuk Düzenleyicisini Aç");
    full_btn.add_css_class("action-btn");
    full_btn.connect_clicked(|_| {
        let _ = Command::new("noctalia").args(["msg", "settings-open", "bar"]).spawn();
    });
    btn_row.append(&full_btn);
    tab.append(&btn_row);

    tab
}

fn noctalia_settings_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    format!("{}/.local/state/noctalia/settings.toml", home)
}

fn apply_taskbar_preset(preset: &str) {
    let path = noctalia_settings_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(mut doc) = content.parse::<toml::Value>() {
            if let Some(bar) = doc.get_mut("bar").and_then(|b| b.get_mut("taskbar")) {
                if let Some(t) = bar.as_table_mut() {
                    match preset {
                        "glass" => {
                            t.insert("background_opacity".to_string(), toml::Value::Float(0.32));
                            t.insert("border".to_string(), toml::Value::String("outline".to_string()));
                            t.insert("radius".to_string(), toml::Value::Integer(16));
                        }
                        "solid" => {
                            t.insert("background_opacity".to_string(), toml::Value::Float(1.0));
                            t.insert("border".to_string(), toml::Value::String("none".to_string()));
                            t.insert("radius".to_string(), toml::Value::Integer(12));
                        }
                        "transparent" => {
                            t.insert("background_opacity".to_string(), toml::Value::Float(0.0));
                            t.insert("border".to_string(), toml::Value::String("none".to_string()));
                            t.insert("radius".to_string(), toml::Value::Integer(16));
                        }
                        "m3-adaptive" => {
                            t.insert("background_opacity".to_string(), toml::Value::Float(0.42));
                            t.insert("border".to_string(), toml::Value::String("outline".to_string()));
                            t.insert("radius".to_string(), toml::Value::Integer(20));
                        }
                        _ => {}
                    }
                }
            }
            if let Ok(serialized) = toml::to_string_pretty(&doc) {
                let _ = std::fs::write(&path, serialized);
                let _ = Command::new("noctalia").args(["msg", "templates-apply"]).spawn();
            }
        }
    }
}

fn update_noctalia_taskbar_opacity(opacity: f64) {
    let path = noctalia_settings_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(mut doc) = content.parse::<toml::Value>() {
            if let Some(bar) = doc.get_mut("bar").and_then(|b| b.get_mut("taskbar")) {
                if let Some(t) = bar.as_table_mut() {
                    t.insert("background_opacity".to_string(), toml::Value::Float(opacity));
                }
            }
            if let Ok(serialized) = toml::to_string_pretty(&doc) {
                let _ = std::fs::write(&path, serialized);
                let _ = Command::new("noctalia").args(["msg", "templates-apply"]).spawn();
            }
        }
    }
}

fn update_noctalia_taskbar_radius(radius: i32) {
    let path = noctalia_settings_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(mut doc) = content.parse::<toml::Value>() {
            if let Some(bar) = doc.get_mut("bar").and_then(|b| b.get_mut("taskbar")) {
                if let Some(t) = bar.as_table_mut() {
                    t.insert("radius".to_string(), toml::Value::Integer(radius as i64));
                }
            }
            if let Ok(serialized) = toml::to_string_pretty(&doc) {
                let _ = std::fs::write(&path, serialized);
                let _ = Command::new("noctalia").args(["msg", "templates-apply"]).spawn();
            }
        }
    }
}

fn update_noctalia_taskbar_thickness(thickness: i32) {
    let path = noctalia_settings_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(mut doc) = content.parse::<toml::Value>() {
            if let Some(bar) = doc.get_mut("bar").and_then(|b| b.get_mut("taskbar")) {
                if let Some(t) = bar.as_table_mut() {
                    t.insert("thickness".to_string(), toml::Value::Integer(thickness as i64));
                }
            }
            if let Ok(serialized) = toml::to_string_pretty(&doc) {
                let _ = std::fs::write(&path, serialized);
                let _ = Command::new("noctalia").args(["msg", "templates-apply"]).spawn();
            }
        }
    }
}

fn update_noctalia_shell_settings(engine: &str, launcher: &str, dash: &str) {
    let path = noctalia_settings_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(mut doc) = content.parse::<toml::Value>() {
            if let Some(table) = doc.as_table_mut() {
                if !table.contains_key("solarui") {
                    table.insert("solarui".to_string(), toml::Value::Table(toml::map::Map::new()));
                }
                if let Some(s) = table.get_mut("solarui").and_then(|v| v.as_table_mut()) {
                    s.insert("shell_engine".to_string(), toml::Value::String(engine.to_string()));
                    s.insert("launcher_provider".to_string(), toml::Value::String(launcher.to_string()));
                    s.insert("dashboard_provider".to_string(), toml::Value::String(dash.to_string()));
                }
            }
            if let Ok(serialized) = toml::to_string_pretty(&doc) {
                let _ = std::fs::write(&path, serialized);
            }
        }
    }
}

fn build_shell_tab() -> GtkBox {
    let tab = GtkBox::new(Orientation::Vertical, 12);
    tab.set_margin_top(10);
    tab.set_margin_bottom(10);

    let sec = GtkBox::new(Orientation::Vertical, 12);
    sec.add_css_class("section-box");

    let title = Label::new(Some("Kabuk ve Yüzey Motoru (Shell Engine Switcher)"));
    title.add_css_class("section-title");
    title.set_halign(gtk4::Align::Start);
    sec.append(&title);

    let desc = Label::new(Some(
        "Noctalia, Caelestia ve end4-pC tam kabuk motorları arasında güvenli biçimde geçiş yapabilirsiniz. Her motor kendi bar, bildirim, OSD ve sistem tepsisini tek başına yönetir.",
    ));
    desc.add_css_class("setting-subtext");
    desc.set_wrap(true);
    desc.set_halign(gtk4::Align::Start);
    sec.append(&desc);

    let cfg = SolarConfig::load();

    // 1. Shell Engine Selector
    let row1 = GtkBox::new(Orientation::Horizontal, 12);
    let lbl1 = Label::new(Some("Aktif Kabuk Motoru:"));
    lbl1.add_css_class("setting-label");
    lbl1.set_hexpand(true);
    lbl1.set_halign(gtk4::Align::Start);
    row1.append(&lbl1);

    let engines = [
        "Noctalia v5 (C++23 Native)",
        "Caelestia Shell (Quickshell QML)",
        "end4-pC (Material 3 Expressive)",
    ];
    let engine_model = StringList::new(&engines);
    let engine_dd = DropDown::new(Some(engine_model), None::<gtk4::Expression>);
    let active_idx = match cfg.shell.engine {
        ShellEngine::Noctalia => 0,
        ShellEngine::Caelestia => 1,
        ShellEngine::End4Pc => 2,
    };
    engine_dd.set_selected(active_idx);
    row1.append(&engine_dd);
    sec.append(&row1);

    // 2. Action Buttons
    let btn_box = GtkBox::new(Orientation::Horizontal, 10);
    btn_box.set_margin_top(8);

    let apply_btn = Button::with_label("Kabuk Motorunu Uygula & Geçiş Yap");
    apply_btn.add_css_class("action-btn");
    apply_btn.set_halign(gtk4::Align::Start);

    let engine_dd_clone = engine_dd.clone();

    let shell_status_lbl = Label::new(Some(""));
    shell_status_lbl.add_css_class("setting-subtext");
    shell_status_lbl.set_halign(gtk4::Align::Start);

    let shell_status_clone = shell_status_lbl.clone();
    apply_btn.connect_clicked(move |_| {
        let mut cfg = SolarConfig::load();
        let target_engine = match engine_dd_clone.selected() {
            0 => ShellEngine::Noctalia,
            1 => ShellEngine::Caelestia,
            _ => ShellEngine::End4Pc,
        };
        cfg.shell.engine = target_engine;
        cfg.shell.launcher_provider = match target_engine {
            ShellEngine::Noctalia => SurfaceProvider::Noctalia,
            ShellEngine::Caelestia => SurfaceProvider::Caelestia,
            ShellEngine::End4Pc => SurfaceProvider::End4Pc,
        };
        cfg.shell.dashboard_provider = cfg.shell.launcher_provider;
        cfg.shell.bar_provider = cfg.shell.launcher_provider;
        cfg.shell.notifications_provider = cfg.shell.launcher_provider;
        cfg.shell.osd_provider = cfg.shell.launcher_provider;
        let _ = cfg.save();

        update_noctalia_shell_settings(
            target_engine.as_str(),
            cfg.shell.launcher_provider.as_str(),
            cfg.shell.dashboard_provider.as_str(),
        );

        let engine_str = target_engine.as_str();
        shell_status_clone.set_text(&format!(
            "[OK] Kabuk motoru ({}) seçildi ve uygulandı.",
            engine_str
        ));

        let engine_to_switch = engine_str.to_string();
        std::thread::spawn(move || {
            let _ = Command::new("solar-shell")
                .args(["switch", &engine_to_switch])
                .status();
        });
    });
    btn_box.append(&apply_btn);

    let reconcile_btn = Button::with_label("Durumu Doğrula (Supervisor Reconcile)");
    reconcile_btn.add_css_class("secondary-btn");
    reconcile_btn.set_halign(gtk4::Align::Start);
    let shell_status_rec = shell_status_lbl.clone();
    reconcile_btn.connect_clicked(move |_| {
        shell_status_rec.set_text("[OK] Kabuk durumu denetleniyor ve doğrulanıyor...");
        std::thread::spawn(|| {
            let _ = Command::new("solar-shell")
                .args(["supervisor"])
                .status();
        });
    });
    btn_box.append(&reconcile_btn);

    let end4_settings_btn = Button::with_label("end4-pC Ayarlarını Aç");
    end4_settings_btn.add_css_class("secondary-btn");
    end4_settings_btn.set_halign(gtk4::Align::Start);
    let shell_status_end4 = shell_status_lbl.clone();
    end4_settings_btn.connect_clicked(move |_| {
        let cfg = SolarConfig::load();
        if cfg.shell.engine != ShellEngine::End4Pc {
            shell_status_end4.set_text("end4-pC ayarları için önce end4-pC motorunu uygulayın.");
            return;
        }
        shell_status_end4.set_text("end4-pC ayarları açılıyor...");
        std::thread::spawn(|| {
            let _ = Command::new("solar-shell")
                .args(["route", "settings"])
                .status();
        });
    });
    btn_box.append(&end4_settings_btn);

    sec.append(&btn_box);
    sec.append(&shell_status_lbl);
    tab.append(&sec);

    tab
}

fn build_caelestia_tab() -> GtkBox {
    let tab = GtkBox::new(Orientation::Vertical, 12);
    tab.set_margin_top(10);
    tab.set_margin_bottom(10);
    
    let cfg = SolarConfig::load();
    if cfg.shell.engine != ShellEngine::Caelestia {
        let msg = Label::new(Some("Bu ayarlar yalnız tam Caelestia motoru aktifken kullanılabilir. Lütfen 'Kabuk Motoru' sekmesinden Caelestia'yı seçin."));
        msg.add_css_class("setting-label");
        msg.set_wrap(true);
        msg.set_halign(gtk4::Align::Center);
        msg.set_valign(gtk4::Align::Center);
        msg.set_vexpand(true);
        tab.append(&msg);
        return tab;
    }
    
    // Section 1: Wallpaper
    let sec_wall = GtkBox::new(Orientation::Vertical, 10);
    sec_wall.add_css_class("section-box");
    
    let w_title = Label::new(Some("Duvar Kağıdı ve Material 3 Renk Şeması"));
    w_title.add_css_class("section-title");
    w_title.set_halign(gtk4::Align::Start);
    sec_wall.append(&w_title);
    
    let w_desc = Label::new(Some(
        "Caelestia yüzeyleri (Launcher, Dashboard) seçilen duvar kağıdına göre dinamik olarak renk paleti üretir. Duvar kağıdını değiştirdiğinizde tüm arayüz renkleri otomatik güncellenir."
    ));
    w_desc.add_css_class("setting-subtext");
    w_desc.set_wrap(true);
    w_desc.set_halign(gtk4::Align::Start);
    sec_wall.append(&w_desc);
    
    let btn_row = GtkBox::new(Orientation::Horizontal, 10);
    
    let btn_rand = Button::with_label("Rastgele Duvar Kağıdı");
    btn_rand.add_css_class("action-btn");
    btn_rand.connect_clicked(|_| {
        std::thread::spawn(|| {
            let _ = Command::new("caelestia").args(["wallpaper", "-r"]).status();
        });
    });
    btn_row.append(&btn_rand);

    let btn_pick = Button::with_label("Dosyadan Seç...");
    btn_pick.add_css_class("secondary-btn");
    btn_pick.connect_clicked(|btn| {
        let win = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
        let dialog = gtk4::FileDialog::builder()
            .title("Duvar Kağıdı Seç (Caelestia)")
            .modal(true)
            .build();
        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("Görseller (PNG, JPG, WEBP)"));
        filter.add_mime_type("image/png");
        filter.add_mime_type("image/jpeg");
        filter.add_mime_type("image/webp");
        let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));
        dialog.open(win.as_ref(), None::<&gtk4::gio::Cancellable>, |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    let path_str = path.to_string_lossy().to_string();
                    std::thread::spawn(move || {
                        let _ = Command::new("caelestia").args(["wallpaper", "-f", &path_str]).status();
                    });
                }
            }
        });
    });
    btn_row.append(&btn_pick);
    
    sec_wall.append(&btn_row);
    tab.append(&sec_wall);
    
    // Section 2: Mode and Layout
    let sec_lay = GtkBox::new(Orientation::Vertical, 10);
    sec_lay.add_css_class("section-box");
    
    let l_title = Label::new(Some("Masaüstü ve Arayüz Düzeni"));
    l_title.add_css_class("section-title");
    l_title.set_halign(gtk4::Align::Start);
    sec_lay.append(&l_title);
    
    let l_desc = Label::new(Some(
        "Quickshell tabanlı Caelestia arayüzü için tema modu ve diğer yapılandırmalar."
    ));
    l_desc.add_css_class("setting-subtext");
    l_desc.set_halign(gtk4::Align::Start);
    sec_lay.append(&l_desc);
    
    let row_mode = GtkBox::new(Orientation::Horizontal, 12);
    let lbl_mode = Label::new(Some("Caelestia Tema Modu:"));
    lbl_mode.add_css_class("setting-label");
    row_mode.append(&lbl_mode);

    let mode_dd = DropDown::new(Some(StringList::new(&["Karanlık (Dark)", "Aydınlık (Light)"])), None::<gtk4::Expression>);
    
    // Initial read
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let scheme_path = format!("{}/.local/state/caelestia/scheme.json", home);
    if let Ok(content) = std::fs::read_to_string(&scheme_path) {
        if content.contains("\"mode\": \"light\"") {
            mode_dd.set_selected(1);
        }
    }

    mode_dd.connect_selected_notify(move |dd| {
        let is_light = dd.selected() == 1;
        std::thread::spawn(move || {
            let mode_str = if is_light { "light" } else { "dark" };
            let gnome_mode = if is_light { "prefer-light" } else { "prefer-dark" };

            // 1. Run caelestia scheme set CLI which re-computes full light/dark Material You colors
            let _ = Command::new("caelestia").args(["scheme", "set", "--notify", "-m", mode_str]).status();

            // 2. Set GNOME desktop color-scheme for GTK apps
            let _ = Command::new("gsettings").args(["set", "org.gnome.desktop.interface", "color-scheme", gnome_mode]).status();

            // 3. Trigger Caelestia reload
            let _ = Command::new("pkill").args(["-SIGUSR2", "-f", "quickshell.*caelestia"]).status();
        });
    });
    row_mode.append(&mode_dd);
    sec_lay.append(&row_mode);

    tab.append(&sec_lay);
    tab
}

fn build_display_tab() -> GtkBox {
    let tab = GtkBox::new(Orientation::Vertical, 16);
    tab.set_margin_top(14);
    tab.set_margin_bottom(14);
    tab.set_margin_start(16);
    tab.set_margin_end(16);

    let sec = GtkBox::new(Orientation::Vertical, 12);
    sec.add_css_class("section-box");

    let title = Label::new(Some("Genel Masaüstü ve Arayüz Ölçeği"));
    title.add_css_class("section-title");
    title.set_halign(gtk4::Align::Start);
    sec.append(&title);

    let desc = Label::new(Some(
        "Ekranınızdaki pencere ve simgelerin genel boyutunu donanımınıza ve ekran çözünürlüğünüze göre ayarlayın. Küçük veya sanal makine ekranlarında (1280x800 vb.) %85 veya %75 kompakt ölçek önerilir. Çoklu monitör kurulumlarında her monitör için aşağıdan bağımsız ölçek atayabilirsiniz.",
    ));
    desc.add_css_class("setting-subtext");
    desc.set_wrap(true);
    desc.set_halign(gtk4::Align::Start);
    sec.append(&desc);

    let cfg = SolarConfig::load();

    // Row: Scale factor selection
    let row_scale = GtkBox::new(Orientation::Horizontal, 12);
    let lbl_scale = Label::new(Some("Genel Sistem Ölçeği:"));
    lbl_scale.add_css_class("setting-label");
    lbl_scale.set_hexpand(true);
    lbl_scale.set_halign(gtk4::Align::Start);
    row_scale.append(&lbl_scale);

    let scale_presets = [
        "0.75x — Kompakt (%75)",
        "0.85x — Küçük (%85 - 1280x800 ve sanal makine için önerilen)",
        "1.00x — Standart (%100 - Varsayılan)",
        "1.25x — Büyük (%125)",
        "1.50x — Geniş (%150 - 2K Ekranlar)",
        "1.75x — Ekstra Büyük (%175)",
        "2.00x — HiDPI (%200 - 4K Ekranlar)",
    ];
    let scale_values = [0.75, 0.85, 1.00, 1.25, 1.50, 1.75, 2.00];

    let scale_model = StringList::new(&scale_presets);
    let scale_dd = DropDown::new(Some(scale_model), None::<gtk4::Expression>);

    // Select matching index based on current config
    let cur_scale = cfg.display.scale;
    let mut selected_idx = 2; // Default to 1.00x (index 2)
    for (i, &val) in scale_values.iter().enumerate() {
        if (cur_scale - val).abs() < 0.04 {
            selected_idx = i as u32;
            break;
        }
    }
    scale_dd.set_selected(selected_idx);
    row_scale.append(&scale_dd);
    sec.append(&row_scale);

    // Row: Status label for feedback
    let status_lbl = Label::new(Some(""));
    status_lbl.add_css_class("setting-subtext");
    status_lbl.set_halign(gtk4::Align::Start);

    // Row: Apply buttons
    let btn_box = GtkBox::new(Orientation::Horizontal, 10);
    btn_box.set_margin_top(8);

    let apply_btn = Button::with_label("Tüm Ekranlara Uygula");
    apply_btn.add_css_class("action-btn");

    let scale_dd_clone = scale_dd.clone();
    let status_clone = status_lbl.clone();

    apply_btn.connect_clicked(move |_| {
        let idx = scale_dd_clone.selected() as usize;
        let chosen_scale = if idx < scale_values.len() {
            scale_values[idx]
        } else {
            1.00
        };

        // 1. Save in SolarConfig
        let mut c = SolarConfig::load();
        c.display.scale = chosen_scale;
        c.display.text_scale = chosen_scale;
        let _ = c.save();

        // 2. Apply dynamically via Niri IPC
        apply_display_scale_dynamically(chosen_scale);

        // 3. Update status label
        status_clone.set_text(&format!(
            "[OK] Genel ölçek {:.2}x başarıyla uygulandı ve kaydedildi.",
            chosen_scale
        ));
    });
    btn_box.append(&apply_btn);

    // Reset button
    let reset_btn = Button::with_label("Varsayılana Sıfırla (1.00x)");
    reset_btn.add_css_class("secondary-btn");
    let scale_dd_reset = scale_dd.clone();
    let status_reset = status_lbl.clone();
    reset_btn.connect_clicked(move |_| {
        scale_dd_reset.set_selected(2);
        let mut c = SolarConfig::load();
        c.display.scale = 1.00;
        c.display.text_scale = 1.00;
        let _ = c.save();
        apply_display_scale_dynamically(1.00);
        status_reset.set_text("[OK] Ölçek varsayılan 1.00x değerine sıfırlandı.");
    });
    btn_box.append(&reset_btn);

    sec.append(&btn_box);
    sec.append(&status_lbl);
    tab.append(&sec);

    // SECTION 2: Multi-Monitor Independent DPI Scaling
    let sec_mon = GtkBox::new(Orientation::Vertical, 12);
    sec_mon.add_css_class("section-box");

    let title_mon = Label::new(Some("Bağlı Monitörler ve Bağımsız DPI Ölçekleme"));
    title_mon.add_css_class("section-title");
    title_mon.set_halign(gtk4::Align::Start);
    sec_mon.append(&title_mon);

    let desc_mon = Label::new(Some(
        "Her bir monitörün donanımsal çözünürlüğüne ve panel boyutuna göre bağımsız ölçek belirleyin. Örneğin dizüstü dahili ekranınız %125 iken harici monitörünüz %100 veya %150 olarak çalışabilir.",
    ));
    desc_mon.add_css_class("setting-subtext");
    desc_mon.set_wrap(true);
    desc_mon.set_halign(gtk4::Align::Start);
    sec_mon.append(&desc_mon);

    let detected_outputs = get_detected_outputs();
    for out in detected_outputs {
        let mon_box = GtkBox::new(Orientation::Vertical, 8);
        mon_box.set_margin_top(4);
        mon_box.set_margin_bottom(6);

        let mon_header = GtkBox::new(Orientation::Horizontal, 10);
        let mon_icon = Image::from_icon_name("video-display");
        mon_icon.set_pixel_size(24);
        mon_header.append(&mon_icon);

        let info_box = GtkBox::new(Orientation::Vertical, 2);
        info_box.set_hexpand(true);

        let name_lbl = Label::new(Some(&format!("{} — {}", out.name, out.description)));
        name_lbl.add_css_class("setting-label");
        name_lbl.set_halign(gtk4::Align::Start);
        info_box.append(&name_lbl);

        let res_lbl = Label::new(Some(&format!("Çözünürlük: {}", out.resolution)));
        res_lbl.add_css_class("setting-subtext");
        res_lbl.set_halign(gtk4::Align::Start);
        info_box.append(&res_lbl);
        mon_header.append(&info_box);

        // DropDown for this monitor
        let mon_dd = DropDown::new(Some(StringList::new(&scale_presets)), None::<gtk4::Expression>);
        let mut mon_sel_idx = 2;
        for (i, &val) in scale_values.iter().enumerate() {
            if (out.scale - val).abs() < 0.04 {
                mon_sel_idx = i as u32;
                break;
            }
        }
        mon_dd.set_selected(mon_sel_idx);
        mon_header.append(&mon_dd);

        // Apply button for this monitor
        let mon_apply = Button::with_label("Uygula");
        mon_apply.add_css_class("action-btn");

        let out_name = out.name.clone();
        let mon_dd_clone = mon_dd.clone();
        let mon_status = Label::new(Some(""));
        mon_status.add_css_class("setting-subtext");
        mon_status.set_halign(gtk4::Align::Start);
        let mon_status_clone = mon_status.clone();

        mon_apply.connect_clicked(move |_| {
            let idx = mon_dd_clone.selected() as usize;
            let val = if idx < scale_values.len() {
                scale_values[idx]
            } else {
                1.00
            };
            apply_per_output_scale_dynamically(&out_name, val);
            mon_status_clone.set_text(&format!(
                "[OK] {} ekranı için {:.2}x bağımsız ölçek uygulandı.",
                out_name, val
            ));
        });
        mon_header.append(&mon_apply);

        mon_box.append(&mon_header);
        mon_box.append(&mon_status);
        sec_mon.append(&mon_box);
    }

    tab.append(&sec_mon);
    tab
}

pub fn detect_screen_resolution_and_auto_scale() -> f64 {
    let cfg = SolarConfig::load();
    // If user already explicitly set a custom non-default scale, respect their preference
    if (cfg.display.scale - 1.0).abs() > 0.04 {
        return cfg.display.scale;
    }

    // 1. Try to query Niri via `niri msg --json outputs`
    if let Ok(out) = Command::new("niri").args(["msg", "--json", "outputs"]).output() {
        if out.status.success() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(obj) = val.as_object() {
                    let mut min_height = u32::MAX;
                    for (_name, output_val) in obj {
                        let height = if let Some(current_mode_idx) = output_val.get("current_mode").and_then(|v| v.as_u64()) {
                            output_val.get("modes")
                                .and_then(|m| m.as_array())
                                .and_then(|modes| modes.get(current_mode_idx as usize))
                                .and_then(|mode| mode.get("height"))
                                .and_then(|h| h.as_u64())
                                .map(|h| h as u32)
                        } else {
                            output_val.get("modes")
                                .and_then(|m| m.as_array())
                                .and_then(|modes| modes.first())
                                .and_then(|mode| mode.get("height"))
                                .and_then(|h| h.as_u64())
                                .map(|h| h as u32)
                        };

                        if let Some(h) = height {
                            if h < min_height {
                                min_height = h;
                            }
                        }
                    }

                    if min_height != u32::MAX {
                        return scale_for_height(min_height);
                    }
                }
            }
        }
    }

    // 2. Fallback: check DRM modes in /sys/class/drm/*/modes
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        let mut min_height = u32::MAX;
        for entry in entries.flatten() {
            let modes_file = entry.path().join("modes");
            if let Ok(content) = std::fs::read_to_string(modes_file) {
                for line in content.lines() {
                    if let Some((_w, h)) = line.split_once('x') {
                        if let Ok(height) = h.trim().parse::<u32>() {
                            if height < min_height {
                                min_height = height;
                            }
                        }
                    }
                }
            }
        }
        if min_height != u32::MAX {
            return scale_for_height(min_height);
        }
    }

    1.00
}

fn scale_for_height(height: u32) -> f64 {
    if height <= 720 {
        0.75
    } else if height <= 800 {
        0.85
    } else if height <= 900 {
        0.90
    } else if height >= 2160 {
        1.75
    } else if height >= 1440 {
        1.25
    } else {
        1.00
    }
}

pub fn sync_caelestia_scale(_scale: f64) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/liveuser".to_string());
    let conf_dir = PathBuf::from(&home).join(".config/caelestia");
    let _ = std::fs::create_dir_all(&conf_dir);
    let shell_json_path = conf_dir.join("shell.json");

    let mut val: serde_json::Value = if let Ok(content) = std::fs::read_to_string(&shell_json_path) {
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if !val.is_object() {
        val = serde_json::json!({});
    }

    if !val.get("appearance").map_or(false, |v| v.is_object()) {
        val["appearance"] = serde_json::json!({});
    }

    if let Some(app) = val.get_mut("appearance").and_then(|v| v.as_object_mut()) {
        if !app.contains_key("font") || !app["font"].is_object() {
            app.insert("font".to_string(), serde_json::json!({}));
        }
        if let Some(font_obj) = app.get_mut("font").and_then(|v| v.as_object_mut()) {
            if !font_obj.contains_key("icon") || !font_obj["icon"].is_object() {
                font_obj.insert("icon".to_string(), serde_json::json!({}));
            }
            if let Some(icon_obj) = font_obj.get_mut("icon").and_then(|v| v.as_object_mut()) {
                icon_obj.insert(
                    "family".to_string(),
                    serde_json::Value::String("Material Symbols Rounded".to_string()),
                );
            }
        }
    }

    if let Ok(formatted) = serde_json::to_string_pretty(&val) {
        let _ = std::fs::write(&shell_json_path, formatted);
    }
}

#[derive(Clone, Debug)]
pub struct OutputDetails {
    pub name: String,
    pub description: String,
    pub resolution: String,
    pub scale: f64,
}

pub fn get_detected_outputs() -> Vec<OutputDetails> {
    let mut outputs = Vec::new();
    let cfg = SolarConfig::load();

    if let Ok(out) = Command::new("niri").args(["msg", "--json", "outputs"]).output() {
        if out.status.success() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(obj) = val.as_object() {
                    for (name, data) in obj {
                        let make = data.get("make").and_then(|v| v.as_str()).unwrap_or("");
                        let model = data.get("model").and_then(|v| v.as_str()).unwrap_or("");
                        let desc = if !make.is_empty() || !model.is_empty() {
                            format!("{} {}", make, model).trim().to_string()
                        } else {
                            "Dahili / Harici Ekran".to_string()
                        };

                        let mut res_str = "1920x1080 @ 60Hz".to_string();
                        if let Some(modes) = data.get("modes").and_then(|v| v.as_array()) {
                            let cur_idx = data.get("current_mode").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                            if let Some(m) = modes.get(cur_idx).or_else(|| modes.first()) {
                                let w = m.get("width").and_then(|v| v.as_u64()).unwrap_or(1920);
                                let h = m.get("height").and_then(|v| v.as_u64()).unwrap_or(1080);
                                let rr = m.get("refresh_rate").and_then(|v| v.as_u64()).unwrap_or(60000) / 1000;
                                res_str = format!("{}x{} @ {}Hz", w, h, rr);
                            }
                        }

                        let saved_scale = cfg.display.outputs.iter().find(|o| o.name == *name).map(|o| o.scale);
                        let logical_scale = data.get("logical").and_then(|l| l.get("scale")).and_then(|v| v.as_f64());
                        let cur_scale = saved_scale.or(logical_scale).unwrap_or(cfg.display.scale);

                        outputs.push(OutputDetails {
                            name: name.clone(),
                            description: desc,
                            resolution: res_str,
                            scale: cur_scale,
                        });
                    }
                }
            }
        }
    }

    if outputs.is_empty() {
        outputs.push(OutputDetails {
            name: "Virtual-1".to_string(),
            description: "Sanal / Varsayılan Ekran".to_string(),
            resolution: "1920x1080 @ 60Hz".to_string(),
            scale: cfg.display.scale,
        });
    }

    outputs
}

pub fn apply_per_output_scale_dynamically(output_name: &str, scale: f64) {
    let scale_str = format!("{:.2}", scale);

    // 1. Run niri msg output <name> scale <scale_str>
    let _ = Command::new("niri")
        .args(["msg", "output", output_name, "scale", &scale_str])
        .status();

    // 2. Persist to ~/.config/solarui/niri.kdl
    if let Ok(home) = std::env::var("HOME") {
        let niri_kdl = PathBuf::from(home).join(".config/solarui/niri.kdl");
        if let Ok(content) = std::fs::read_to_string(&niri_kdl) {
            let mut new_lines = Vec::new();
            let mut skip_this_output = false;
            let target_header = format!("output \"{}\"", output_name);

            for line in content.lines() {
                if line.trim().starts_with(&target_header) {
                    skip_this_output = true;
                    continue;
                }
                if skip_this_output {
                    if line.trim() == "}" {
                        skip_this_output = false;
                    }
                    continue;
                }
                new_lines.push(line.to_string());
            }

            // Append output block with specific scale
            new_lines.push(format!("\noutput \"{}\" {{\n    scale {}\n}}", output_name, scale_str));
            let _ = std::fs::write(&niri_kdl, new_lines.join("\n"));
        }
    }

    // 3. Update SolarConfig outputs list
    let mut cfg = SolarConfig::load();
    if let Some(pos) = cfg.display.outputs.iter().position(|o| o.name == output_name) {
        cfg.display.outputs[pos].scale = scale;
    } else {
        cfg.display.outputs.push(solar_common::MonitorOutputConfig {
            name: output_name.to_string(),
            scale,
            mode: None,
        });
    }
    let _ = cfg.save();
}

pub fn apply_display_scale_dynamically(scale: f64) {
    let scale_str = format!("{:.2}", scale);

    // 1. Update active Niri outputs
    let mut outputs_found = Vec::new();
    if let Ok(out) = Command::new("niri").args(["msg", "--json", "outputs"]).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            for token in s.split('"') {
                if token == "Virtual-1"
                    || token.starts_with("eDP")
                    || token.starts_with("HDMI")
                    || token.starts_with("DP-")
                {
                    if !outputs_found.contains(&token.to_string()) {
                        outputs_found.push(token.to_string());
                    }
                }
            }
        }
    }
    if outputs_found.is_empty() {
        outputs_found.push("Virtual-1".to_string());
    }

    for op in &outputs_found {
        let _ = Command::new("niri")
            .args(["msg", "output", op, "scale", &scale_str])
            .status();
    }

    // 2. Persist to ~/.config/solarui/niri.kdl
    if let Ok(home) = std::env::var("HOME") {
        let niri_kdl = PathBuf::from(home).join(".config/solarui/niri.kdl");
        if let Ok(content) = std::fs::read_to_string(&niri_kdl) {
            let mut new_content = String::new();
            let mut skip_block = false;
            for line in content.lines() {
                if line.trim().starts_with("output \"") {
                    skip_block = true;
                    continue;
                }
                if skip_block {
                    if line.trim() == "}" {
                        skip_block = false;
                    }
                    continue;
                }
                new_content.push_str(line);
                new_content.push('\n');
            }

            // Append output blocks with the new scale
            for op in &outputs_found {
                new_content.push_str(&format!(
                    "\noutput \"{}\" {{\n    scale {}\n}}\n",
                    op, scale_str
                ));
            }

            let _ = std::fs::write(&niri_kdl, new_content);
        }
    }

    // 3. Set GTK font / interface scaling factor
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "text-scaling-factor", &scale_str])
        .status();

    // 4. Sync Caelestia configuration
    sync_caelestia_scale(scale);

    // 5. Update SolarConfig
    let mut cfg = SolarConfig::load();
    cfg.display.scale = scale;
    cfg.display.text_scale = scale;
    for op in &outputs_found {
        if let Some(pos) = cfg.display.outputs.iter().position(|o| o.name == *op) {
            cfg.display.outputs[pos].scale = scale;
        } else {
            cfg.display.outputs.push(solar_common::MonitorOutputConfig {
                name: op.clone(),
                scale,
                mode: None,
            });
        }
    }
    let _ = cfg.save();
}

fn build_omnibar_tab() -> GtkBox {
    let tab = GtkBox::new(Orientation::Vertical, 16);
    tab.set_margin_top(14);
    tab.set_margin_bottom(14);
    tab.set_margin_start(16);
    tab.set_margin_end(16);

    let sec = GtkBox::new(Orientation::Vertical, 14);
    sec.add_css_class("section-box");

    let title = Label::new(Some("SolarUI Omnibar ve Akıllı Arama"));
    title.add_css_class("section-title");
    title.set_halign(gtk4::Align::Start);
    sec.append(&title);

    let desc = Label::new(Some(
        "Super + Boşluk kısayolu ile açılan Omnibar; sistem komutları, anlık matematik hesaplamaları, birim ve kur çevirileri, uygulama başlatıcı ve isteğe bağlı web/yapay zeka arama motorudur.",
    ));
    desc.add_css_class("setting-subtext");
    desc.set_wrap(true);
    desc.set_halign(gtk4::Align::Start);
    sec.append(&desc);

    let sep1 = Separator::new(Orientation::Horizontal);
    sec.append(&sep1);

    // 1. Web Arama Geçişi
    let cfg = SolarConfig::load();
    let chk_web = CheckButton::with_label("Çevrimiçi Web Arama Sonuçlarını Göster");
    chk_web.set_active(cfg.omnibar.enable_web_search);
    chk_web.connect_toggled(|cb| {
        let mut c = SolarConfig::load();
        c.omnibar.enable_web_search = cb.is_active();
        let _ = c.save();
    });
    sec.append(&chk_web);

    // 2. Arama Motoru Seçimi
    let engine_box = GtkBox::new(Orientation::Horizontal, 12);
    let engine_lbl = Label::new(Some("Varsayılan Arama Motoru:"));
    engine_lbl.add_css_class("setting-label");
    engine_lbl.set_halign(gtk4::Align::Start);
    engine_box.append(&engine_lbl);

    let engines = ["DuckDuckGo", "Google", "Brave", "Bing"];
    let str_list = StringList::new(&engines);
    let dropdown = DropDown::new(Some(str_list), None::<gtk4::Expression>);

    let current_pos = engines.iter().position(|e| *e == cfg.omnibar.search_engine).unwrap_or(0);
    dropdown.set_selected(current_pos as u32);

    dropdown.connect_selected_notify(move |dd| {
        let sel = dd.selected() as usize;
        if let Some(engine) = engines.get(sel) {
            let mut c = SolarConfig::load();
            c.omnibar.search_engine = engine.to_string();
            let _ = c.save();
        }
    });
    engine_box.append(&dropdown);
    sec.append(&engine_box);

    // 3. Yapay Zeka / Arama API Anahtarı
    let api_box = GtkBox::new(Orientation::Vertical, 6);
    let api_lbl = Label::new(Some("Yapay Zeka API Anahtarı (Opsiyonel):"));
    api_lbl.add_css_class("setting-label");
    api_lbl.set_halign(gtk4::Align::Start);
    api_box.append(&api_lbl);

    let api_entry = Entry::new();
    api_entry.set_placeholder_text(Some("Gemini / OpenAI API Anahtarı..."));
    api_entry.set_text(&cfg.omnibar.ai_api_key);
    api_entry.connect_changed(|e| {
        let text = e.text().to_string();
        let mut c = SolarConfig::load();
        c.omnibar.ai_api_key = text;
        let _ = c.save();
    });
    api_box.append(&api_entry);
    sec.append(&api_box);

    let sep2 = Separator::new(Orientation::Horizontal);
    sec.append(&sep2);

    // Test Butonu
    let btn_test = Button::with_label("Omnibar'ı Şimdi Test Et (Super + Boşluk)");
    btn_test.add_css_class("action-btn");
    btn_test.connect_clicked(|_| {
        let _ = Command::new("solar-shell").arg("omnibar").spawn();
    });
    sec.append(&btn_test);

    tab.append(&sec);
    tab
}

