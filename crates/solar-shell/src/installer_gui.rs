#![allow(deprecated)]
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, ComboBoxText, CssProvider, Entry,
    Label, Notebook, Orientation, PasswordEntry, ProgressBar, ScrolledWindow, TextView, Window,
};
use gtk4::{gdk, glib};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;

const INSTALLER_CSS: &str = r#"
window {
    background-color: #0b0e14;
    color: #f3f4f6;
}
.installer-root {
    background: linear-gradient(145deg, #0b0e14 0%, #111827 55%, #171324 100%);
    padding: 24px;
}
.installer-header {
    background-color: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 158, 100, 0.28);
    border-radius: 16px;
    padding: 18px 22px;
    margin-bottom: 14px;
}
.installer-title {
    font-size: 22px;
    font-weight: 800;
    color: #ff9e64;
}
.installer-subtitle {
    font-size: 13px;
    color: #9ca3af;
}
.installer-card {
    background-color: rgba(17, 24, 39, 0.85);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 14px;
    padding: 18px;
    margin-bottom: 12px;
}
.section-heading {
    font-size: 16px;
    font-weight: 700;
    color: #f9fafb;
    margin-bottom: 4px;
}
.section-desc {
    font-size: 12px;
    color: #9ca3af;
    margin-bottom: 10px;
}
.disk-badge-ok {
    background-color: rgba(16, 185, 129, 0.18);
    color: #34d399;
    border-radius: 8px;
    padding: 4px 10px;
    font-weight: 700;
    font-size: 12px;
}
.disk-badge-warn {
    background-color: rgba(245, 158, 11, 0.18);
    color: #fbbf24;
    border-radius: 8px;
    padding: 4px 10px;
    font-weight: 700;
    font-size: 12px;
}
.primary-btn {
    background: linear-gradient(90deg, #ff7a2f 0%, #ff9e64 100%);
    color: #0b0e14;
    font-weight: 800;
    border-radius: 10px;
    padding: 10px 20px;
    border: none;
}
.secondary-btn {
    background-color: rgba(255, 255, 255, 0.08);
    color: #e5e7eb;
    font-weight: 600;
    border-radius: 10px;
    padding: 10px 18px;
    border: 1px solid rgba(255, 255, 255, 0.14);
}
.danger-btn {
    background: linear-gradient(90deg, #dc2626 0%, #ef4444 100%);
    color: #ffffff;
    font-weight: 800;
    border-radius: 10px;
    padding: 10px 22px;
    border: none;
}
.log-view {
    background-color: #07090d;
    color: #a7f3d0;
    font-family: monospace;
    font-size: 12px;
    padding: 12px;
    border-radius: 10px;
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

fn probe_disks() -> Vec<ProbedDisk> {
    // 1. Try solar-installer probe --json
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

    // 2. Fallback /sys/block scan if solar-installer is not in PATH or non-root preview
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
                .unwrap_or_else(|_| "Sanal / NVMe Disk".to_string());
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

    let provider = CssProvider::new();
    provider.load_from_data(INSTALLER_CSS);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let main_loop = glib::MainLoop::new(None, false);
    let window = Window::builder()
        .title("BlazeOS Native — Solar Kurulum Sihirbazı")
        .default_width(920)
        .default_height(680)
        .build();

    let loop_quit = main_loop.clone();
    window.connect_close_request(move |_| {
        loop_quit.quit();
        glib::Propagation::Proceed
    });

    let root = GtkBox::new(Orientation::Vertical, 12);
    root.add_css_class("installer-root");

    // Header
    let header = GtkBox::new(Orientation::Vertical, 4);
    header.add_css_class("installer-header");
    let title = Label::new(Some("BlazeOS Native — Solar Kurulum Sihirbazı"));
    title.add_css_class("installer-title");
    title.set_halign(gtk4::Align::Start);
    let subtitle = Label::new(Some(
        "Kriptografik bütünlük doğrulanmış kök dosya sistemi • Limine 12.x UEFI/BIOS • BERP Kurtarma • Tam Donanım Kalkanı",
    ));
    subtitle.add_css_class("installer-subtitle");
    subtitle.set_halign(gtk4::Align::Start);
    header.append(&title);
    header.append(&subtitle);
    root.append(&header);

    let notebook = Notebook::new();
    notebook.set_vexpand(true);

    let disks_state: Rc<RefCell<Vec<ProbedDisk>>> = Rc::new(RefCell::new(probe_disks()));

    // ── STEP 1: HEDEF DİSK VE BÖLÜMLEME ──────────────────────────────────────
    let page_disk = GtkBox::new(Orientation::Vertical, 12);
    page_disk.set_margin_top(12);
    page_disk.set_margin_bottom(12);
    page_disk.set_margin_start(12);
    page_disk.set_margin_end(12);

    let disk_card = GtkBox::new(Orientation::Vertical, 10);
    disk_card.add_css_class("installer-card");

    let d_head = Label::new(Some("1. Hedef Depolama Aygıtını Seçin"));
    d_head.add_css_class("section-heading");
    d_head.set_halign(gtk4::Align::Start);
    disk_card.append(&d_head);

    let d_desc = Label::new(Some(
        "Solar Installer seçilen diskte otomatik olarak GPT bölüm tablosu, 512 MiB FAT32 EFI Sistem Bölümü (ESP), 1 MiB BIOS Boot ve geri kalan alanda BlazeOS Native (ext4, fs-verity &BERP) kök bölümünü oluşturur.",
    ));
    d_desc.add_css_class("section-desc");
    d_desc.set_wrap(true);
    d_desc.set_halign(gtk4::Align::Start);
    disk_card.append(&d_desc);

    let disk_combo = ComboBoxText::new();
    let disk_warning_lbl = Label::new(Some(""));
    disk_warning_lbl.add_css_class("disk-badge-warn");
    disk_warning_lbl.set_halign(gtk4::Align::Start);
    disk_warning_lbl.set_wrap(true);

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
                let status = if d.eligible { "Uygun" } else { "Uygun Değil" };
                let label = format!("{} — {} ({:.1} GiB) [{}]", d.path, d.model, gib, status);
                disk_combo.append(Some(&d.path), &label);
            }
            if let Some(first) = list.iter().find(|d| d.eligible).or_else(|| list.first()) {
                disk_combo.set_active_id(Some(&first.path));
                if !first.warnings.is_empty() {
                    disk_warning_lbl.set_text(&format!("Uyarılar: {}", first.warnings.join(" | ")));
                } else if !first.rejection_reasons.is_empty() {
                    disk_warning_lbl.set_text(&format!(
                        "Engeller: {}",
                        first.rejection_reasons.join(" | ")
                    ));
                } else {
                    disk_warning_lbl.set_text("Seçilen disk temiz kurulum için hazır.");
                }
            } else {
                disk_warning_lbl.set_text("Kuruluma uygun bir hedef disk tespit edilemedi.");
            }
        }
    };
    populate_disks();

    {
        let disks_state = disks_state.clone();
        let disk_warning_lbl = disk_warning_lbl.clone();
        disk_combo.connect_changed(move |cb| {
            if let Some(id) = cb.active_id() {
                if let Some(d) = disks_state.borrow().iter().find(|x| x.path == id.as_str()) {
                    if !d.warnings.is_empty() {
                        disk_warning_lbl.set_text(&format!(
                            "Uyarılar: {}",
                            d.warnings.join(" | ")
                        ));
                    } else if !d.rejection_reasons.is_empty() {
                        disk_warning_lbl.set_text(&format!(
                            "Engeller: {}",
                            d.rejection_reasons.join(" | ")
                        ));
                    } else {
                        disk_warning_lbl.set_text("Seçilen disk temiz kurulum için hazır.");
                    }
                }
            }
        });
    }

    let disk_row = GtkBox::new(Orientation::Horizontal, 10);
    disk_combo.set_hexpand(true);
    disk_row.append(&disk_combo);

    let refresh_btn = Button::with_label("Diskleri Yenile");
    refresh_btn.add_css_class("secondary-btn");
    let pop_clone = populate_disks.clone();
    refresh_btn.connect_clicked(move |_| {
        pop_clone();
    });
    disk_row.append(&refresh_btn);
    disk_card.append(&disk_row);
    disk_card.append(&disk_warning_lbl);

    let chk_removable = CheckButton::with_label(
        "Çıkarılabilir (USB / Harici NVMe) disklere kuruluma izin ver (--allow-removable)",
    );
    disk_card.append(&chk_removable);

    page_disk.append(&disk_card);

    let nav_disk = GtkBox::new(Orientation::Horizontal, 10);
    nav_disk.set_halign(gtk4::Align::End);
    let to_step2_btn = Button::with_label("İleri: Kullanıcı & Bölge Ayarları →");
    to_step2_btn.add_css_class("primary-btn");
    let nb_ref1 = notebook.clone();
    to_step2_btn.connect_clicked(move |_| {
        nb_ref1.set_current_page(Some(1));
    });
    nav_disk.append(&to_step2_btn);
    page_disk.append(&nav_disk);

    notebook.append_page(&page_disk, Some(&Label::new(Some("1. Hedef Disk"))));

    // ── STEP 2: KULLANICI, BİLGİSAYAR ADI VE BÖLGE ──────────────────────────
    let page_user = GtkBox::new(Orientation::Vertical, 12);
    page_user.set_margin_top(12);
    page_user.set_margin_bottom(12);
    page_user.set_margin_start(12);
    page_user.set_margin_end(12);

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
    combo_kb.connect_changed(|cb| {
        if let Some(id) = cb.active_id() {
            let idx = match id.as_str() {
                "tr" => "0",
                "us" => "1",
                _ => "0",
            };
            let _ = Command::new("niri")
                .args(["msg", "action", "switch-layout", idx])
                .status();
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
        lbl.set_width_chars(26);
        lbl.set_xalign(0.0);
        widget.set_hexpand(true);
        row.append(&lbl);
        row.append(&widget);
        user_card.append(&row);
    }

    page_user.append(&user_card);

    let nav_user = GtkBox::new(Orientation::Horizontal, 10);
    nav_user.set_halign(gtk4::Align::End);
    let back_step1_btn = Button::with_label("← Geri");
    back_step1_btn.add_css_class("secondary-btn");
    let nb_ref2 = notebook.clone();
    back_step1_btn.connect_clicked(move |_| {
        nb_ref2.set_current_page(Some(0));
    });
    let to_step3_btn = Button::with_label("İleri: Kurulum Özeti & Onay →");
    to_step3_btn.add_css_class("primary-btn");
    let nb_ref3 = notebook.clone();
    to_step3_btn.connect_clicked(move |_| {
        nb_ref3.set_current_page(Some(2));
    });
    nav_user.append(&back_step1_btn);
    nav_user.append(&to_step3_btn);
    page_user.append(&nav_user);

    notebook.append_page(&page_user, Some(&Label::new(Some("2. Kimlik & Bölge"))));

    // ── STEP 3: ÖZET, DOĞRULAMA VE KURULUM (`solar-installer apply`) ────────
    let page_apply = GtkBox::new(Orientation::Vertical, 12);
    page_apply.set_margin_top(12);
    page_apply.set_margin_bottom(12);
    page_apply.set_margin_start(12);
    page_apply.set_margin_end(12);

    let apply_card = GtkBox::new(Orientation::Vertical, 10);
    apply_card.add_css_class("installer-card");

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
        .min_content_height(210)
        .vexpand(true)
        .build();
    let log_view = TextView::new();
    log_view.set_editable(false);
    log_view.set_monospace(true);
    log_view.add_css_class("log-view");
    log_scroll.set_child(Some(&log_view));
    apply_card.append(&log_scroll);

    let action_row = GtkBox::new(Orientation::Horizontal, 10);
    action_row.set_halign(gtk4::Align::End);

    let plan_btn = Button::with_label("Kurulum Planını Doğrula (Dry-Run)");
    plan_btn.add_css_class("secondary-btn");

    let install_btn = Button::with_label("BlazeOS'u Şimdi Diske Kur");
    install_btn.add_css_class("danger-btn");

    let reboot_btn = Button::with_label("Şimdi Yeniden Başlat");
    reboot_btn.add_css_class("primary-btn");
    reboot_btn.set_visible(false);
    reboot_btn.connect_clicked(|_| {
        let _ = Command::new("sudo").args(["-n", "reboot"]).spawn();
        let _ = Command::new("systemctl").arg("reboot").spawn();
    });

    // Plan button logic
    {
        let disk_combo = disk_combo.clone();
        let log_view = log_view.clone();
        plan_btn.connect_clicked(move |_| {
            let target = disk_combo
                .active_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "/dev/vdb".to_string());
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

        install_btn.connect_clicked(move |btn| {
            if !chk_confirm_erase.is_active() {
                log_view
                    .buffer()
                    .set_text("[UYARI] Lütfen önce disk silme onay kutucuğunu işaretleyin.");
                return;
            }
            let pass1 = entry_pass.text().to_string();
            let pass2 = entry_pass2.text().to_string();
            if pass1.len() < 8 {
                log_view
                    .buffer()
                    .set_text("[HATA] Parola en az 8 karakter uzunluğunda olmalıdır.");
                return;
            }
            if pass1 != pass2 {
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
            glib::MainContext::default().spawn_local(async move {
                let mut accumulated = String::new();
                while let Ok((frac, msg, done)) = rx.recv().await {
                    accumulated.push_str(&msg);
                    log_ui.buffer().set_text(&accumulated);
                    progress_ui.set_fraction(frac);
                    if done {
                        progress_ui.set_text(Some("Kurulum Başarıyla Tamamlandı!"));
                        reboot_ui.set_visible(true);
                    } else if frac == 0.0 {
                        progress_ui.set_text(Some("Kurulum durduruldu / hata oluştu"));
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

    notebook.append_page(&page_apply, Some(&Label::new(Some("3. Kurulum & Onay"))));

    root.append(&notebook);
    window.set_child(Some(&root));
    window.present();
    main_loop.run();
}
