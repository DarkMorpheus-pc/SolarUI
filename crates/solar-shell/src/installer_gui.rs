#![allow(deprecated)]
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, ComboBoxText, Entry, Label, Orientation, PasswordEntry,
    ProgressBar, ScrolledWindow, Stack, StackTransitionType, TextView, Window,
};
use gtk4::{gdk, glib};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;

use crate::living_flow::{
    apply_living_flow_theme, build_living_flow_topbar, load_png_emblem, FluidCloud, FluidCloudTone,
};

const INSTALLER_EMBLEM_PNG: &[u8] =
    include_bytes!("../../../data/icons/living-flow/solar-installer-64.png");

const INSTALLER_EXTRA_CSS: &str = r#"
    .installer-body {
        padding: 16px 24px 20px 24px;
    }

    .installer-hero {
        background: linear-gradient(135deg, rgba(40, 34, 34, 0.92) 0%, rgba(25, 26, 31, 0.96) 100%);
        border: 1px solid rgba(255, 255, 255, 0.09);
        border-radius: 18px;
        padding: 14px 20px;
        margin-bottom: 12px;
    }

    .installer-hero-title {
        font-size: 18px;
        font-weight: 700;
        color: #f5f5f7;
    }

    .installer-hero-sub {
        font-size: 12px;
        color: #a1a1aa;
    }

    .stepper-bar {
        background-color: rgba(24, 26, 31, 0.88);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 999px;
        padding: 6px;
        margin-bottom: 12px;
    }

    .stepper-connector {
        color: rgba(161, 161, 170, 0.40);
        font-weight: 700;
        font-size: 13px;
    }

    .log-view,
    .log-view text {
        background-color: #0e0f12;
        color: #d4d4d8;
        font-family: monospace;
        font-size: 12px;
        padding: 12px;
        border-radius: 14px;
    }

    .log-frame {
        background-color: #0e0f12;
        border: 1px solid rgba(255, 255, 255, 0.09);
        border-radius: 16px;
        padding: 4px;
    }
"#;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProbedDisk {
    pub path: String,
    pub bytes: u64,
    pub model: String,
    pub removable: bool,
    pub read_only: bool,
    pub mounted: bool,
    pub eligible: bool,
    #[serde(default)]
    pub rejection_reasons: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct InstallerAnswersPayload {
    hostname: String,
    locale: String,
    timezone: String,
    keyboard_layout: String,
    username: String,
    full_name: String,
}

fn localize_probe_message(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("disk already contains ") {
        if let Some(count_str) = rest.split_whitespace().next() {
            return format!(
                "Diskte kurulum sırasında silinecek {} mevcut bölüm bulunuyor",
                count_str
            );
        }
    }
    if let Some(rest) = trimmed.strip_prefix("partition ") {
        if let Some((dev, label_part)) = rest.split_once(" is an existing EFI System Partition ") {
            return format!(
                "{} üzerinde mevcut bir EFI Sistem Bölümü {} yer alıyor",
                dev, label_part
            );
        }
        if let Some((dev, label_part)) = rest.split_once(" carries Windows/Microsoft label ") {
            return format!(
                "{} bölümü Windows/Microsoft etiketi ({}) taşıyor",
                dev, label_part
            );
        }
    }
    match trimmed {
        "smaller than 16 GiB" => "Disk kapasitesi minimum 16 GiB sınırının altında".to_string(),
        "read-only device" => "Salt okunur (yazma korumalı) depolama aygıtı".to_string(),
        "device or partition is mounted" => {
            "Bu disk şu anda çalışan sistem tarafından kullanılıyor (aktif kök/EFI bölümü bağlı)"
                .to_string()
        }
        other if other.contains("removable") => {
            "Çıkarılabilir aygıt algılandı (kurulum için aşağıdaki izin kutucuğunu işaretleyin)"
                .to_string()
        }
        other => other.to_string(),
    }
}

fn format_disk_feedback(d: &ProbedDisk) -> (String, bool) {
    let mut parts = Vec::new();
    for r in &d.rejection_reasons {
        parts.push(localize_probe_message(r));
    }
    for w in &d.warnings {
        parts.push(localize_probe_message(w));
    }
    if parts.is_empty() {
        (
            "✓ Seçilen disk temiz GPT + Limine UEFI/BIOS + fs-verity kurulumu için tamamen hazır."
                .to_string(),
            true,
        )
    } else if !d.eligible {
        (format!("Durum: {}", parts.join("  •  ")), false)
    } else {
        (format!("Bilgi: {}", parts.join("  •  ")), false)
    }
}

fn probe_disks() -> Vec<ProbedDisk> {
    for cmd_pair in [
        vec!["solar-installer", "probe", "--json"],
        vec!["sudo", "-n", "solar-installer", "probe", "--json"],
    ] {
        if let Ok(out) = Command::new(cmd_pair[0]).args(&cmd_pair[1..]).output() {
            if out.status.success() {
                if let Ok(list) = serde_json::from_slice::<Vec<ProbedDisk>>(&out.stdout) {
                    if !list.is_empty() {
                        return list;
                    }
                }
            }
        }
    }

    let mut disks = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("loop")
                || name.starts_with("ram")
                || name.starts_with("zram")
                || name.starts_with("sr")
                || name.starts_with("dm-")
            {
                continue;
            }
            let sectors = std::fs::read_to_string(entry.path().join("size"))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);
            let bytes = sectors * 512;
            let model = std::fs::read_to_string(entry.path().join("device/model"))
                .map(|s| s.trim().to_string())
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "VirtIO / NVMe Depolama".to_string());
            let removable = std::fs::read_to_string(entry.path().join("removable"))
                .map(|s| s.trim() == "1")
                .unwrap_or(false);
            let ro = std::fs::read_to_string(entry.path().join("ro"))
                .map(|s| s.trim() == "1")
                .unwrap_or(false);
            let eligible = bytes >= 16 * 1024 * 1024 * 1024 && !ro;
            disks.push(ProbedDisk {
                path: format!("/dev/{}", name),
                bytes,
                model,
                removable,
                read_only: ro,
                mounted: false,
                eligible,
                rejection_reasons: if eligible {
                    vec![]
                } else {
                    vec!["Minimum 16 GiB yazılabilir disk gereklidir".to_string()]
                },
                warnings: vec![],
            });
        }
    }
    disks
}

pub fn launch_installer_window() {
    glib::set_prgname(Some("solar-installer-gui"));
    glib::set_application_name("BlazeOS Native Kurulum Sihirbazı");

    if gtk4::init().is_err() {
        eprintln!("Failed to initialize GTK4 for Solar Installer GUI");
        return;
    }

    apply_living_flow_theme(INSTALLER_EXTRA_CSS);

    let main_loop = glib::MainLoop::new(None, false);
    let window = Window::builder()
        .title("Blaze Universe — Solar Kurulum Sihirbazı")
        .decorated(false)
        .default_width(920)
        .default_height(640)
        .resizable(false)
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

    let fluid_cloud = FluidCloud::new("Adım 1/3 • Depolama Hazır");
    let emblem = load_png_emblem(INSTALLER_EMBLEM_PNG, 34);
    let loop_top = main_loop.clone();
    let topbar = build_living_flow_topbar(
        &window,
        Some(emblem),
        "Blaze Universe — Solar Kurulum Sihirbazı",
        "Living Flow Design • Limine 12.x UEFI/BIOS • fs-verity & BERP",
        &fluid_cloud,
        Some(Box::new(move || loop_top.quit())),
    );
    shell_box.append(&topbar);

    let body = GtkBox::new(Orientation::Vertical, 0);
    body.add_css_class("installer-body");
    body.set_vexpand(true);

    // Luminous Hero Card
    let hero = GtkBox::new(Orientation::Vertical, 3);
    hero.add_css_class("installer-hero");
    let h_title = Label::new(Some("BlazeOS Native Kurulum ve Disk Yapılandırma Merkezi"));
    h_title.add_css_class("installer-hero-title");
    h_title.set_halign(gtk4::Align::Start);
    let h_sub = Label::new(Some(
        "Kriptografik bütünlük doğrulanmış kök dosya sistemi • Tam Donanım Kalkanı • Atomik Kurtarma Protokolü",
    ));
    h_sub.add_css_class("installer-hero-sub");
    h_sub.set_halign(gtk4::Align::Start);
    hero.append(&h_title);
    hero.append(&h_sub);
    body.append(&hero);

    // ColorOS 17 Capsule Stepper Bar + Fluid Stack
    let stepper = GtkBox::new(Orientation::Horizontal, 10);
    stepper.add_css_class("stepper-bar");
    stepper.set_halign(gtk4::Align::Center);

    let step_btn1 = Button::with_label("①  Hedef Disk Seçimi");
    step_btn1.add_css_class("lf-step-pill");
    step_btn1.add_css_class("active");

    let sep1 = Label::new(Some("───"));
    sep1.add_css_class("stepper-connector");

    let step_btn2 = Button::with_label("②  Kimlik & Bölgesel Ayarlar");
    step_btn2.add_css_class("lf-step-pill");

    let sep2 = Label::new(Some("───"));
    sep2.add_css_class("stepper-connector");

    let step_btn3 = Button::with_label("③  Doğrulama & Kurulum");
    step_btn3.add_css_class("lf-step-pill");

    stepper.append(&step_btn1);
    stepper.append(&sep1);
    stepper.append(&step_btn2);
    stepper.append(&sep2);
    stepper.append(&step_btn3);
    body.append(&stepper);

    let stack = Stack::new();
    stack.set_transition_type(StackTransitionType::SlideLeftRight);
    stack.set_transition_duration(280);
    stack.set_vexpand(true);

    let switch_step = {
        let stack_c = stack.clone();
        let b1 = step_btn1.clone();
        let b2 = step_btn2.clone();
        let b3 = step_btn3.clone();
        let cloud = fluid_cloud.clone();
        Rc::new(move |idx: usize| {
            b1.remove_css_class("active");
            b2.remove_css_class("active");
            b3.remove_css_class("active");
            match idx {
                0 => {
                    b1.add_css_class("active");
                    stack_c.set_visible_child_name("step1");
                    cloud.pulse("Adım 1/3 • Hedef Disk Seçimi", FluidCloudTone::Info, false);
                }
                1 => {
                    b2.add_css_class("active");
                    stack_c.set_visible_child_name("step2");
                    cloud.pulse(
                        "Adım 2/3 • Kullanıcı & Bölge",
                        FluidCloudTone::Info,
                        false,
                    );
                }
                _ => {
                    b3.add_css_class("active");
                    stack_c.set_visible_child_name("step3");
                    cloud.pulse("Adım 3/3 • Onay & Kurulum", FluidCloudTone::Success, false);
                }
            }
        })
    };

    {
        let sw = switch_step.clone();
        step_btn1.connect_clicked(move |_| sw(0));
    }
    {
        let sw = switch_step.clone();
        step_btn2.connect_clicked(move |_| sw(1));
    }
    {
        let sw = switch_step.clone();
        step_btn3.connect_clicked(move |_| sw(2));
    }

    let disks_state: Rc<RefCell<Vec<ProbedDisk>>> = Rc::new(RefCell::new(Vec::new()));

    // ── STEP 1: HEDEF DİSK VE BÖLÜMLEME ──────────────────────────────────────
    let page_disk = GtkBox::new(Orientation::Vertical, 12);
    page_disk.set_margin_top(6);

    let disk_card = GtkBox::new(Orientation::Vertical, 12);
    disk_card.add_css_class("installer-card");

    let d_head = Label::new(Some("1. Hedef Depolama Aygıtını Seçin"));
    d_head.add_css_class("section-heading");
    d_head.set_halign(gtk4::Align::Start);
    disk_card.append(&d_head);

    let d_desc = Label::new(Some(
        "Solar Installer seçilen diskte otomatik olarak GPT bölüm tablosu, 512 MiB FAT32 EFI Sistem Bölümü (ESP), 1 MiB BIOS Boot ve geri kalan alanda BlazeOS Native (ext4, fs-verity & BERP) kök bölümünü oluşturur.",
    ));
    d_desc.add_css_class("section-desc");
    d_desc.set_wrap(true);
    d_desc.set_xalign(0.0);
    d_desc.set_halign(gtk4::Align::Start);
    disk_card.append(&d_desc);

    let disk_combo = ComboBoxText::new();
    let disk_warning_lbl = Label::new(Some(""));
    disk_warning_lbl.add_css_class("disk-badge-warn");
    disk_warning_lbl.set_halign(gtk4::Align::Fill);
    disk_warning_lbl.set_xalign(0.0);
    disk_warning_lbl.set_wrap(true);

    let apply_badge_style = |lbl: &Label, is_clean: bool| {
        lbl.remove_css_class("disk-badge-ok");
        lbl.remove_css_class("disk-badge-warn");
        if is_clean {
            lbl.add_css_class("disk-badge-ok");
        } else {
            lbl.add_css_class("disk-badge-warn");
        }
    };

    let populate_disks = {
        let disks_state = disks_state.clone();
        let disk_combo = disk_combo.clone();
        let disk_warning_lbl = disk_warning_lbl.clone();
        move || {
            disk_combo.remove_all();
            let list = probe_disks();
            *disks_state.borrow_mut() = list.clone();
            for d in &list {
                let gib = (d.bytes as f64) / (1024.0 * 1024.0 * 1024.0);
                let model_disp = if d.model.trim().is_empty() {
                    "Depolama Aygıtı"
                } else {
                    d.model.trim()
                };
                let status = if d.eligible {
                    "Kuruluma Uygun"
                } else if d.mounted {
                    "Aktif Sistem Diski"
                } else {
                    "Uygun Değil"
                };
                let label = format!(
                    "{} — {} ({:.1} GiB)  [{}]",
                    d.path, model_disp, gib, status
                );
                disk_combo.append(Some(&d.path), &label);
            }
            if let Some(first) = list.iter().find(|d| d.eligible).or_else(|| list.first()) {
                disk_combo.set_active_id(Some(&first.path));
                let (txt, is_clean) = format_disk_feedback(first);
                apply_badge_style(&disk_warning_lbl, is_clean);
                disk_warning_lbl.set_text(&txt);
            } else {
                apply_badge_style(&disk_warning_lbl, false);
                disk_warning_lbl.set_text("Kuruluma uygun bir depolama aygıtı tespit edilemedi.");
            }
            list.len()
        }
    };
    let _ = populate_disks();

    {
        let disks_state = disks_state.clone();
        let disk_warning_lbl = disk_warning_lbl.clone();
        disk_combo.connect_changed(move |cb| {
            if let Some(id) = cb.active_id() {
                if let Some(d) = disks_state.borrow().iter().find(|x| x.path == id.as_str()) {
                    let (txt, is_clean) = format_disk_feedback(d);
                    apply_badge_style(&disk_warning_lbl, is_clean);
                    disk_warning_lbl.set_text(&txt);
                }
            }
        });
    }

    let disk_row = GtkBox::new(Orientation::Horizontal, 12);
    disk_combo.set_hexpand(true);
    disk_row.append(&disk_combo);

    let refresh_btn = Button::with_label("↻  Diskleri Yenile");
    refresh_btn.add_css_class("lf-btn-glass");
    let pop_clone = populate_disks.clone();
    let cloud_refresh = fluid_cloud.clone();
    refresh_btn.connect_clicked(move |_| {
        let count = pop_clone();
        cloud_refresh.pulse(
            &format!("{} depolama aygıtı tarandı", count),
            FluidCloudTone::Success,
            false,
        );
    });
    disk_row.append(&refresh_btn);
    disk_card.append(&disk_row);
    disk_card.append(&disk_warning_lbl);

    let chk_removable = CheckButton::with_label(
        "Çıkarılabilir (USB / Harici NVMe) disklere kuruluma izin ver (--allow-removable)",
    );
    let cloud_rem = fluid_cloud.clone();
    chk_removable.connect_toggled(move |cb| {
        if cb.is_active() {
            cloud_rem.pulse(
                "Harici USB/NVMe kurulum izni etkin",
                FluidCloudTone::Warning,
                false,
            );
        } else {
            cloud_rem.pulse(
                "Yalnızca dahili disk koruması etkin",
                FluidCloudTone::Info,
                false,
            );
        }
    });
    disk_card.append(&chk_removable);

    page_disk.append(&disk_card);

    let nav_disk = GtkBox::new(Orientation::Horizontal, 10);
    nav_disk.set_halign(gtk4::Align::End);
    let to_step2_btn = Button::with_label("İleri: Kullanıcı & Bölge Ayarları  →");
    to_step2_btn.add_css_class("lf-btn-flame");
    let sw_to_2 = switch_step.clone();
    to_step2_btn.connect_clicked(move |_| {
        sw_to_2(1);
    });
    nav_disk.append(&to_step2_btn);
    page_disk.append(&nav_disk);

    stack.add_named(&page_disk, Some("step1"));

    // ── STEP 2: KULLANICI, BİLGİSAYAR ADI VE BÖLGE ──────────────────────────
    let page_user = GtkBox::new(Orientation::Vertical, 10);
    page_user.set_margin_top(4);

    let user_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .build();

    let user_card = GtkBox::new(Orientation::Vertical, 10);
    user_card.add_css_class("installer-card");

    let u_head = Label::new(Some("2. Yönetici Hesabı ve Bölgesel Yapılandırma"));
    u_head.add_css_class("section-heading");
    u_head.set_halign(gtk4::Align::Start);
    user_card.append(&u_head);

    let entry_fullname = Entry::builder()
        .placeholder_text("Tam Adınız (Örn: Dark Morpheus)")
        .text("Blaze Kullanıcısı")
        .build();
    let entry_username = Entry::builder()
        .placeholder_text("Kullanıcı Adı (küçük harf, örn: blaze)")
        .text("blaze")
        .build();
    let entry_hostname = Entry::builder()
        .placeholder_text("Bilgisayar Adı (örn: blaze-workstation)")
        .text("blazeos")
        .build();
    let entry_pass = PasswordEntry::builder()
        .placeholder_text("Parola (En az 8 karakter — YESCRYPT ile özetlenir)")
        .show_peek_icon(true)
        .build();
    let entry_pass2 = PasswordEntry::builder()
        .placeholder_text("Parolayı Tekrar Girin")
        .show_peek_icon(true)
        .build();

    let combo_kb = ComboBoxText::new();
    combo_kb.append(Some("tr"), "Türkçe (Q Klavye — tr)");
    combo_kb.append(Some("us"), "English (US — us)");
    combo_kb.append(Some("de"), "Deutsch (de)");
    combo_kb.append(Some("fr"), "Français (fr)");
    combo_kb.set_active_id(Some("tr"));
    let cloud_kb = fluid_cloud.clone();
    combo_kb.connect_changed(move |cb| {
        if let Some(id) = cb.active_id() {
            let idx = match id.as_str() {
                "tr" => "0",
                "us" => "1",
                _ => "0",
            };
            let _ = Command::new("niri")
                .args(["msg", "action", "switch-layout", idx])
                .status();
            cloud_kb.pulse(
                &format!("Klavye düzeni '{}' olarak ayarlandı", id),
                FluidCloudTone::Success,
                false,
            );
        }
    });

    let combo_tz = ComboBoxText::new();
    combo_tz.append(Some("Europe/Istanbul"), "Europe/Istanbul (Türkiye — GMT+3)");
    combo_tz.append(Some("UTC"), "UTC (Evrensel Saat)");
    combo_tz.append(Some("Europe/Berlin"), "Europe/Berlin (Orta Avrupa)");
    combo_tz.append(Some("Europe/London"), "Europe/London (Birleşik Krallık)");
    combo_tz.append(Some("America/New_York"), "America/New_York (ABD Doğu)");
    combo_tz.set_active_id(Some("Europe/Istanbul"));

    let combo_locale = ComboBoxText::new();
    combo_locale.append(Some("tr_TR.UTF-8"), "Türkçe (tr_TR.UTF-8)");
    combo_locale.append(Some("en_US.UTF-8"), "English (en_US.UTF-8)");
    combo_locale.set_active_id(Some("tr_TR.UTF-8"));

    for (lbl_txt, widget) in [
        ("Tam Ad:", entry_fullname.clone().upcast::<gtk4::Widget>()),
        ("Kullanıcı Adı:", entry_username.clone().upcast::<gtk4::Widget>()),
        ("Bilgisayar Adı (Hostname):", entry_hostname.clone().upcast::<gtk4::Widget>()),
        ("Kullanıcı Parolası (min 8 krkt):", entry_pass.clone().upcast::<gtk4::Widget>()),
        ("Parola Doğrulama:", entry_pass2.clone().upcast::<gtk4::Widget>()),
        ("Klavye Düzeni:", combo_kb.clone().upcast::<gtk4::Widget>()),
        ("Saat Dilimi:", combo_tz.clone().upcast::<gtk4::Widget>()),
        ("Sistem Dili:", combo_locale.clone().upcast::<gtk4::Widget>()),
    ] {
        let row = GtkBox::new(Orientation::Horizontal, 12);
        let lbl = Label::new(Some(lbl_txt));
        lbl.add_css_class("lf-label");
        lbl.set_width_chars(26);
        lbl.set_xalign(0.0);
        widget.set_hexpand(true);
        row.append(&lbl);
        row.append(&widget);
        user_card.append(&row);
    }

    user_scroll.set_child(Some(&user_card));
    page_user.append(&user_scroll);

    let nav_user = GtkBox::new(Orientation::Horizontal, 10);
    nav_user.set_halign(gtk4::Align::End);
    let back_step1_btn = Button::with_label("←  Geri");
    back_step1_btn.add_css_class("lf-btn-glass");
    let sw_back_1 = switch_step.clone();
    back_step1_btn.connect_clicked(move |_| {
        sw_back_1(0);
    });
    let to_step3_btn = Button::with_label("İleri: Kurulum Özeti & Onay  →");
    to_step3_btn.add_css_class("lf-btn-flame");
    let sw_to_3 = switch_step.clone();
    to_step3_btn.connect_clicked(move |_| {
        sw_to_3(2);
    });
    nav_user.append(&back_step1_btn);
    nav_user.append(&to_step3_btn);
    page_user.append(&nav_user);

    stack.add_named(&page_user, Some("step2"));

    // ── STEP 3: ÖZET, DOĞRULAMA VE KURULUM (`solar-installer apply`) ────────
    let page_apply = GtkBox::new(Orientation::Vertical, 10);
    page_apply.set_margin_top(4);

    let apply_card = GtkBox::new(Orientation::Vertical, 10);
    apply_card.add_css_class("installer-card");
    apply_card.set_vexpand(true);

    let a_head = Label::new(Some("3. Kurulum Planı ve Doğrulanmış Disk Yazımı"));
    a_head.add_css_class("section-heading");
    a_head.set_halign(gtk4::Align::Start);
    apply_card.append(&a_head);

    let chk_confirm_erase = CheckButton::with_label(
        "Seçilen hedef diskteki tüm mevcut bölümlerin silineceğini ve BlazeOS Native kurulacağını onaylıyorum.",
    );
    apply_card.append(&chk_confirm_erase);

    let progress = ProgressBar::new();
    progress.set_show_text(true);
    progress.set_text(Some("Kurulum için onayınız bekleniyor..."));
    apply_card.append(&progress);

    let log_scroll = ScrolledWindow::builder()
        .min_content_height(175)
        .vexpand(true)
        .build();
    log_scroll.add_css_class("log-frame");
    let log_view = TextView::new();
    log_view.set_editable(false);
    log_view.set_monospace(true);
    log_view.add_css_class("log-view");
    log_scroll.set_child(Some(&log_view));
    apply_card.append(&log_scroll);

    let action_row = GtkBox::new(Orientation::Horizontal, 10);
    let back_step2_btn = Button::with_label("←  Geri");
    back_step2_btn.add_css_class("lf-btn-glass");
    let sw_back_2 = switch_step.clone();
    back_step2_btn.connect_clicked(move |_| {
        sw_back_2(1);
    });
    action_row.append(&back_step2_btn);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    action_row.append(&spacer);

    let plan_btn = Button::with_label("Kurulum Planını Doğrula (Dry-Run)");
    plan_btn.add_css_class("lf-btn-glass");

    let install_btn = Button::with_label("BlazeOS'u Şimdi Diske Kur");
    install_btn.add_css_class("lf-btn-danger");

    let reboot_btn = Button::with_label("Şimdi Yeniden Başlat");
    reboot_btn.add_css_class("lf-btn-primary");
    reboot_btn.set_visible(false);
    reboot_btn.connect_clicked(|_| {
        let _ = Command::new("sudo").args(["-n", "reboot"]).spawn();
        let _ = Command::new("systemctl").arg("reboot").spawn();
    });

    // Plan button logic
    {
        let disk_combo = disk_combo.clone();
        let log_view = log_view.clone();
        let cloud_plan = fluid_cloud.clone();
        plan_btn.connect_clicked(move |_| {
            let target = disk_combo
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "/dev/vdb".to_string());
            cloud_plan.pulse("Dry-Run kurulum planı denetleniyor...", FluidCloudTone::Info, false);
            let out = Command::new("sudo")
                .args(["-n", "solar-installer", "plan", "--target", &target])
                .output();
            let text = match out {
                Ok(o) => format!(
                    "[PLAN] Hedef: {}\n{}{}",
                    target,
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                ),
                Err(e) => format!("[HATA] solar-installer plan çalıştırılamadı: {}", e),
            };
            log_view.buffer().set_text(&text);
        });
    }

    // Install button logic
    {
        let disk_combo = disk_combo.clone();
        let chk_removable = chk_removable.clone();
        let chk_confirm_erase = chk_confirm_erase.clone();
        let entry_fullname = entry_fullname.clone();
        let entry_username = entry_username.clone();
        let entry_hostname = entry_hostname.clone();
        let entry_pass = entry_pass.clone();
        let entry_pass2 = entry_pass2.clone();
        let combo_kb = combo_kb.clone();
        let combo_tz = combo_tz.clone();
        let combo_locale = combo_locale.clone();
        let progress = progress.clone();
        let log_view = log_view.clone();
        let reboot_btn = reboot_btn.clone();
        let cloud_inst = fluid_cloud.clone();

        install_btn.connect_clicked(move |btn| {
            if !chk_confirm_erase.is_active() {
                cloud_inst.pulse("Disk silme onayı gerekli", FluidCloudTone::Warning, false);
                log_view
                    .buffer()
                    .set_text("[UYARI] Lütfen önce disk silme onay kutucuğunu işaretleyin.");
                return;
            }
            let pass1 = entry_pass.text().to_string();
            let pass2 = entry_pass2.text().to_string();
            if pass1.len() < 8 {
                cloud_inst.pulse("Parola en az 8 karakter olmalı", FluidCloudTone::Warning, false);
                log_view
                    .buffer()
                    .set_text("[HATA] Parola en az 8 karakter uzunluğunda olmalıdır.");
                return;
            }
            if pass1 != pass2 {
                cloud_inst.pulse("Parolalar eşleşmiyor", FluidCloudTone::Danger, false);
                log_view
                    .buffer()
                    .set_text("[HATA] Girdiğiniz parolalar birbiriyle eşleşmiyor.");
                return;
            }
            let Some(target) = disk_combo.active_id().map(|s| s.to_string()) else {
                log_view
                    .buffer()
                    .set_text("[HATA] Lütfen 1. adımdan geçerli bir hedef disk seçin.");
                return;
            };

            let payload = InstallerAnswersPayload {
                hostname: entry_hostname.text().trim().to_string(),
                locale: combo_locale
                    .active_id()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "tr_TR.UTF-8".to_string()),
                timezone: combo_tz
                    .active_id()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Europe/Istanbul".to_string()),
                keyboard_layout: combo_kb
                    .active_id()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "tr".to_string()),
                username: entry_username.text().trim().to_string(),
                full_name: entry_fullname.text().trim().to_string(),
            };

            let answers_path = PathBuf::from("/tmp/solar-installer-answers.json");
            if let Ok(json_bytes) = serde_json::to_vec_pretty(&payload) {
                let _ = std::fs::write(&answers_path, json_bytes);
            }

            let allow_rem = chk_removable.is_active();
            btn.set_sensitive(false);
            cloud_inst.pulse("Kurulum devam ediyor...", FluidCloudTone::Info, true);
            progress.set_fraction(0.15);
            progress.set_text(Some("Kurulum başlatıldı: Disk bölümleniyor ve doğrulanıyor..."));

            let (tx, rx) = async_channel::unbounded::<(f64, String, bool)>();
            std::thread::spawn(move || {
                let mut args = vec![
                    "-n".to_string(),
                    "solar-installer".to_string(),
                    "apply".to_string(),
                    "--target".to_string(),
                    target.clone(),
                    "--confirm-erase".to_string(),
                    target.clone(),
                    "--answers".to_string(),
                    answers_path.to_string_lossy().to_string(),
                    "--password-stdin".to_string(),
                ];
                if allow_rem {
                    args.push("--allow-removable".to_string());
                }

                let _ = tx.send_blocking((
                    0.25,
                    format!("[BAŞLATILDI] Hedef disk: {}\n", target),
                    false,
                ));

                let child_res = Command::new("sudo")
                    .args(&args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn();

                match child_res {
                    Ok(mut child) => {
                        if let Some(mut stdin) = child.stdin.take() {
                            let _ = stdin.write_all(format!("{}\n", pass1).as_bytes());
                        }
                        if let Some(stdout) = child.stdout.take() {
                            let reader = BufReader::new(stdout);
                            for line in reader.lines().flatten() {
                                let _ = tx.send_blocking((0.65, format!("{}\n", line), false));
                            }
                        }
                        let out = child.wait_with_output();
                        let _ = std::fs::remove_file(&answers_path);
                        match out {
                            Ok(res) if res.status.success() => {
                                let _ = tx.send_blocking((
                                    1.0,
                                    "\n[TAMAMLANDI] BlazeOS Native başarıyla diske kuruldu! Sistemi yeniden başlatabilirsiniz.\n".to_string(),
                                    true,
                                ));
                            }
                            Ok(res) => {
                                let err_txt = String::from_utf8_lossy(&res.stderr);
                                let _ = tx.send_blocking((
                                    0.0,
                                    format!("\n[HATA] Kurulum başarısız oldu:\n{}\n", err_txt),
                                    false,
                                ));
                            }
                            Err(e) => {
                                let _ = tx.send_blocking((
                                    0.0,
                                    format!("\n[HATA] Süreç beklenirken hata: {}\n", e),
                                    false,
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send_blocking((
                            0.0,
                            format!("[HATA] solar-installer başlatılamadı: {}\n", e),
                            false,
                        ));
                    }
                }
            });

            let progress_ui = progress.clone();
            let log_ui = log_view.clone();
            let reboot_ui = reboot_btn.clone();
            let btn_ui = btn.clone();
            let cloud_done = cloud_inst.clone();
            glib::MainContext::default().spawn_local(async move {
                let mut accumulated = String::new();
                while let Ok((frac, msg, done)) = rx.recv().await {
                    accumulated.push_str(&msg);
                    log_ui.buffer().set_text(&accumulated);
                    progress_ui.set_fraction(frac);
                    if done {
                        progress_ui.set_text(Some("Kurulum Başarıyla Tamamlandı!"));
                        cloud_done.pulse("BlazeOS Native Kuruldu!", FluidCloudTone::Success, true);
                        reboot_ui.set_visible(true);
                    } else if frac == 0.0 {
                        progress_ui.set_text(Some("Kurulum durduruldu / hata oluştu"));
                        cloud_done.pulse("Kurulum hatası algılandı", FluidCloudTone::Danger, false);
                        btn_ui.set_sensitive(true);
                    }
                }
            });
        });
    }

    action_row.append(&plan_btn);
    action_row.append(&install_btn);
    action_row.append(&reboot_btn);
    apply_card.append(&action_row);
    page_apply.append(&apply_card);

    stack.add_named(&page_apply, Some("step3"));
    stack.set_visible_child_name("step1");

    body.append(&stack);
    shell_box.append(&body);
    window.set_child(Some(&shell_box));
    window.present();
    main_loop.run();
}
