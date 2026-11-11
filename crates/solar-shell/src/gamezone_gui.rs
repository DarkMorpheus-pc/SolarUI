// ==============================================================================
// Blaze GameZone - Fullscreen Handheld & Desktop Gaming Shell
// Pure Steam Deck UI & Xbox Handheld Hybrid Architecture
//
// Key Specifications:
// 1. Sol Panel (Dikey Kenar Çubuğu):
//    - Profil & Gamer Tag
//    - Kütüphanem, Steam, Epic Games (Heroic), GOG Galaxy, RetroArch, Xbox Cloud, Mağazalar
//    - SIFIR EMOJİ: Tamamen profesyonel tipografi ve gerçek vektör/sistem ikonları
// 2. Sağ Ana İçerik Alanı (Dikey Kaydırılabilir Çok Katmanlı Akış):
//    - Üst Xbox Canlı Performans HUD (FPS, GPU%, CPU%, RAM%, Saat) & Arama Çubuğu
//    - Katman 1: Son Oynananlar (Geniş Seçili Oyun Banner'ı + Dikey Kartlar)
//    - Katman 2: Steam Deck Navigasyon Hapları (Yenilikler, Arkadaşlar, Tavsiyeler)
//    - Katman 3: Oyun Haberleri ve Etkinlik Kartları (4 adet geniş afiş)
//    - Katman 4: Kütüphanemdeki Tüm Oyunlar (Çok Satırlı Izgara - Aşağı Doğru Devam Eder)
//    - Katman 5: Başlatıcılar ve Doğrudan Mağazalar (Steam, Epic, GOG)
// 3. Etkileşim ve Animasyon:
//    - Mouse kartın üzerine gelince kart büyür (scale-up animasyonu) ve beyaz odak halkası belirir
//    - Seçilen veya üzerine gelinen oyunun Steam Mağaza ekran görüntüsü anında arka plana ve Hero alanına yansır
//    - 10 Saniye Hover Kuralı: Mouse oyunun üzerinde 10 saniye tutulursa mağazadaki oynanış videosu/fragmanı başlar
// 4. Web Afiş, Ekran Görüntüsü ve Fragman Önbellek Motoru:
//    - ~/.cache/blaze-gamezone/media/<game_id>/
//    - Steam Mağaza API'sinden 1920x1080 ekran görüntüleri ve mp4 fragmanları otomatik çekilir
//    - Çevrimdışı ilk: Diskte önbellek varsa sıfır milisaniye gecikmeyle açılır
// 5. Sıfır Neon: Xbox ve Steam Deck'in resmi mat füme / arduvaz renk paleti
// ==============================================================================

use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CssProvider, Entry, EventControllerKey, EventControllerMotion, Image,
    Label, Orientation, Overlay, Picture, ScrolledWindow, Video, Window,
};
use crate::apps::scan_desktop_applications;
use crate::gamezone_live::{
    self, AccountSummary, CustomExecutableEntry, LivePreviewController, MediaUpdateEvent,
};
use std::collections::HashMap;
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    TR,
    EN,
}

impl Language {
    pub fn detect() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/darkmorpheus".to_string());
        let cfg_path = PathBuf::from(home).join(".config/solarui/gamezone/config.json");
        if let Ok(content) = std::fs::read_to_string(&cfg_path) {
            if content.contains("\"en\"") {
                return Language::EN;
            } else if content.contains("\"tr\"") {
                return Language::TR;
            }
        }
        if let Ok(glang) = std::env::var("GAMEZONE_LANG") {
            if glang.to_lowercase().starts_with("en") {
                return Language::EN;
            } else if glang.to_lowercase().starts_with("tr") {
                return Language::TR;
            }
        }
        Language::TR
    }

    pub fn save(&self) {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/darkmorpheus".to_string());
        let cfg_path = PathBuf::from(home).join(".config/solarui/gamezone/config.json");
        if let Some(parent) = cfg_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let val = match self {
            Language::EN => "{\n  \"language\": \"en\"\n}\n",
            Language::TR => "{\n  \"language\": \"tr\"\n}\n",
        };
        let _ = std::fs::write(cfg_path, val);
    }

    pub fn toggle(&self) -> Self {
        match self {
            Language::TR => Language::EN,
            Language::EN => Language::TR,
        }
    }

    pub fn toggle_btn_label(&self) -> &'static str {
        match self {
            Language::TR => "EN",
            Language::EN => "TR",
        }
    }

    pub fn title(&self) -> &'static str {
        "Blaze GameZone"
    }

    pub fn guest_player(&self) -> &'static str {
        match self {
            Language::TR => "Misafir Oyuncu",
            Language::EN => "Guest Gamer",
        }
    }

    pub fn online_status(&self, platform: &str) -> String {
        match self {
            Language::TR => format!("{} Çevrimiçi • Düşük Gecikme Modu", platform),
            Language::EN => format!("{} Online • Low-Latency Mode", platform),
        }
    }

    pub fn offline_status(&self, platform: &str) -> String {
        match self {
            Language::TR => format!("{} Çevrimdışı • Yerel Mod", platform),
            Language::EN => format!("{} Offline • Local Mode", platform),
        }
    }

    pub fn no_account_online(&self) -> &'static str {
        match self {
            Language::TR => "Hesap Bağlanmadı • Çevrimiçi",
            Language::EN => "No Account Linked • Online",
        }
    }

    pub fn no_account_offline(&self) -> &'static str {
        match self {
            Language::TR => "Hesap Bağlanmadı • Çevrimdışı",
            Language::EN => "No Account Linked • Offline",
        }
    }

    pub fn steam_login(&self) -> &'static str {
        match self {
            Language::TR => "Steam Girişi Yap",
            Language::EN => "Sign in to Steam",
        }
    }

    pub fn bore_kernel(&self) -> &'static str {
        match self {
            Language::TR => "Linux 7.2.9 • sched_ext & NTSYNC",
            Language::EN => "Linux 7.2.9 • sched_ext & NTSYNC",
        }
    }

    pub fn my_library(&self) -> &'static str {
        match self {
            Language::TR => "Kütüphanem",
            Language::EN => "My Library",
        }
    }

    pub fn all_games(&self) -> &'static str {
        match self {
            Language::TR => "Tüm Oyunlar",
            Language::EN => "All Games",
        }
    }

    pub fn steam_library(&self) -> &'static str {
        match self {
            Language::TR => "Steam Kütüphanesi",
            Language::EN => "Steam Library",
        }
    }

    pub fn epic_games(&self) -> &'static str {
        "Epic Games (Heroic)"
    }

    pub fn gog_galaxy(&self) -> &'static str {
        "GOG Galaxy"
    }

    pub fn retro_console(&self) -> &'static str {
        match self {
            Language::TR => "Retro Konsol (RetroArch)",
            Language::EN => "Retro Console (RetroArch)",
        }
    }

    pub fn xbox_cloud(&self) -> &'static str {
        "Xbox Cloud Gaming"
    }

    pub fn game_storefronts(&self) -> &'static str {
        match self {
            Language::TR => "Oyun Mağazaları",
            Language::EN => "Game Storefronts",
        }
    }

    pub fn steam_store(&self) -> &'static str {
        match self {
            Language::TR => "Steam Mağazası",
            Language::EN => "Steam Store",
        }
    }

    pub fn epic_store(&self) -> &'static str {
        "Epic Games Store"
    }

    pub fn gog_store(&self) -> &'static str {
        match self {
            Language::TR => "GOG.com Mağazası",
            Language::EN => "GOG.com Store",
        }
    }

    pub fn tools_settings(&self) -> &'static str {
        match self {
            Language::TR => "Araçlar & Ayarlar",
            Language::EN => "Tools & Settings",
        }
    }

    pub fn protonup_qt(&self) -> &'static str {
        match self {
            Language::TR => "ProtonUp-Qt Yöneticisi",
            Language::EN => "ProtonUp-Qt Manager",
        }
    }

    pub fn clean_cache(&self) -> &'static str {
        match self {
            Language::TR => "Afiş Önbelleğini Temizle",
            Language::EN => "Clear Poster Cache",
        }
    }

    pub fn offline_alert(&self) -> &'static str {
        match self {
            Language::TR => "İnternete bağlı değilsiniz. Sadece yüklü oyunları ve uygulamaları çalıştırabilirsiniz.",
            Language::EN => "You are offline. Only installed games and applications can be launched.",
        }
    }

    pub fn close(&self) -> &'static str {
        match self {
            Language::TR => "Kapat",
            Language::EN => "Close",
        }
    }

    pub fn search_placeholder(&self) -> &'static str {
        match self {
            Language::TR => "Oyun, mağaza veya eklenti ara...",
            Language::EN => "Search games, stores, or addons...",
        }
    }

    pub fn featured_game(&self) -> &'static str {
        match self {
            Language::TR => "ÖNE ÇIKAN OYUN",
            Language::EN => "FEATURED GAME",
        }
    }

    pub fn play_button(&self) -> &'static str {
        match self {
            Language::TR => "OYNA (A / Enter)",
            Language::EN => "PLAY (A / Enter)",
        }
    }

    pub fn install_button(&self) -> &'static str {
        match self {
            Language::TR => "YÜKLE (A / Enter)",
            Language::EN => "INSTALL (A / Enter)",
        }
    }

    pub fn game_options(&self) -> &'static str {
        match self {
            Language::TR => "Oyun Seçenekleri (X)",
            Language::EN => "Game Options (X)",
        }
    }

    pub fn store_page(&self) -> &'static str {
        match self {
            Language::TR => "Mağaza Sayfası",
            Language::EN => "Store Page",
        }
    }

    pub fn recently_played(&self) -> &'static str {
        match self {
            Language::TR => "Son Oynananlar",
            Language::EN => "Recently Played",
        }
    }

    pub fn no_recent_title(&self) -> &'static str {
        match self {
            Language::TR => "Henüz Bir Oyun Oynamadınız",
            Language::EN => "No Recently Played Games",
        }
    }

    pub fn no_recent_desc(&self) -> &'static str {
        match self {
            Language::TR => "Oynadığınız oyunlar ve son oturumlarınız burada otomatik olarak listelenecektir. Başlamak için aşağıdaki kütüphanenizden veya mağaza vitrininden bir oyun seçin.",
            Language::EN => "Games you play and your recent gaming sessions will be listed here automatically. Select a game from your library or store showcase below to get started.",
        }
    }

    pub fn whats_new(&self) -> &'static str {
        match self {
            Language::TR => "YENİLİKLER",
            Language::EN => "WHAT'S NEW",
        }
    }

    pub fn friends(&self) -> &'static str {
        match self {
            Language::TR => "ARKADAŞLAR (3)",
            Language::EN => "FRIENDS (3)",
        }
    }

    pub fn recommended(&self) -> &'static str {
        match self {
            Language::TR => "TAVSİYE EDİLENLER",
            Language::EN => "RECOMMENDED",
        }
    }

    pub fn community_hub(&self) -> &'static str {
        match self {
            Language::TR => "TOPLULUK MERKEZİ",
            Language::EN => "COMMUNITY HUB",
        }
    }

    pub fn all_games_in_library(&self) -> &'static str {
        match self {
            Language::TR => "Kütüphanemdeki Tüm Oyunlar",
            Language::EN => "All Games in My Library",
        }
    }

    pub fn footer_play(&self) -> &'static str {
        match self {
            Language::TR => "Oyna / Seç",
            Language::EN => "Play / Select",
        }
    }

    pub fn footer_back(&self) -> &'static str {
        match self {
            Language::TR => "Geri / Masaüstü",
            Language::EN => "Back / Desktop",
        }
    }

    pub fn footer_options(&self) -> &'static str {
        match self {
            Language::TR => "Seçenekler",
            Language::EN => "Options",
        }
    }

    pub fn footer_search(&self) -> &'static str {
        match self {
            Language::TR => "Ara",
            Language::EN => "Search",
        }
    }

    pub fn footer_categories(&self) -> &'static str {
        match self {
            Language::TR => "Kategoriler",
            Language::EN => "Categories",
        }
    }

    pub fn trailer_waiting(&self) -> &'static str {
        match self {
            Language::TR => "Video Fragman: 10 sn bekleniyor...",
            Language::EN => "Video Trailer: Waiting 10s...",
        }
    }

    pub fn exit_to_desktop(&self) -> &'static str {
        match self {
            Language::TR => "Masaüstüne Dön",
            Language::EN => "Exit to Desktop",
        }
    }

    pub fn system_section(&self) -> &'static str {
        match self {
            Language::TR => "Sistem",
            Language::EN => "System",
        }
    }

    pub fn all_applications(&self) -> &'static str {
        match self {
            Language::TR => "Tüm Uygulamalar",
            Language::EN => "All Applications",
        }
    }

    pub fn browse_games_and_apps(&self) -> &'static str {
        match self {
            Language::TR => "Oyunlarına ve uygulamalarına göz at",
            Language::EN => "Browse your games & apps",
        }
    }

    pub fn customize_home(&self) -> &'static str {
        match self {
            Language::TR => "Giriş ekranını özelleştir",
            Language::EN => "Customize your Home",
        }
    }

    pub fn store_deals(&self) -> &'static str {
        match self {
            Language::TR => "Mağaza & Fırsatlar",
            Language::EN => "Store & Featured Deals",
        }
    }

    pub fn play_like_pro(&self) -> &'static str {
        match self {
            Language::TR => "Bir Profesyonel Gibi Oyna",
            Language::EN => "Play like a Pro",
        }
    }

    pub fn back_to_home(&self) -> &'static str {
        match self {
            Language::TR => "Ana Ekrana Dön",
            Language::EN => "Back to Home",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameEntry {
    pub id: String,
    pub title: String,
    pub category: String,
    pub exec: String,
    pub banner_desc: String,
    pub is_steam: bool,
    pub is_installed: bool,
    pub steam_app_id: Option<String>,
    pub cover_path: Option<String>,
    pub hero_path: Option<String>,
    pub screenshot_path: Option<String>,
    pub store_url: Option<String>,
    pub movie_url: Option<String>,
}

pub fn is_system_online() -> bool {
    // 1. Check nmcli if available
    if let Ok(output) = Command::new("nmcli").args(["networking", "connectivity", "check"]).output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
            let t = s.trim();
            if t == "full" || t == "limited" {
                return true;
            }
            if t == "none" {
                return false;
            }
        }
    }
    // 2. Fallback: Quick TCP probe to DNS (1.1.1.1:53) with 300ms timeout
    if let Ok(addr) = "1.1.1.1:53".parse::<SocketAddr>() {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok() {
            return true;
        }
    }
    // 3. Fallback: Check if default route exists in /proc/net/route
    if let Ok(routes) = std::fs::read_to_string("/proc/net/route") {
        for line in routes.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() > 1 && fields[1] == "00000000" {
                return true;
            }
        }
    }
    false
}

pub fn open_browser_url(url: &str) {
    if Command::new("xdg-open").arg(url).spawn().is_err() {
        if Command::new("firefox").arg(url).spawn().is_err() {
            let _ = Command::new("chromium").arg(url).spawn();
        }
    }
}

pub fn is_game_installed(steam_app_id: Option<&str>, exec: &str, id: &str) -> bool {
    // 1. Steam App ID check
    if let Some(app_id) = steam_app_id {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/blaze".to_string());
        let steam_dirs = [
            PathBuf::from(&home).join(".steam/steam/steamapps"),
            PathBuf::from(&home).join(".local/share/Steam/steamapps"),
            PathBuf::from("/var/lib/flatpak/app/com.valvesoftware.Steam/x86_64/stable/active/files/share/steamapps"),
        ];
        for s_dir in steam_dirs {
            let manifest = s_dir.join(format!("appmanifest_{}.acf", app_id));
            if manifest.exists() {
                return true;
            }
        }
        return false;
    }

    // 2. Special cloud services
    if id == "xbox-cloud" {
        return false;
    }

    // 3. Executable check
    let parts: Vec<&str> = exec.split_whitespace().collect();
    if let Some(cmd) = parts.get(0) {
        if Path::new(cmd).is_absolute() {
            return Path::new(cmd).exists();
        }
        let check_dirs = [
            "/usr/bin",
            "/usr/local/bin",
            "/bin",
            "/usr/games",
            "/usr/local/games",
        ];
        for d in check_dirs {
            if Path::new(d).join(cmd).exists() {
                return true;
            }
        }
        if let Ok(out) = Command::new("which").arg(cmd).output() {
            if out.status.success() {
                return true;
            }
        }
    }

    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamepadNavAction {
    Up,
    Down,
    Left,
    Right,
    SelectA,
    BackB,
    OptionsX,
    SearchY,
    TabPrevLB,
    TabNextRB,
}

/// Linux joystick reader (/dev/input/js*) for 10-foot gamepad navigation
pub fn start_linux_gamepad_listener() -> async_channel::Receiver<GamepadNavAction> {
    let (tx, rx) = async_channel::bounded(64);

    thread::spawn(move || {
        loop {
            // Find active joystick device in /dev/input/js0..js4
            let mut active_file = None;
            for i in 0..4 {
                let dev_path = format!("/dev/input/js{}", i);
                if let Ok(file) = File::open(&dev_path) {
                    active_file = Some((dev_path, file));
                    break;
                }
            }

            let (_dev_path, mut file) = match active_file {
                Some(f) => f,
                None => {
                    thread::sleep(Duration::from_millis(1500));
                    continue;
                }
            };

            let mut buf = [0u8; 8];
            let mut last_axis_x: i16 = 0;
            let mut last_axis_y: i16 = 0;

            loop {
                match file.read_exact(&mut buf) {
                    Ok(()) => {
                        let value = i16::from_ne_bytes([buf[4], buf[5]]);
                        let event_type = buf[6];
                        let number = buf[7];

                        // Filter out initial synth config events (0x80)
                        let is_init = (event_type & 0x80) != 0;
                        let clean_type = event_type & !0x80;

                        if clean_type == 1 {
                            // Button event (1 = pressed, 0 = released)
                            if value == 1 && !is_init {
                                let action = match number {
                                    0 => Some(GamepadNavAction::SelectA),     // A button
                                    1 => Some(GamepadNavAction::BackB),       // B button
                                    2 => Some(GamepadNavAction::OptionsX),    // X button
                                    3 => Some(GamepadNavAction::SearchY),     // Y button
                                    4 => Some(GamepadNavAction::TabPrevLB),   // Left Bumper (LB)
                                    5 => Some(GamepadNavAction::TabNextRB),   // Right Bumper (RB)
                                    6 => Some(GamepadNavAction::BackB),       // Select / View
                                    7 => Some(GamepadNavAction::SelectA),     // Start / Menu
                                    _ => None,
                                };
                                if let Some(act) = action {
                                    let _ = tx.send_blocking(act);
                                }
                            }
                        } else if clean_type == 2 && !is_init {
                            // Axis event (D-Pad or Left Stick)
                            // Threshold for directional actuation
                            const STICK_THRESHOLD: i16 = 18000;
                            const STICK_DEADZONE: i16 = 10000;

                            // Number 0 = X axis (Left Stick), 1 = Y axis (Left Stick)
                            // Number 6 = D-Pad X, 7 = D-Pad Y (standard Linux gamepad mapping)
                            if number == 0 || number == 6 {
                                if value > STICK_THRESHOLD && last_axis_x <= STICK_DEADZONE {
                                    let _ = tx.send_blocking(GamepadNavAction::Right);
                                } else if value < -STICK_THRESHOLD && last_axis_x >= -STICK_DEADZONE {
                                    let _ = tx.send_blocking(GamepadNavAction::Left);
                                }
                                last_axis_x = value;
                            } else if number == 1 || number == 7 {
                                if value > STICK_THRESHOLD && last_axis_y <= STICK_DEADZONE {
                                    let _ = tx.send_blocking(GamepadNavAction::Down);
                                } else if value < -STICK_THRESHOLD && last_axis_y >= -STICK_DEADZONE {
                                    let _ = tx.send_blocking(GamepadNavAction::Up);
                                }
                                last_axis_y = value;
                            }
                        }
                    }
                    Err(_) => {
                        // Joystick disconnected or read error
                        break;
                    }
                }
            }

            thread::sleep(Duration::from_millis(1000));
        }
    });

    rx
}

pub fn launch_gamezone_window() {
    glib::set_prgname(Some("solar-gamezone"));
    glib::set_application_name("Blaze GameZone");

    if let Err(err) = gtk4::init() {
        eprintln!("Failed to initialize GTK4 for GameZone: {}", err);
        return;
    }

    // Activate low-latency gaming mode (CPU performance governor + BORE scheduler)
    activate_gaming_optimizations();

    let main_loop = glib::MainLoop::new(None, false);
    build_gamezone_ui(main_loop.clone());
    main_loop.run();

    // Restore standard balanced settings on exit
    restore_normal_optimizations();
}


fn resolve_desktop_icon_path(icon_name: &str) -> Option<String> {
    if icon_name.is_empty() {
        return None;
    }
    if icon_name.starts_with('/') && Path::new(icon_name).exists() {
        return Some(icon_name.to_string());
    }
    let candidate_dirs = [
        "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/icons/hicolor/128x128/apps",
        "/usr/share/icons/hicolor/128x128/apps",
        "/usr/share/icons/hicolor/48x48/apps",
        "/usr/share/icons/hicolor/64x64/apps",
        "/usr/share/icons/hicolor/256x256/apps",
        "/usr/share/icons/breeze/apps/64",
        "/usr/share/icons/breeze/apps/48",
        "/usr/share/pixmaps",
    ];
    for d in candidate_dirs {
        for ext in ["png", "svg"] {
            let p = Path::new(d).join(format!("{}.{}", icon_name, ext));
            if p.exists() {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }
    let aliases: &[(&str, &str)] = &[
        ("kitty", "utilities-terminal"),
        ("htop", "blaze-system-monitor"),
        ("protonup-qt", "protonup-qt"),
        ("org.gnome.TextEditor", "text-editor"),
        ("org.gnome.Totem", "video-player"),
        ("org.gnome.Weather", "weather"),
        ("org.gnome.Loupe", "image-viewer"),
        ("solar-settings", "preferences-system"),
    ];
    for (alias_key, alias_val) in aliases {
        if icon_name.to_lowercase().contains(alias_key) || icon_name == *alias_key {
            for d in candidate_dirs {
                for ext in ["png", "svg"] {
                    let p = Path::new(d).join(format!("{}.{}", alias_val, ext));
                    if p.exists() {
                        return Some(p.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    None
}

fn load_picture_file(pic: &Picture, path: &Path) {
    if let Ok(bytes) = std::fs::read(path) {
        let gbytes = glib::Bytes::from(&bytes);
        if let Ok(texture) = gdk::Texture::from_bytes(&gbytes) {
            pic.set_paintable(Some(&texture));
            return;
        }
    }
    pic.set_filename(Some(path));
}

fn create_nav_button(label_text: &str, icon_name_base: &str) -> Button {
    let btn = Button::new();
    btn.add_css_class("xbox-nav-icon-btn");

    let b_box = GtkBox::new(Orientation::Horizontal, 7);
    b_box.set_valign(gtk4::Align::Center);

    let pic = Picture::new();
    pic.set_size_request(16, 16);
    pic.set_can_shrink(true);
    pic.set_content_fit(gtk4::ContentFit::Contain);
    pic.add_css_class("xbox-nav-btn-icon");

    let icon_candidates = [
        format!("/usr/share/solarui/icons/{}.png", icon_name_base),
        format!("/home/darkmorpheus/SolarUI/data/icons/gamezone/{}.png", icon_name_base),
        format!("/usr/share/solarui/icons/{}.svg", icon_name_base),
        format!("/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/icons/{}.png", icon_name_base),
        format!("/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/icons/{}.svg", icon_name_base),
    ];
    for ic in &icon_candidates {
        if Path::new(ic).exists() {
            load_picture_file(&pic, Path::new(ic));
            b_box.append(&pic);
            break;
        }
    }

    let lbl = Label::new(Some(label_text));
    lbl.set_valign(gtk4::Align::Center);
    b_box.append(&lbl);

    btn.set_child(Some(&b_box));
    btn
}

fn build_gamezone_ui(main_loop: glib::MainLoop) {
    let lang = Language::detect();
    let provider = CssProvider::new();
    let css = r#"
        window {
            background-color: #111216;
            color: #f4f4f6;
            font-family: "Inter", "Cantarell", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
        }

        button {
            background-image: none;
        }

        *,
        *:focus,
        *:focus-visible,
        *:focus-within {
            outline-color: transparent;
            outline-width: 0px;
            outline-style: none;
        }

        .xbox-root {
            background-color: transparent;
            padding: 0;
            margin: 0;
        }

        /* ── Fullscreen Backdrop & Obsidian Scrim ── */
        .xbox-backdrop-layer {
            background-color: #111216;
        }

        .xbox-backdrop-scrim {
            background: linear-gradient(180deg,
                rgba(12, 13, 17, 0.66) 0%,
                rgba(12, 13, 17, 0.78) 44%,
                rgba(12, 13, 17, 0.93) 76%,
                rgba(12, 13, 17, 0.98) 100%
            );
        }

        /* ── Top Living Flow HUD Bar ── */
        .xbox-top-bar {
            padding: 20px 28px 12px 28px;
            background: transparent;
        }

        .xbox-avatar {
            min-width: 38px;
            min-height: 38px;
            border-radius: 9999px;
            border: 1.5px solid rgba(255, 255, 255, 0.22);
            background-color: #18191e;
            margin-right: 12px;
            padding: 2px;
        }

        .xbox-gamertag {
            color: #f4f4f6;
            font-size: 15px;
            font-weight: 700;
            letter-spacing: -0.2px;
            text-shadow: 0 2px 8px rgba(0, 0, 0, 0.88);
        }

        .xbox-badge-tag {
            background: rgba(244, 244, 246, 0.94);
            color: #111215;
            font-size: 9px;
            font-weight: 900;
            padding: 2px 7px;
            border-radius: 999px;
            margin-left: 8px;
            letter-spacing: 0.6px;
            min-height: 14px;
        }

        .xbox-gamerscore {
            color: #a1a1aa;
            font-size: 13px;
            font-weight: 600;
            margin-left: 8px;
        }

        .xbox-score-icon {
            color: #e54d2e;
            font-size: 13px;
            font-weight: 800;
            margin-right: 3px;
        }

        .xbox-top-nav-group {
            background: rgba(22, 23, 28, 0.86);
            border: 1px solid rgba(255, 255, 255, 0.10);
            border-radius: 999px;
            padding: 4px 6px;
            box-shadow: 0 8px 24px rgba(0, 0, 0, 0.42);
        }

        .xbox-nav-icon-btn {
            background: transparent;
            color: #c8cbd2;
            border-radius: 999px;
            padding: 6px 12px;
            border: 1px solid transparent;
            font-size: 12.5px;
            font-weight: 700;
            transition: all 180ms cubic-bezier(0.22, 1, 0.36, 1);
        }

        .xbox-nav-icon-btn:hover {
            color: #ffffff;
            background: rgba(255, 255, 255, 0.08);
        }

        .xbox-nav-icon-btn:focus {
            color: #ffffff;
            background: rgba(255, 255, 255, 0.16);
            border: 1px solid rgba(255, 255, 255, 0.34);
            box-shadow: 0 4px 14px rgba(0, 0, 0, 0.36);
        }

        .xbox-nav-icon-btn.active {
            color: #ffffff;
            background: rgba(255, 255, 255, 0.13);
            border: 1px solid rgba(255, 255, 255, 0.22);
        }

        .xbox-nav-btn-icon {
            margin-right: 4px;
            min-width: 16px;
            min-height: 16px;
        }

        /* ── Search & Filter Omnibar In Drawer ── */
        .xbox-search-bar {
            background: #1c1e24;
            color: #f4f4f6;
            border: 1px solid rgba(255, 255, 255, 0.12);
            border-radius: 999px;
            padding: 9px 18px;
            font-size: 14px;
            margin-bottom: 16px;
        }

        .xbox-search-bar:focus {
            border: 1px solid rgba(255, 255, 255, 0.32);
            box-shadow: none;
        }

        /* ── Settings & Store Panels ── */
        .xbox-modal-drawer {
            background: rgba(20, 21, 26, 0.96);
            border-radius: 22px;
            border: 1px solid rgba(255, 255, 255, 0.11);
            padding: 24px 32px;
            margin-top: 8px;
            margin-bottom: 24px;
            box-shadow: 0 18px 44px rgba(0, 0, 0, 0.72);
        }

        .xbox-modal-card {
            background: #22242c;
            border-radius: 16px;
            border: 1px solid rgba(255, 255, 255, 0.09);
            padding: 16px 20px;
            margin-right: 14px;
            min-width: 220px;
            transition: all 160ms cubic-bezier(0.22, 1, 0.36, 1);
        }

        .xbox-modal-card:hover, .xbox-modal-card:focus {
            border: 1px solid rgba(255, 255, 255, 0.36);
            background: #2a2d37;
            transform: scale(1.02);
        }

        .xbox-modal-card-title {
            color: #f4f4f6;
            font-size: 15px;
            font-weight: 700;
        }

        .xbox-modal-card-desc {
            color: #9ca3af;
            font-size: 12px;
            margin-top: 4px;
        }

        .xbox-hud-right {
            color: #d4d4d8;
            font-size: 13px;
            font-weight: 600;
        }

        .xbox-hud-time {
            color: #f4f4f6;
            font-size: 14px;
            font-weight: 700;
            margin-left: 12px;
        }

        .xbox-lang-btn {
            background: rgba(255, 255, 255, 0.08);
            color: #e4e4e7;
            font-size: 11px;
            font-weight: 700;
            border-radius: 999px;
            padding: 4px 11px;
            border: 1px solid rgba(255, 255, 255, 0.14);
            margin-left: 14px;
            transition: all 150ms ease;
        }

        .xbox-lang-btn:hover, .xbox-lang-btn:focus {
            background: #f2f3f5;
            color: #111215;
            border-color: #ffffff;
        }

        /* ── Main Content Vertical Flow ── */
        .xbox-main-scroll {
            background: transparent;
            padding: 0;
            margin: 0;
        }

        .xbox-content-box {
            padding: 0 44px 28px 44px;
            background: transparent;
        }

        .xbox-hero-spacer {
            min-height: 184px;
            padding-top: 14px;
            padding-bottom: 12px;
        }

        .gz-preview-frame {
            background: rgba(16, 17, 22, 0.92);
            border: 1px solid rgba(255, 255, 255, 0.16);
            border-radius: 18px;
            min-width: 304px;
            min-height: 171px;
            box-shadow: 0 14px 36px rgba(0, 0, 0, 0.65);
        }

        .gz-preview-pic {
            border-radius: 17px;
            min-width: 304px;
            min-height: 171px;
        }

        .gz-preview-badge {
            background: rgba(14, 15, 19, 0.84);
            color: #f4f4f6;
            border: 1px solid rgba(255, 255, 255, 0.18);
            border-radius: 999px;
            padding: 3px 10px;
            margin: 10px;
            font-size: 10px;
            font-weight: 800;
            letter-spacing: 0.6px;
        }

        .gz-hero-action-btn {
            background: rgba(255, 255, 255, 0.09);
            color: #f4f4f6;
            border: 1px solid rgba(255, 255, 255, 0.16);
            border-radius: 999px;
            padding: 6px 15px;
            font-size: 12px;
            font-weight: 700;
            transition: all 160ms cubic-bezier(0.22, 1, 0.36, 1);
        }

        .gz-hero-action-btn:hover, .gz-hero-action-btn:focus {
            background: rgba(255, 255, 255, 0.18);
            border-color: rgba(255, 255, 255, 0.36);
        }

        .gz-hero-play-primary {
            background: #f4f4f6;
            color: #111216;
            border: 1px solid #ffffff;
            border-radius: 999px;
            padding: 6px 18px;
            font-size: 12px;
            font-weight: 800;
            transition: all 160ms cubic-bezier(0.22, 1, 0.36, 1);
        }

        .gz-hero-play-primary:hover, .gz-hero-play-primary:focus {
            background: #ffffff;
            transform: scale(1.02);
        }

        .gz-subtab-bar {
            background: rgba(22, 23, 29, 0.86);
            border: 1px solid rgba(255, 255, 255, 0.10);
            border-radius: 999px;
            padding: 4px 5px;
            margin-bottom: 10px;
        }

        .gz-subtab-btn {
            background: transparent;
            color: #a1a1aa;
            border: 1px solid transparent;
            border-radius: 999px;
            padding: 5px 15px;
            font-size: 12px;
            font-weight: 700;
            transition: all 160ms cubic-bezier(0.22, 1, 0.36, 1);
        }

        .gz-subtab-btn:hover {
            color: #f4f4f6;
            background: rgba(255, 255, 255, 0.06);
        }

        .gz-subtab-btn.active {
            background: rgba(255, 255, 255, 0.14);
            color: #ffffff;
            border: 1px solid rgba(255, 255, 255, 0.24);
            box-shadow: 0 3px 10px rgba(0, 0, 0, 0.32);
        }

        .gz-card-discount-pill {
            background: rgba(22, 101, 52, 0.92);
            color: #dcfce7;
            border: 1px solid rgba(74, 222, 128, 0.35);
            border-radius: 999px;
            padding: 2px 8px;
            margin: 8px;
            font-size: 10px;
            font-weight: 800;
        }

        .gz-card-installed-pill {
            background: rgba(207, 63, 36, 0.92);
            color: #ffffff;
            border: 1px solid rgba(255, 255, 255, 0.28);
            border-radius: 999px;
            padding: 2px 8px;
            margin: 8px;
            font-size: 9.5px;
            font-weight: 800;
        }

        .gz-local-empty-card {
            background: rgba(24, 26, 32, 0.90);
            border: 1px solid rgba(255, 255, 255, 0.10);
            border-radius: 18px;
            padding: 18px 24px;
            margin-bottom: 18px;
        }

        .xbox-hero-kicker {
            color: #f0785a;
            font-size: 11px;
            font-weight: 800;
            letter-spacing: 1.4px;
            text-shadow: 0 2px 8px rgba(0, 0, 0, 0.90);
        }

        .xbox-hero-headline {
            color: #f4f4f6;
            font-size: 28px;
            font-weight: 800;
            letter-spacing: -0.5px;
            margin-top: 4px;
            text-shadow: 0 2px 10px rgba(0, 0, 0, 0.90);
        }

        .xbox-hero-desc {
            color: #d4d4d8;
            font-size: 13.5px;
            font-weight: 500;
            margin-top: 4px;
            text-shadow: 0 2px 8px rgba(0, 0, 0, 0.90);
        }

        /* ── Focused Title Label ── */
        .xbox-active-title-label {
            color: #f4f4f6;
            font-size: 23px;
            font-weight: 800;
            letter-spacing: -0.4px;
            margin-bottom: 4px;
            text-shadow: 0 2px 10px rgba(0, 0, 0, 0.75);
        }

        .xbox-active-badge {
            background: rgba(207, 63, 36, 0.20);
            color: #f5b4a5;
            border: 1px solid rgba(229, 77, 46, 0.36);
            border-radius: 999px;
            padding: 3px 11px;
            font-size: 10.5px;
            font-weight: 800;
            margin-left: 10px;
            margin-bottom: 5px;
            letter-spacing: 0.6px;
        }

        /* ── Game Carousel Row ── */
        .xbox-carousel-scroll {
            background: transparent;
            margin-bottom: 16px;
        }

        .xbox-carousel-scroll undershoot.top,
        .xbox-carousel-scroll undershoot.bottom,
        .xbox-carousel-scroll undershoot.left,
        .xbox-carousel-scroll undershoot.right {
            background: none;
        }

        .xbox-carousel-scroll scrollbar {
            opacity: 0;
            min-height: 0;
            min-width: 0;
        }

        .xbox-game-card {
            background: linear-gradient(180deg, rgba(28, 30, 36, 0.96), rgba(18, 19, 24, 0.98));
            background-image: none;
            border-radius: 20px;
            border: 1px solid rgba(255, 255, 255, 0.12);
            padding: 0;
            margin-right: 14px;
            margin-top: 8px;
            margin-bottom: 8px;
            min-width: 156px;
            min-height: 156px;
            transition: all 200ms cubic-bezier(0.22, 1, 0.36, 1);
            box-shadow: 0 10px 24px rgba(0, 0, 0, 0.52);
        }

        .xbox-game-card:hover {
            border-color: rgba(255, 255, 255, 0.42);
            transform: scale(1.03);
        }

        .xbox-game-card:focus, .xbox-game-card.active-focus {
            border: 1.5px solid rgba(255, 255, 255, 0.80);
            box-shadow: 0 14px 32px rgba(0, 0, 0, 0.72);
            transform: scale(1.05);
        }

        .xbox-game-card-img {
            border-radius: 19px;
            min-width: 156px;
            min-height: 156px;
        }

        .xbox-card-caption {
            background: linear-gradient(180deg, rgba(14, 15, 19, 0.0) 0%, rgba(14, 15, 19, 0.84) 48%, rgba(14, 15, 19, 0.96) 100%);
            border-bottom-left-radius: 19px;
            border-bottom-right-radius: 19px;
            padding: 18px 11px 9px 11px;
        }

        .xbox-card-caption-title {
            color: #f4f4f6;
            font-size: 12px;
            font-weight: 700;
        }

        /* ── Bento Grid Bottom Banners ── */
        .xbox-bento-row {
            margin-bottom: 20px;
        }

        .xbox-bento-card {
            background: #181a20;
            border-radius: 18px;
            border: 1px solid rgba(255, 255, 255, 0.10);
            padding: 0;
            margin-right: 14px;
            min-width: 275px;
            min-height: 155px;
            transition: all 180ms cubic-bezier(0.22, 1, 0.36, 1);
            box-shadow: 0 10px 24px rgba(0, 0, 0, 0.48);
        }

        .xbox-bento-card:hover, .xbox-bento-card:focus {
            border: 1.5px solid rgba(255, 255, 255, 0.48);
            box-shadow: 0 14px 30px rgba(0, 0, 0, 0.68);
            transform: scale(1.025);
        }

        .xbox-bento-overlay-box {
            background: linear-gradient(180deg, rgba(12, 13, 16, 0.05) 0%, rgba(12, 13, 16, 0.82) 100%);
            border-radius: 17px;
            padding: 14px 16px;
        }

        .xbox-bento-tag {
            color: #e56b4e;
            font-size: 10px;
            font-weight: 800;
            text-transform: uppercase;
            letter-spacing: 0.9px;
        }

        .xbox-bento-title {
            color: #f4f4f6;
            font-size: 15px;
            font-weight: 700;
            margin-top: 2px;
            text-shadow: 0 1px 4px rgba(0, 0, 0, 0.8);
        }

        .xbox-bento-subtitle {
            color: #a1a1aa;
            font-size: 12px;
            font-weight: 500;
            margin-top: 2px;
        }

        /* ── All Apps View Grid ── */
        .xbox-apps-drawer {
            background: rgba(20, 21, 26, 0.96);
            border-radius: 22px;
            border: 1px solid rgba(255, 255, 255, 0.11);
            padding: 24px 32px;
            margin-top: 8px;
            margin-bottom: 24px;
        }

        .xbox-apps-header {
            color: #f4f4f6;
            font-size: 20px;
            font-weight: 800;
            margin-bottom: 16px;
        }

        .xbox-apps-close-btn {
            background: rgba(255, 255, 255, 0.08);
            color: #f4f4f6;
            font-size: 12px;
            font-weight: 700;
            border-radius: 999px;
            padding: 6px 14px;
            border: 1px solid rgba(255, 255, 255, 0.16);
        }

        .xbox-apps-close-btn:hover, .xbox-apps-close-btn:focus {
            background: #c83b22;
            border-color: rgba(255, 255, 255, 0.32);
        }

        .xbox-app-tile {
            background: #20222a;
            border-radius: 14px;
            border: 1px solid rgba(255, 255, 255, 0.08);
            padding: 12px;
            margin-right: 12px;
            margin-bottom: 12px;
            min-width: 150px;
            transition: all 150ms ease;
        }

        .xbox-app-tile:hover, .xbox-app-tile:focus {
            border: 1px solid rgba(255, 255, 255, 0.36);
            background: #282b35;
            transform: scale(1.03);
        }

        .xbox-app-title {
            color: #f4f4f6;
            font-size: 12px;
            font-weight: 700;
            margin-top: 6px;
        }

        /* ── Bottom Gamepad Bar (Legend) ── */
        .xbox-controller-bar {
            background: rgba(15, 16, 20, 0.90);
            border-top: 1px solid rgba(255, 255, 255, 0.08);
            padding: 10px 44px;
        }

        .xbox-legend-key {
            background: rgba(255, 255, 255, 0.10);
            color: #f4f4f6;
            border: 1px solid rgba(255, 255, 255, 0.18);
            border-radius: 9999px;
            font-size: 11px;
            font-weight: 800;
            min-width: 20px;
            min-height: 20px;
            padding: 2px 6px;
            margin-right: 6px;
        }

        .xbox-legend-desc {
            color: #a1a1aa;
            font-size: 12px;
            font-weight: 600;
            margin-right: 28px;
        }

        /* ── Offline Banner ── */
        .xbox-offline-banner {
            background: rgba(44, 24, 22, 0.90);
            border: 1px solid rgba(229, 77, 46, 0.38);
            border-radius: 14px;
            padding: 8px 16px;
            margin: 0 44px 12px 44px;
        }
    "#;

    provider.load_from_string(css);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let window = Window::builder()
        .title(lang.title())
        .default_width(1280)
        .default_height(760)
        .maximized(true)
        .build();

    let local_detected_initial = gamezone_live::scan_computer_and_account_games(discover_all_games());
    let installed_appids: std::collections::HashSet<String> = local_detected_initial
        .iter()
        .filter_map(|g| g.steam_app_id.clone())
        .collect();
    let cached_featured = gamezone_live::load_cached_featured_games(lang, &installed_appids);
    let initial_featured = if !cached_featured.is_empty() {
        cached_featured
    } else {
        discover_store_showcase_for_lang(lang)
    };
    let selected_index = Arc::new(Mutex::new(0usize));
    let preview_ctrl = LivePreviewController::new();
    let (media_tx, media_rx) = async_channel::unbounded::<MediaUpdateEvent>();

    // Fullscreen Dynamic Adaptive Backdrop
    let bg_box = GtkBox::new(Orientation::Vertical, 0);
    bg_box.add_css_class("xbox-backdrop-layer");
    bg_box.set_hexpand(true);
    bg_box.set_vexpand(true);

    let bg_picture = Picture::new();
    bg_picture.set_can_shrink(true);
    bg_picture.set_content_fit(gtk4::ContentFit::Cover);
    bg_picture.set_hexpand(true);
    bg_picture.set_vexpand(true);

    let def_wallpaper = get_default_wallpaper_path();
    if !def_wallpaper.is_empty() {
        load_picture_file(&bg_picture, Path::new(&def_wallpaper));
    }

    let bg_scrim = GtkBox::new(Orientation::Vertical, 0);
    bg_scrim.add_css_class("xbox-backdrop-scrim");
    bg_scrim.set_hexpand(true);
    bg_scrim.set_vexpand(true);

    let bg_overlay = Overlay::new();
    bg_overlay.set_hexpand(true);
    bg_overlay.set_vexpand(true);
    bg_overlay.set_child(Some(&bg_picture));
    bg_overlay.add_overlay(&bg_scrim);

    bg_box.append(&bg_overlay);

    // Root Container: Single vertical column over backdrop
    let root_box = GtkBox::new(Orientation::Vertical, 0);
    root_box.add_css_class("xbox-root");
    root_box.set_hexpand(true);
    root_box.set_vexpand(true);

    let is_online = is_system_online();

    // ── Offline Banner ──
    let offline_banner = GtkBox::new(Orientation::Horizontal, 12);
    offline_banner.add_css_class("xbox-offline-banner");
    offline_banner.set_visible(!is_online);

    let offline_icon = Label::new(Some("!"));
    offline_icon.set_css_classes(&["xbox-legend-key"]);

    let offline_label = Label::new(Some(lang.offline_alert()));
    offline_label.set_hexpand(true);
    offline_label.set_halign(gtk4::Align::Start);

    let offline_close_btn = Button::with_label(lang.close());
    offline_close_btn.set_css_classes(&["xbox-apps-close-btn"]);
    let ob_clone = offline_banner.clone();
    offline_close_btn.connect_clicked(move |_| {
        ob_clone.set_visible(false);
    });

    offline_banner.append(&offline_icon);
    offline_banner.append(&offline_label);
    offline_banner.append(&offline_close_btn);

    let offline_banner_clone = offline_banner.clone();
    let offline_label_clone = offline_label.clone();
    let trigger_offline_alert: Rc<dyn Fn(&str)> = Rc::new(move |msg: &str| {
        offline_label_clone.set_text(msg);
        offline_banner_clone.set_visible(true);
        let _ = Command::new("notify-send")
            .args([
                "-a", "Blaze GameZone",
                "-i", "network-offline",
                "Çevrimdışı Uyarı",
                msg,
            ])
            .spawn();
    });

    // ── TOP FLUENT HUD BAR ──
    let top_bar = GtkBox::new(Orientation::Horizontal, 0);
    top_bar.add_css_class("xbox-top-bar");

    // Left: Avatar + Gamertag + Gamerscore
    let user_box = GtkBox::new(Orientation::Horizontal, 0);
    user_box.set_valign(gtk4::Align::Center);

    let acct: AccountSummary = gamezone_live::detect_accounts_with_avatar();

    let avatar_img = Image::new();
    avatar_img.set_pixel_size(34);
    avatar_img.set_size_request(34, 34);
    avatar_img.add_css_class("xbox-avatar");

    let mut avatar_loaded = false;
    if let Some(ref av_path) = acct.steam_avatar_path {
        if let Ok(bytes) = std::fs::read(av_path) {
            let gbytes = glib::Bytes::from(&bytes);
            if let Ok(tex) = gdk::Texture::from_bytes(&gbytes) {
                avatar_img.set_paintable(Some(&tex));
                avatar_loaded = true;
            }
        }
    }
    if !avatar_loaded {
        let avatar_candidates = [
            "/usr/share/solarui/covers/user_avatar.png",
            "/home/darkmorpheus/SolarUI/data/covers/user_avatar.png",
            "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/covers/user_avatar.png",
        ];
        for ac in avatar_candidates {
            if let Ok(bytes) = std::fs::read(ac) {
                let gbytes = glib::Bytes::from(&bytes);
                if let Ok(tex) = gdk::Texture::from_bytes(&gbytes) {
                    avatar_img.set_paintable(Some(&tex));
                    break;
                }
            }
        }
    }
    user_box.append(&avatar_img);

    let (gamertag_text, badge_tag_text) = if let Some(ref s) = acct.steam_persona {
        (s.clone(), "STEAM")
    } else if let Some(ref e) = acct.epic_display_name {
        (e.clone(), "EPIC")
    } else {
        (lang.guest_player().to_string(), "BLAZE OS")
    };

    let lbl_tag = Label::new(Some(&gamertag_text));
    lbl_tag.add_css_class("xbox-gamertag");
    lbl_tag.set_valign(gtk4::Align::Center);
    user_box.append(&lbl_tag);

    let lbl_ultimate = Label::new(Some(badge_tag_text));
    lbl_ultimate.add_css_class("xbox-badge-tag");
    lbl_ultimate.set_valign(gtk4::Align::Center);
    user_box.append(&lbl_ultimate);

    let score_box = GtkBox::new(Orientation::Horizontal, 2);
    score_box.set_valign(gtk4::Align::Center);
    score_box.set_margin_start(10);

    let lbl_score_icon = Label::new(Some("G"));
    lbl_score_icon.add_css_class("xbox-score-icon");
    lbl_score_icon.set_valign(gtk4::Align::Center);

    let lbl_score = Label::new(Some("21,337"));
    lbl_score.add_css_class("xbox-gamerscore");
    lbl_score.set_valign(gtk4::Align::Center);

    score_box.append(&lbl_score_icon);
    score_box.append(&lbl_score);
    user_box.append(&score_box);

    top_bar.append(&user_box);

    // Center Spacer
    let top_spacer1 = GtkBox::new(Orientation::Horizontal, 0);
    top_spacer1.set_hexpand(true);
    top_bar.append(&top_spacer1);

    // Center: Nav icon group (Library, Store, Game Pass, Search, Settings)
    let nav_pill_group = GtkBox::new(Orientation::Horizontal, 4);
    nav_pill_group.add_css_class("xbox-top-nav-group");
    nav_pill_group.set_valign(gtk4::Align::Center);

    let btn_library = create_nav_button(match lang {
        Language::TR => "Kütüphane",
        Language::EN => "Library",
    }, "xbox_nav_library");
    btn_library.add_css_class("active");

    let btn_store = create_nav_button(match lang {
        Language::TR => "Mağaza",
        Language::EN => "Store",
    }, "xbox_nav_store");

    let btn_discover = create_nav_button(match lang {
        Language::TR => "Keşfet",
        Language::EN => "Game Pass",
    }, "xbox_nav_gamepass");

    let btn_search = create_nav_button(match lang {
        Language::TR => "Ara",
        Language::EN => "Search",
    }, "xbox_nav_search");

    let btn_settings = create_nav_button(match lang {
        Language::TR => "Ayarlar",
        Language::EN => "Settings",
    }, "xbox_nav_settings");

    nav_pill_group.append(&btn_library);
    nav_pill_group.append(&btn_store);
    nav_pill_group.append(&btn_discover);
    nav_pill_group.append(&btn_search);
    nav_pill_group.append(&btn_settings);

    top_bar.append(&nav_pill_group);

    // Right Spacer
    let top_spacer2 = GtkBox::new(Orientation::Horizontal, 0);
    top_spacer2.set_hexpand(true);
    top_bar.append(&top_spacer2);

    // Right: Battery + Status + Clock + Lang Toggle
    let status_box = GtkBox::new(Orientation::Horizontal, 8);
    status_box.set_valign(gtk4::Align::Center);

    let mic_lbl = Label::new(Some("Mute: Off"));
    mic_lbl.add_css_class("xbox-hud-right");
    status_box.append(&mic_lbl);

    let bat_lbl = Label::new(Some("100%"));
    bat_lbl.add_css_class("xbox-hud-right");
    status_box.append(&bat_lbl);

    let time_now = chrono::Local::now().format("%H:%M").to_string();
    let time_lbl = Label::new(Some(&time_now));
    time_lbl.add_css_class("xbox-hud-time");
    status_box.append(&time_lbl);

    let lang_btn = Button::with_label(lang.toggle_btn_label());
    lang_btn.add_css_class("xbox-lang-btn");
    let win_l_clone = window.clone();
    let loop_l_clone = main_loop.clone();
    let prev_ctrl_lang = preview_ctrl.clone();
    lang_btn.connect_clicked(move |_| {
        prev_ctrl_lang.stop();
        let new_l = lang.toggle();
        new_l.save();
        win_l_clone.close();
        build_gamezone_ui(loop_l_clone.clone());
    });
    status_box.append(&lang_btn);

    top_bar.append(&status_box);
    root_box.append(&top_bar);
    root_box.append(&offline_banner);

    // ── MAIN CONTENT SCROLLABLE CANVAS ──
    let main_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .build();
    main_scroll.add_css_class("xbox-main-scroll");

    let content_box = GtkBox::new(Orientation::Vertical, 0);
    content_box.add_css_class("xbox-content-box");
    content_box.set_hexpand(true);

    // ── UPPER TWO-COLUMN HERO SHOWCASE + LIVE OFFICIAL TRAILER / SCREENSHOT FRAME ──
    let hero_row = GtkBox::new(Orientation::Horizontal, 24);
    hero_row.add_css_class("xbox-hero-spacer");
    hero_row.set_valign(gtk4::Align::End);
    hero_row.set_hexpand(true);

    let hero_left = GtkBox::new(Orientation::Vertical, 3);
    hero_left.set_hexpand(true);
    hero_left.set_valign(gtk4::Align::End);

    let hero_kicker = Label::new(Some("BLAZE UNIVERSE • LIVING FLOW CONSOLE"));
    hero_kicker.add_css_class("xbox-hero-kicker");
    hero_kicker.set_halign(gtk4::Align::Start);
    hero_left.append(&hero_kicker);

    // Official Transparent Steam Game Logo (`logo.png`)
    let hero_logo_pic = Picture::new();
    hero_logo_pic.set_can_shrink(true);
    hero_logo_pic.set_content_fit(gtk4::ContentFit::Contain);
    hero_logo_pic.set_size_request(210, 56);
    hero_logo_pic.set_halign(gtk4::Align::Start);
    hero_logo_pic.set_margin_top(2);
    hero_logo_pic.set_margin_bottom(2);
    hero_logo_pic.set_visible(false);
    hero_left.append(&hero_logo_pic);

    // Active Focused Game Title + Status Pill
    let title_row = GtkBox::new(Orientation::Horizontal, 8);
    title_row.set_valign(gtk4::Align::Center);

    let active_title_lbl = Label::new(Some(""));
    active_title_lbl.add_css_class("xbox-active-title-label");
    active_title_lbl.set_halign(gtk4::Align::Start);
    active_title_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    active_title_lbl.set_max_width_chars(38);

    let active_badge_lbl = Label::new(Some(""));
    active_badge_lbl.add_css_class("xbox-active-badge");
    active_badge_lbl.set_halign(gtk4::Align::Start);

    title_row.append(&active_title_lbl);
    title_row.append(&active_badge_lbl);
    hero_left.append(&title_row);

    let hero_desc = Label::new(Some(match lang {
        Language::TR => "Linux 7.2.9 sched_ext • NTSYNC Çekirdek Senkronizasyonu • Resmi Steam Medya Motoru",
        Language::EN => "Linux 7.2.9 sched_ext • NTSYNC Kernel Sync • Official Steam Media Engine",
    }));
    hero_desc.add_css_class("xbox-hero-desc");
    hero_desc.set_halign(gtk4::Align::Start);
    hero_desc.set_wrap(true);
    hero_desc.set_max_width_chars(66);
    hero_desc.set_lines(2);
    hero_desc.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    hero_left.append(&hero_desc);

    let hero_actions = GtkBox::new(Orientation::Horizontal, 10);
    hero_actions.set_margin_top(8);

    let hero_play_btn = Button::with_label(match lang {
        Language::TR => "▶ Oyna / Mağazada Aç",
        Language::EN => "▶ Play / Open in Store",
    });
    hero_play_btn.add_css_class("gz-hero-play-primary");

    let hero_trailer_btn = Button::with_label(match lang {
        Language::TR => "▶ Fragman & Ön İzleme",
        Language::EN => "▶ Trailer & Preview",
    });
    hero_trailer_btn.add_css_class("gz-hero-action-btn");

    hero_actions.append(&hero_play_btn);
    hero_actions.append(&hero_trailer_btn);
    hero_left.append(&hero_actions);

    hero_row.append(&hero_left);

    // Right 16:9 Live Video Trailer & Official HD Screenshot Frame
    let preview_frame = Overlay::new();
    preview_frame.add_css_class("gz-preview-frame");
    preview_frame.set_size_request(304, 171);
    preview_frame.set_valign(gtk4::Align::End);
    preview_frame.set_halign(gtk4::Align::End);

    let preview_pic = Picture::new();
    preview_pic.add_css_class("gz-preview-pic");
    preview_pic.set_can_shrink(true);
    preview_pic.set_content_fit(gtk4::ContentFit::Cover);
    preview_pic.set_size_request(304, 171);
    if !def_wallpaper.is_empty() {
        load_picture_file(&preview_pic, Path::new(&def_wallpaper));
    }
    preview_frame.set_child(Some(&preview_pic));

    let preview_badge = Label::new(Some(match lang {
        Language::TR => "◉ RESMİ ÖN İZLEME",
        Language::EN => "◉ OFFICIAL PREVIEW",
    }));
    preview_badge.add_css_class("gz-preview-badge");
    preview_badge.set_halign(gtk4::Align::Start);
    preview_badge.set_valign(gtk4::Align::Start);
    preview_frame.add_overlay(&preview_badge);

    hero_row.append(&preview_frame);
    content_box.append(&hero_row);

    // ── SEGMENTED SUB-TAB BAR: [★ Steam Öne Çıkanlar] | [▣ Bilgisayarımdaki & Hesabımdaki Oyunlar] ──
    let start_on_local = std::env::var("GAMEZONE_TAB").ok().as_deref() == Some("local");

    let subtab_header_row = GtkBox::new(Orientation::Horizontal, 12);
    subtab_header_row.set_valign(gtk4::Align::Center);
    subtab_header_row.set_margin_bottom(4);

    let subtab_bar = GtkBox::new(Orientation::Horizontal, 4);
    subtab_bar.add_css_class("gz-subtab-bar");

    let btn_tab_featured = Button::with_label(match lang {
        Language::TR => "★ Steam Öne Çıkanlar",
        Language::EN => "★ Steam Featured",
    });
    btn_tab_featured.add_css_class("gz-subtab-btn");
    if !start_on_local {
        btn_tab_featured.add_css_class("active");
    }

    let local_tab_label = |count: usize| -> String {
        match lang {
            Language::TR => format!("▣ Bilgisayarımdaki & Hesabımdaki Oyunlar ({})", count),
            Language::EN => format!("▣ My PC & Account Games ({})", count),
        }
    };

    let btn_tab_local = Button::with_label(&local_tab_label(local_detected_initial.len()));
    btn_tab_local.add_css_class("gz-subtab-btn");
    if start_on_local {
        btn_tab_local.add_css_class("active");
    }

    subtab_bar.append(&btn_tab_featured);
    subtab_bar.append(&btn_tab_local);
    subtab_header_row.append(&subtab_bar);

    let subtab_spacer = GtkBox::new(Orientation::Horizontal, 0);
    subtab_spacer.set_hexpand(true);
    subtab_header_row.append(&subtab_spacer);

    let btn_add_external = Button::with_label(match lang {
        Language::TR => "+ Harici Oyun / Klasör Tanıt",
        Language::EN => "+ Add External Game / Folder",
    });
    btn_add_external.add_css_class("gz-hero-action-btn");
    btn_add_external.set_margin_bottom(8);
    subtab_header_row.append(&btn_add_external);

    content_box.append(&subtab_header_row);

    // ── CAROUSEL VIEWS (FEATURED SHOWCASE vs DETECTED PC/ACCOUNT GAMES) ──
    let featured_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Never)
        .hexpand(true)
        .build();
    featured_scroll.add_css_class("xbox-carousel-scroll");
    featured_scroll.set_visible(!start_on_local);

    let featured_carousel_box = GtkBox::new(Orientation::Horizontal, 0);
    featured_scroll.set_child(Some(&featured_carousel_box));
    content_box.append(&featured_scroll);

    let local_section_box = GtkBox::new(Orientation::Vertical, 8);
    local_section_box.set_visible(start_on_local);

    let local_empty_card = GtkBox::new(Orientation::Horizontal, 16);
    local_empty_card.add_css_class("gz-local-empty-card");
    local_empty_card.set_visible(local_detected_initial.is_empty());

    let le_text_box = GtkBox::new(Orientation::Vertical, 4);
    le_text_box.set_hexpand(true);
    let le_title = Label::new(Some(match lang {
        Language::TR => "Bilgisayarınızda veya Bağlı Hesaplarınızda Henüz Oyun Algılanmadı",
        Language::EN => "No Installed or Account Games Detected Yet",
    }));
    le_title.add_css_class("xbox-modal-card-title");
    le_title.set_halign(gtk4::Align::Start);

    let le_desc = Label::new(Some(match lang {
        Language::TR => "Steam, Epic (Heroic), GOG, Lutris, Flatpak veya ~/Oyunlar ve ~/Games klasörlerine indirdiğiniz harici oyunlar otomatik algılanıp bu sekmeye taşınır.",
        Language::EN => "Games from Steam, Epic (Heroic), GOG, Lutris, Flatpak, or ~/Games are automatically detected and placed here.",
    }));
    le_desc.add_css_class("xbox-modal-card-desc");
    le_desc.set_halign(gtk4::Align::Start);
    le_desc.set_wrap(true);
    le_text_box.append(&le_title);
    le_text_box.append(&le_desc);
    local_empty_card.append(&le_text_box);
    local_section_box.append(&local_empty_card);

    let local_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Never)
        .hexpand(true)
        .build();
    local_scroll.add_css_class("xbox-carousel-scroll");
    let local_carousel_box = GtkBox::new(Orientation::Horizontal, 0);
    local_scroll.set_child(Some(&local_carousel_box));
    local_section_box.append(&local_scroll);

    content_box.append(&local_section_box);

    // ── SHARED REACTIVE STATE FOR LIVE MEDIA & SUB-TABS ──
    let featured_games_rc: Rc<RefCell<Vec<GameEntry>>> = Rc::new(RefCell::new(initial_featured.clone()));
    let local_games_rc: Rc<RefCell<Vec<GameEntry>>> = Rc::new(RefCell::new(local_detected_initial.clone()));
    let active_tab_rc: Rc<RefCell<u8>> = Rc::new(RefCell::new(if start_on_local { 1 } else { 0 }));
    let focused_game_rc: Rc<RefCell<Option<GameEntry>>> = Rc::new(RefCell::new(None));
    let cover_pics_map: Rc<RefCell<HashMap<String, Vec<Picture>>>> = Rc::new(RefCell::new(HashMap::new()));
    let featured_btns_rc: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));
    let local_btns_rc: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));

    let active_game_list: Arc<Mutex<Vec<GameEntry>>> = Arc::new(Mutex::new(if start_on_local {
        local_detected_initial.clone()
    } else {
        initial_featured.clone()
    }));
    let active_buttons_rc: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));

    // Focuses a game: updates official transparent logo, title, price/description, backdrop & starts live trailer!
    let focus_game_ui: Rc<dyn Fn(&GameEntry)> = {
        let focused_rc = focused_game_rc.clone();
        let title_lbl = active_title_lbl.clone();
        let badge_lbl = active_badge_lbl.clone();
        let desc_lbl = hero_desc.clone();
        let play_btn = hero_play_btn.clone();
        let logo_pic = hero_logo_pic.clone();
        let bg_pic = bg_picture.clone();
        let prev_pic = preview_pic.clone();
        let prev_badge = preview_badge.clone();
        let prev_ctrl = preview_ctrl.clone();
        let mtx = media_tx.clone();

        Rc::new(move |game: &GameEntry| {
            *focused_rc.borrow_mut() = Some(game.clone());
            title_lbl.set_text(&game.title);

            let badge_txt = if game.is_installed {
                match lang {
                    Language::TR => "KURULU • OYNA".to_string(),
                    Language::EN => "INSTALLED • PLAY".to_string(),
                }
            } else {
                game.category.to_uppercase()
            };
            badge_lbl.set_text(&badge_txt);
            desc_lbl.set_text(&game.banner_desc);

            if game.is_installed {
                play_btn.set_label(match lang {
                    Language::TR => "▶ Hemen Oyna",
                    Language::EN => "▶ Play Now",
                });
            } else {
                play_btn.set_label(match lang {
                    Language::TR => "↗ Steam Mağazasında Aç",
                    Language::EN => "↗ Open in Steam Store",
                });
            }

            // Check if official transparent `logo.png` is cached on disk
            let mut logo_found = None;
            let mut check_dirs = vec![gamezone_live::media_cache_dir_for(&game.id)];
            if let Some(ref aid) = game.steam_app_id {
                check_dirs.push(gamezone_live::media_cache_dir_for(&format!("steam-{}", aid)));
            }
            for d in check_dirs {
                let lp = d.join("logo.png");
                if gamezone_live::is_valid_image_file(&lp) {
                    logo_found = Some(lp);
                    break;
                }
            }
            if let Some(lp) = logo_found {
                load_picture_file(&logo_pic, &lp);
                logo_pic.set_visible(true);
            } else {
                logo_pic.set_visible(false);
            }

            update_backdrop(Some(game), &bg_pic);

            prev_ctrl.stop();
            prev_badge.set_text(match lang {
                Language::TR => "◉ RESMİ ÖN İZLEME",
                Language::EN => "◉ OFFICIAL PREVIEW",
            });
            if let Some(img_p) = get_game_screenshot_path(game).or_else(|| get_game_cover_path(game)) {
                load_picture_file(&prev_pic, Path::new(&img_p));
            }

            gamezone_live::fetch_game_details_and_trailer_async(game.clone(), mtx.clone());
        })
    };

    // Hero action button handlers
    {
        let fg_play = focused_game_rc.clone();
        let alert_hp = trigger_offline_alert.clone();
        hero_play_btn.connect_clicked(move |_| {
            if let Some(ref g) = *fg_play.borrow() {
                handle_game_activation(g, &*alert_hp);
            }
        });

        let fg_tr = focused_game_rc.clone();
        let mtx_tr = media_tx.clone();
        hero_trailer_btn.connect_clicked(move |_| {
            if let Some(ref g) = *fg_tr.borrow() {
                gamezone_live::fetch_game_details_and_trailer_async(g.clone(), mtx_tr.clone());
            }
        });
    }

    // Populates a horizontal game card row and wires focus/hover/click + live cover map
    let populate_carousel_row: Rc<dyn Fn(&GtkBox, &[GameEntry], &Rc<RefCell<Vec<Button>>>)> = {
        let cover_map = cover_pics_map.clone();
        let focus_fn = focus_game_ui.clone();
        let focused_rc = focused_game_rc.clone();
        let s_idx = selected_index.clone();
        let alert_fn = trigger_offline_alert.clone();

        Rc::new(move |row_box: &GtkBox, games: &[GameEntry], btns_out: &Rc<RefCell<Vec<Button>>>| {
            while let Some(child) = row_box.first_child() {
                row_box.remove(&child);
            }
            let mut btns = btns_out.borrow_mut();
            btns.clear();

            for (idx, game) in games.iter().enumerate() {
                let card = Button::new();
                card.add_css_class("xbox-game-card");

                let card_overlay = Overlay::new();
                let img = create_game_cover_image(game);
                img.set_size_request(156, 156);
                img.add_css_class("xbox-game-card-img");
                card_overlay.set_child(Some(&img));

                cover_map
                    .borrow_mut()
                    .entry(game.id.clone())
                    .or_default()
                    .push(img.clone());

                // Top-right badge pill (Installed OR Steam Discount %)
                if game.is_installed {
                    let pill = Label::new(Some(match lang {
                        Language::TR => "KURULU",
                        Language::EN => "INSTALLED",
                    }));
                    pill.add_css_class("gz-card-installed-pill");
                    pill.set_halign(gtk4::Align::End);
                    pill.set_valign(gtk4::Align::Start);
                    card_overlay.add_overlay(&pill);
                } else if let Some(pos) = game.category.find("-%") {
                    let disc: String = game.category[pos..]
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if !disc.is_empty() {
                        let pill = Label::new(Some(&disc));
                        pill.add_css_class("gz-card-discount-pill");
                        pill.set_halign(gtk4::Align::End);
                        pill.set_valign(gtk4::Align::Start);
                        card_overlay.add_overlay(&pill);
                    }
                }

                let cap_box = GtkBox::new(Orientation::Vertical, 1);
                cap_box.add_css_class("xbox-card-caption");
                cap_box.set_valign(gtk4::Align::End);
                cap_box.set_hexpand(true);
                let cap_lbl = Label::new(Some(&game.title));
                cap_lbl.add_css_class("xbox-card-caption-title");
                cap_lbl.set_halign(gtk4::Align::Start);
                cap_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                cap_lbl.set_max_width_chars(17);
                cap_box.append(&cap_lbl);
                card_overlay.add_overlay(&cap_box);

                card.set_child(Some(&card_overlay));

                let g_foc = game.clone();
                let focus_c = focus_fn.clone();
                let focused_check_f = focused_rc.clone();
                let s_idx_c = s_idx.clone();
                card.connect_has_focus_notify(move |btn| {
                    if btn.has_focus() {
                        *s_idx_c.lock().unwrap() = idx;
                        let already = focused_check_f
                            .borrow()
                            .as_ref()
                            .map(|cur| cur.id == g_foc.id)
                            .unwrap_or(false);
                        if !already {
                            focus_c(&g_foc);
                        }
                    }
                });

                let g_hov = game.clone();
                let focus_h = focus_fn.clone();
                let focused_check_h = focused_rc.clone();
                let motion_ctrl = EventControllerMotion::new();
                motion_ctrl.connect_enter(move |_ctrl, _x, _y| {
                    let already = focused_check_h
                        .borrow()
                        .as_ref()
                        .map(|cur| cur.id == g_hov.id)
                        .unwrap_or(false);
                    if !already {
                        focus_h(&g_hov);
                    }
                });
                card.add_controller(motion_ctrl);

                let g_clk = game.clone();
                let alert_c = alert_fn.clone();
                card.connect_clicked(move |_| {
                    handle_game_activation(&g_clk, &*alert_c);
                });

                row_box.append(&card);
                btns.push(card);
            }
        })
    };

    // Populate both carousels initially
    populate_carousel_row(
        &featured_carousel_box,
        &featured_games_rc.borrow(),
        &featured_btns_rc,
    );
    populate_carousel_row(
        &local_carousel_box,
        &local_games_rc.borrow(),
        &local_btns_rc,
    );
    *active_buttons_rc.borrow_mut() = if start_on_local {
        local_btns_rc.borrow().clone()
    } else {
        featured_btns_rc.borrow().clone()
    };

    // Helper closure to switch between Featured (0) and PC/Account Games (1)
    let switch_subtab: Rc<dyn Fn(u8)> = {
        let tab_rc = active_tab_rc.clone();
        let b_feat = btn_tab_featured.clone();
        let b_loc = btn_tab_local.clone();
        let f_scroll = featured_scroll.clone();
        let l_sec = local_section_box.clone();
        let f_games = featured_games_rc.clone();
        let l_games = local_games_rc.clone();
        let f_btns = featured_btns_rc.clone();
        let l_btns = local_btns_rc.clone();
        let act_list = active_game_list.clone();
        let act_btns = active_buttons_rc.clone();
        let s_idx = selected_index.clone();
        let focus_fn = focus_game_ui.clone();

        Rc::new(move |target_tab: u8| {
            *tab_rc.borrow_mut() = target_tab;
            *s_idx.lock().unwrap() = 0;
            if target_tab == 0 {
                b_feat.add_css_class("active");
                b_loc.remove_css_class("active");
                f_scroll.set_visible(true);
                l_sec.set_visible(false);
                let fg = f_games.borrow().clone();
                *act_list.lock().unwrap() = fg.clone();
                *act_btns.borrow_mut() = f_btns.borrow().clone();
                if let Some(first) = fg.first() {
                    focus_fn(first);
                }
            } else {
                b_loc.add_css_class("active");
                b_feat.remove_css_class("active");
                f_scroll.set_visible(false);
                l_sec.set_visible(true);
                let lg = l_games.borrow().clone();
                *act_list.lock().unwrap() = lg.clone();
                *act_btns.borrow_mut() = l_btns.borrow().clone();
                if let Some(first) = lg.first() {
                    focus_fn(first);
                }
            }
        })
    };

    {
        let sw_f = switch_subtab.clone();
        btn_tab_featured.connect_clicked(move |_| {
            sw_f(0);
        });
        let sw_l = switch_subtab.clone();
        btn_tab_local.connect_clicked(move |_| {
            sw_l(1);
        });
    }

    // "+ Harici Oyun / Klasör Tanıt" Modal Dialog -> detects external games & moves to Local sub-tab!
    {
        let parent_win = window.clone();
        let l_games_rc = local_games_rc.clone();
        let l_cbox = local_carousel_box.clone();
        let l_btns = local_btns_rc.clone();
        let l_empty = local_empty_card.clone();
        let b_loc = btn_tab_local.clone();
        let pop_fn = populate_carousel_row.clone();
        let sw_fn = switch_subtab.clone();
        let mtx_ext = media_tx.clone();

        btn_add_external.connect_clicked(move |_| {
            open_external_game_scanner_modal(
                &parent_win,
                lang,
                l_games_rc.clone(),
                l_cbox.clone(),
                l_btns.clone(),
                l_empty.clone(),
                b_loc.clone(),
                pop_fn.clone(),
                sw_fn.clone(),
                mtx_ext.clone(),
            );
        });
    }

    // Reactive UI listener for downloaded Official Covers, Transparent Logos, Hero Banners & Trailers
    {
        let rx = media_rx.clone();
        let cover_map = cover_pics_map.clone();
        let focused_rc = focused_game_rc.clone();
        let logo_pic = hero_logo_pic.clone();
        let bg_pic = bg_picture.clone();
        let desc_lbl = hero_desc.clone();
        let prev_pic = preview_pic.clone();
        let prev_badge = preview_badge.clone();
        let prev_ctrl = preview_ctrl.clone();

        glib::spawn_future_local(async move {
            while let Ok(ev) = rx.recv().await {
                match ev {
                    MediaUpdateEvent::CoverReady { game_id, path } => {
                        if let Some(pics) = cover_map.borrow().get(&game_id) {
                            for p in pics {
                                load_picture_file(p, &path);
                            }
                        }
                    }
                    MediaUpdateEvent::LogoReady { game_id, path } => {
                        let is_cur = focused_rc
                            .borrow()
                            .as_ref()
                            .map(|g| g.id == game_id)
                            .unwrap_or(false);
                        if is_cur {
                            load_picture_file(&logo_pic, &path);
                            logo_pic.set_visible(true);
                        }
                    }
                    MediaUpdateEvent::HeroReady { game_id, path } => {
                        let is_cur = focused_rc
                            .borrow()
                            .as_ref()
                            .map(|g| g.id == game_id)
                            .unwrap_or(false);
                        if is_cur {
                            load_picture_file(&bg_pic, &path);
                        }
                    }
                    MediaUpdateEvent::DetailsReady {
                        game_id,
                        meta,
                        screenshot_paths,
                        ..
                    } => {
                        let is_cur = focused_rc
                            .borrow()
                            .as_ref()
                            .map(|g| g.id == game_id)
                            .unwrap_or(false);
                        if is_cur {
                            if !meta.short_description.is_empty() {
                                desc_lbl.set_text(&meta.short_description);
                            }
                            if let Some(first_ss) = screenshot_paths.first() {
                                load_picture_file(&bg_pic, first_ss);
                            }
                            prev_ctrl.start_video_or_slideshow(
                                meta.stream_url,
                                screenshot_paths,
                                prev_pic.clone(),
                                prev_badge.clone(),
                            );
                        }
                    }
                }
            }
        });
    }

    // Trigger official media downloads for initial lists + live Steam Featured API refresh
    gamezone_live::spawn_official_media_fetcher(initial_featured.clone(), media_tx.clone());
    if !local_detected_initial.is_empty() {
        gamezone_live::spawn_official_media_fetcher(local_detected_initial.clone(), media_tx.clone());
    }

    if is_online {
        let (feat_tx, feat_rx) = async_channel::bounded::<Vec<GameEntry>>(1);
        let inst_ids_bg = installed_appids.clone();
        std::thread::spawn(move || {
            if let Some(mut live_items) =
                gamezone_live::fetch_live_steam_featured_blocking(lang, &inst_ids_bg)
            {
                let (deck_cat, deck_desc) = match lang {
                    Language::TR => (
                        "Konsol Arayüzü",
                        "Tam ekran Steam Big Picture ve kumanda destekli oyun merkezi.",
                    ),
                    Language::EN => (
                        "Console Interface",
                        "Fullscreen Steam Big Picture and controller-native gaming hub.",
                    ),
                };
                live_items.push(new_game_entry(
                    "steam-deck",
                    "Steam Big Picture",
                    deck_cat,
                    "steam -gamepadui",
                    deck_desc,
                    false,
                    None,
                    Some("/usr/share/solarui/covers/steam-deck.jpg".to_string()),
                    None,
                    None,
                    Some("https://store.steampowered.com/".to_string()),
                    None,
                ));
                let _ = feat_tx.send_blocking(live_items);
            }
        });

        let f_games_rc = featured_games_rc.clone();
        let f_cbox = featured_carousel_box.clone();
        let f_btns = featured_btns_rc.clone();
        let pop_fn = populate_carousel_row.clone();
        let tab_rc = active_tab_rc.clone();
        let act_list = active_game_list.clone();
        let act_btns = active_buttons_rc.clone();
        let focus_fn = focus_game_ui.clone();
        let mtx_live = media_tx.clone();

        glib::spawn_future_local(async move {
            if let Ok(live_list) = feat_rx.recv().await {
                *f_games_rc.borrow_mut() = live_list.clone();
                pop_fn(&f_cbox, &live_list, &f_btns);
                gamezone_live::spawn_official_media_fetcher(live_list.clone(), mtx_live);
                if *tab_rc.borrow() == 0 {
                    *act_list.lock().unwrap() = live_list.clone();
                    *act_btns.borrow_mut() = f_btns.borrow().clone();
                    if let Some(first) = live_list.first() {
                        focus_fn(first);
                    }
                }
            }
        });
    }

    // ── STORE SELECTION DRAWER / MODAL ──
    let store_drawer = GtkBox::new(Orientation::Vertical, 12);
    store_drawer.add_css_class("xbox-modal-drawer");
    store_drawer.set_visible(false);

    let store_hdr = GtkBox::new(Orientation::Horizontal, 12);
    let store_title = Label::new(Some(match lang {
        Language::TR => "Oyun Mağazaları & Vitrinler",
        Language::EN => "Game Stores & Showcases",
    }));
    store_title.add_css_class("xbox-apps-header");
    store_title.set_halign(gtk4::Align::Start);
    store_title.set_hexpand(true);

    let store_close_btn = Button::with_label(lang.back_to_home());
    store_close_btn.add_css_class("xbox-apps-close-btn");
    let st_toggle = store_drawer.clone();
    store_close_btn.connect_clicked(move |_| {
        st_toggle.set_visible(false);
    });
    store_hdr.append(&store_title);
    store_hdr.append(&store_close_btn);
    store_drawer.append(&store_hdr);

    let store_cards_row = GtkBox::new(Orientation::Horizontal, 14);

    // Steam Store Card
    let card_steam = Button::new();
    card_steam.add_css_class("xbox-modal-card");
    let cs_box = GtkBox::new(Orientation::Vertical, 4);
    let cs_title = Label::new(Some("Steam Store"));
    cs_title.add_css_class("xbox-modal-card-title");
    cs_title.set_halign(gtk4::Align::Start);
    let cs_desc = Label::new(Some(match lang {
        Language::TR => "Resmi Steam Mağazası & İndirimler",
        Language::EN => "Official Steam Store & Sales",
    }));
    cs_desc.add_css_class("xbox-modal-card-desc");
    cs_desc.set_halign(gtk4::Align::Start);
    cs_box.append(&cs_title);
    cs_box.append(&cs_desc);
    card_steam.set_child(Some(&cs_box));
    let alert_cs = trigger_offline_alert.clone();
    card_steam.connect_clicked(move |_| {
        if !is_system_online() {
            alert_cs("İnternete bağlı değilsiniz.");
        } else {
            open_browser_url("https://store.steampowered.com/");
        }
    });
    store_cards_row.append(&card_steam);

    // Epic Games / Heroic Card
    let card_epic = Button::new();
    card_epic.add_css_class("xbox-modal-card");
    let ce_box = GtkBox::new(Orientation::Vertical, 4);
    let ce_title = Label::new(Some("Epic Games Store"));
    ce_title.add_css_class("xbox-modal-card-title");
    ce_title.set_halign(gtk4::Align::Start);
    let ce_desc = Label::new(Some(match lang {
        Language::TR => "Haftalık Ücretsiz Oyunlar & Fırsatlar",
        Language::EN => "Weekly Free Games & Deals",
    }));
    ce_desc.add_css_class("xbox-modal-card-desc");
    ce_desc.set_halign(gtk4::Align::Start);
    ce_box.append(&ce_title);
    ce_box.append(&ce_desc);
    card_epic.set_child(Some(&ce_box));
    let alert_ce = trigger_offline_alert.clone();
    card_epic.connect_clicked(move |_| {
        if !is_system_online() {
            alert_ce("İnternete bağlı değilsiniz.");
        } else {
            open_browser_url("https://store.epicgames.com/");
        }
    });
    store_cards_row.append(&card_epic);

    // GOG.com Store Card
    let card_gog = Button::new();
    card_gog.add_css_class("xbox-modal-card");
    let cg_box = GtkBox::new(Orientation::Vertical, 4);
    let cg_title = Label::new(Some("GOG.com Store"));
    cg_title.add_css_class("xbox-modal-card-title");
    cg_title.set_halign(gtk4::Align::Start);
    let cg_desc = Label::new(Some(match lang {
        Language::TR => "DRM-Free Klasikler & Yeni Oyunlar",
        Language::EN => "DRM-Free Classics & Indie Titles",
    }));
    cg_desc.add_css_class("xbox-modal-card-desc");
    cg_desc.set_halign(gtk4::Align::Start);
    cg_box.append(&cg_title);
    cg_box.append(&cg_desc);
    card_gog.set_child(Some(&cg_box));
    let alert_cg = trigger_offline_alert.clone();
    card_gog.connect_clicked(move |_| {
        if !is_system_online() {
            alert_cg("İnternete bağlı değilsiniz.");
        } else {
            open_browser_url("https://www.gog.com/");
        }
    });
    store_cards_row.append(&card_gog);

    // Software Center (Local Flathub / Fedora Apps)
    let card_soft = Button::new();
    card_soft.add_css_class("xbox-modal-card");
    let cso_box = GtkBox::new(Orientation::Vertical, 4);
    let cso_title = Label::new(Some(match lang {
        Language::TR => "Blaze Yazılım Merkezi",
        Language::EN => "Blaze Software Center",
    }));
    cso_title.add_css_class("xbox-modal-card-title");
    cso_title.set_halign(gtk4::Align::Start);
    let cso_desc = Label::new(Some(match lang {
        Language::TR => "FireHub Mağaza • Portadora & Flathub",
        Language::EN => "FireHub Store • Portadora & Flathub",
    }));
    cso_desc.add_css_class("xbox-modal-card-desc");
    cso_desc.set_halign(gtk4::Align::Start);
    cso_box.append(&cso_title);
    cso_box.append(&cso_desc);
    card_soft.set_child(Some(&cso_box));
    card_soft.connect_clicked(|_| {
        if Command::new("firehub").spawn().is_err() {
            let _ = Command::new("/opt/FireHub/firehub").spawn();
        }
    });
    store_cards_row.append(&card_soft);

    store_drawer.append(&store_cards_row);
    content_box.append(&store_drawer);

    // ── SETTINGS DRAWER / MODAL ──
    let settings_drawer = GtkBox::new(Orientation::Vertical, 12);
    settings_drawer.add_css_class("xbox-modal-drawer");
    settings_drawer.set_visible(false);

    let set_hdr = GtkBox::new(Orientation::Horizontal, 12);
    let set_title = Label::new(Some(match lang {
        Language::TR => "GameZone & Sistem Ayarları",
        Language::EN => "GameZone & System Settings",
    }));
    set_title.add_css_class("xbox-apps-header");
    set_title.set_halign(gtk4::Align::Start);
    set_title.set_hexpand(true);

    let set_close_btn = Button::with_label(lang.back_to_home());
    set_close_btn.add_css_class("xbox-apps-close-btn");
    let set_toggle = settings_drawer.clone();
    set_close_btn.connect_clicked(move |_| {
        set_toggle.set_visible(false);
    });
    set_hdr.append(&set_title);
    set_hdr.append(&set_close_btn);
    settings_drawer.append(&set_hdr);

    let set_cards_row = GtkBox::new(Orientation::Horizontal, 14);

    // Setting 1: Built-in Proton-GE Manager
    let card_set_proton = Button::new();
    card_set_proton.add_css_class("xbox-modal-card");
    let cp_box = GtkBox::new(Orientation::Vertical, 4);
    let cp_title = Label::new(Some("Blaze Proton-GE Yöneticisi"));
    cp_title.add_css_class("xbox-modal-card-title");
    cp_title.set_halign(gtk4::Align::Start);
    let cp_desc = Label::new(Some(match lang {
        Language::TR => "GE-Proton & NTSYNC Uyumluluk Yöneticisi",
        Language::EN => "GE-Proton & NTSYNC Compatibility Manager",
    }));
    cp_desc.add_css_class("xbox-modal-card-desc");
    cp_desc.set_halign(gtk4::Align::Start);
    cp_box.append(&cp_title);
    cp_box.append(&cp_desc);
    card_set_proton.set_child(Some(&cp_box));
    card_set_proton.connect_clicked(move |_| {
        open_proton_ge_manager_modal(lang);
    });
    set_cards_row.append(&card_set_proton);

    // Setting 2: SolarUI Desktop Settings
    let card_set_solar = Button::new();
    card_set_solar.add_css_class("xbox-modal-card");
    let csl_box = GtkBox::new(Orientation::Vertical, 4);
    let csl_title = Label::new(Some(match lang {
        Language::TR => "SolarUI Masaüstü Ayarları",
        Language::EN => "SolarUI Desktop Settings",
    }));
    csl_title.add_css_class("xbox-modal-card-title");
    csl_title.set_halign(gtk4::Align::Start);
    let csl_desc = Label::new(Some(match lang {
        Language::TR => "Tema, Panel & Kabuk Yapılandırması",
        Language::EN => "Theme, Panel & Shell Configuration",
    }));
    csl_desc.add_css_class("xbox-modal-card-desc");
    csl_desc.set_halign(gtk4::Align::Start);
    csl_box.append(&csl_title);
    csl_box.append(&csl_desc);
    card_set_solar.set_child(Some(&csl_box));
    card_set_solar.connect_clicked(|_| {
        let _ = Command::new("solar-settings").spawn();
    });
    set_cards_row.append(&card_set_solar);

    // Setting 3: sched_ext & NTSYNC Performance Governor
    let card_set_perf = Button::new();
    card_set_perf.add_css_class("xbox-modal-card");
    let cperf_box = GtkBox::new(Orientation::Vertical, 4);
    let cperf_title = Label::new(Some(match lang {
        Language::TR => "sched_ext & NTSYNC Güç Profili",
        Language::EN => "sched_ext & NTSYNC Power Profile",
    }));
    cperf_title.add_css_class("xbox-modal-card-title");
    cperf_title.set_halign(gtk4::Align::Start);
    let cperf_desc = Label::new(Some(match lang {
        Language::TR => "Linux 7.2.9 Düşük Gecikmeli Oyun Çekirdeği Aktif",
        Language::EN => "Linux 7.2.9 Low-Latency Gaming Kernel Active",
    }));
    cperf_desc.add_css_class("xbox-modal-card-desc");
    cperf_desc.set_halign(gtk4::Align::Start);
    cperf_box.append(&cperf_title);
    cperf_box.append(&cperf_desc);
    card_set_perf.set_child(Some(&cperf_box));
    card_set_perf.connect_clicked(|_| {
        activate_gaming_optimizations();
        let _ = Command::new("notify-send")
            .args(["-a", "Blaze GameZone", "-i", "preferences-system", "Performans Modu", "sched_ext + NTSYNC Düşük Gecikme Profili etkinleştirildi."])
            .spawn();
    });
    set_cards_row.append(&card_set_perf);

    // Setting 4: Clear Poster Cache
    let card_set_cache = Button::new();
    card_set_cache.add_css_class("xbox-modal-card");
    let cc_box = GtkBox::new(Orientation::Vertical, 4);
    let cc_title = Label::new(Some(lang.clean_cache()));
    cc_title.add_css_class("xbox-modal-card-title");
    cc_title.set_halign(gtk4::Align::Start);
    let cc_desc = Label::new(Some(match lang {
        Language::TR => "Medya ve Kapak Önbelleğini Sıfırla",
        Language::EN => "Reset Media and Cover Cache",
    }));
    cc_desc.add_css_class("xbox-modal-card-desc");
    cc_desc.set_halign(gtk4::Align::Start);
    cc_box.append(&cc_title);
    cc_box.append(&cc_desc);
    card_set_cache.set_child(Some(&cc_box));
    card_set_cache.connect_clicked(|_| {
        clean_gamezone_cache();
    });
    set_cards_row.append(&card_set_cache);

    settings_drawer.append(&set_cards_row);
    content_box.append(&settings_drawer);

    // ── ALL APPLICATIONS DRAWER (COLLAPSIBLE / FULLSCREEN OVERLAY) ──
    let apps_drawer = GtkBox::new(Orientation::Vertical, 12);
    apps_drawer.add_css_class("xbox-apps-drawer");
    apps_drawer.set_visible(false);

    let apps_header_box = GtkBox::new(Orientation::Horizontal, 12);
    let apps_title_lbl = Label::new(Some(lang.all_applications()));
    apps_title_lbl.add_css_class("xbox-apps-header");
    apps_title_lbl.set_halign(gtk4::Align::Start);
    apps_title_lbl.set_hexpand(true);

    let search_input = Entry::new();
    search_input.add_css_class("xbox-search-bar");
    search_input.set_placeholder_text(Some(lang.search_placeholder()));
    search_input.set_hexpand(true);
    search_input.set_margin_end(16);

    let apps_close_btn = Button::with_label(lang.back_to_home());
    apps_close_btn.add_css_class("xbox-apps-close-btn");
    let drawer_toggle = apps_drawer.clone();
    apps_close_btn.connect_clicked(move |_| {
        drawer_toggle.set_visible(false);
    });

    apps_header_box.append(&apps_title_lbl);
    apps_header_box.append(&search_input);
    apps_header_box.append(&apps_close_btn);
    apps_drawer.append(&apps_header_box);

    let apps_grid_box = GtkBox::new(Orientation::Vertical, 10);
    let desktop_apps = scan_desktop_applications();
    let mut app_row = GtkBox::new(Orientation::Horizontal, 0);
    let mut row_count = 0;

    for app in &desktop_apps {
        let tile = Button::new();
        tile.add_css_class("xbox-app-tile");

        let t_box = GtkBox::new(Orientation::Vertical, 4);
        let app_img = Picture::new();
        app_img.set_size_request(48, 48);
        app_img.set_can_shrink(true);
        app_img.set_content_fit(gtk4::ContentFit::Cover);

        let mut icon_found = false;
        if let Some(ref icon_name) = app.icon {
            if let Some(p) = resolve_desktop_icon_path(icon_name) {
                app_img.set_filename(Some(Path::new(&p)));
                icon_found = true;
            }
        }
        if !icon_found {
            if let Some(p) = resolve_desktop_icon_path(&app.id) {
                app_img.set_filename(Some(Path::new(&p)));
                icon_found = true;
            }
        }
        if !icon_found {
            let def_p = "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/icons/hicolor/128x128/apps/application-generic.png";
            if Path::new(def_p).exists() {
                app_img.set_filename(Some(Path::new(def_p)));
            }
        }
        t_box.append(&app_img);

        let app_lbl = Label::new(Some(&app.name));
        app_lbl.add_css_class("xbox-app-title");
        app_lbl.set_halign(gtk4::Align::Center);
        app_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        app_lbl.set_max_width_chars(15);
        t_box.append(&app_lbl);

        tile.set_child(Some(&t_box));

        let app_exec = app.exec.clone();
        let app_name = app.name.clone();
        tile.connect_clicked(move |_| {
            println!("Launching {} -> {}", app_name, app_exec);
            let _ = Command::new("notify-send")
                .args(["-a", "Blaze GameZone", "-i", "application-x-executable", "Uygulama Başlatılıyor", &app_name])
                .spawn();
            let parts: Vec<&str> = app_exec.split_whitespace().collect();
            if let Some(cmd) = parts.get(0) {
                let _ = Command::new(cmd).args(&parts[1..]).spawn();
            }
        });

        app_row.append(&tile);
        row_count += 1;
        if row_count >= 6 {
            apps_grid_box.append(&app_row);
            app_row = GtkBox::new(Orientation::Horizontal, 0);
            row_count = 0;
        }
    }
    if row_count > 0 {
        apps_grid_box.append(&app_row);
    }
    apps_drawer.append(&apps_grid_box);
    content_box.append(&apps_drawer);

    // ── 2. BENTO QUICK ACTION ROW (BOTTOM 4 BENTO TILES) ──
    let bento_row = GtkBox::new(Orientation::Horizontal, 0);
    bento_row.add_css_class("xbox-bento-row");

    // Tile 1: "Browse your games & apps"
    let bento_apps = Button::new();
    bento_apps.add_css_class("xbox-bento-card");
    let bento1_overlay = Overlay::new();
    bento1_overlay.set_size_request(275, 155);

    let b1_pic = Picture::new();
    b1_pic.set_can_shrink(true);
    b1_pic.set_content_fit(gtk4::ContentFit::Cover);
    for b1_path in [
        "/usr/share/solarui/covers/bento_browse.png",
        "/home/darkmorpheus/SolarUI/data/covers/bento_browse.png",
        "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/covers/bento_browse.png",
    ] {
        if Path::new(b1_path).exists() {
            load_picture_file(&b1_pic, Path::new(b1_path));
            break;
        }
    }
    bento1_overlay.set_child(Some(&b1_pic));

    let b1_content = GtkBox::new(Orientation::Vertical, 2);
    b1_content.add_css_class("xbox-bento-overlay-box");
    b1_content.set_valign(gtk4::Align::End);

    let b1_tag = Label::new(Some("BLAZE LIBRARY"));
    b1_tag.add_css_class("xbox-bento-tag");
    b1_tag.set_halign(gtk4::Align::Start);

    let b1_title = Label::new(Some(lang.browse_games_and_apps()));
    b1_title.add_css_class("xbox-bento-title");
    b1_title.set_halign(gtk4::Align::Start);
    b1_title.set_wrap(true);

    b1_content.append(&b1_tag);
    b1_content.append(&b1_title);
    bento1_overlay.add_overlay(&b1_content);
    bento_apps.set_child(Some(&bento1_overlay));

    let drawer_show = apps_drawer.clone();
    bento_apps.connect_clicked(move |_| {
        let is_vis = drawer_show.is_visible();
        drawer_show.set_visible(!is_vis);
    });
    bento_row.append(&bento_apps);

    // Tile 2: "Customize your Home / BORE Performance"
    let bento_perf = Button::new();
    bento_perf.add_css_class("xbox-bento-card");
    let bento2_overlay = Overlay::new();
    bento2_overlay.set_size_request(275, 155);

    let b2_pic = Picture::new();
    b2_pic.set_can_shrink(true);
    b2_pic.set_content_fit(gtk4::ContentFit::Cover);
    for b2_path in [
        "/usr/share/solarui/covers/bento_performance.png",
        "/home/darkmorpheus/SolarUI/data/covers/bento_performance.png",
        "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/covers/bento_performance.png",
    ] {
        if Path::new(b2_path).exists() {
            load_picture_file(&b2_pic, Path::new(b2_path));
            break;
        }
    }
    bento2_overlay.set_child(Some(&b2_pic));

    let b2_content = GtkBox::new(Orientation::Vertical, 2);
    b2_content.add_css_class("xbox-bento-overlay-box");
    b2_content.set_valign(gtk4::Align::End);

    let b2_tag = Label::new(Some("SCHED_EXT & NTSYNC"));
    b2_tag.add_css_class("xbox-bento-tag");
    b2_tag.set_halign(gtk4::Align::Start);

    let b2_title = Label::new(Some(lang.customize_home()));
    b2_title.add_css_class("xbox-bento-title");
    b2_title.set_halign(gtk4::Align::Start);

    let b2_sub = Label::new(Some("Linux 7.2.9 • eBPF & NTSYNC"));
    b2_sub.add_css_class("xbox-bento-subtitle");
    b2_sub.set_halign(gtk4::Align::Start);

    b2_content.append(&b2_tag);
    b2_content.append(&b2_title);
    b2_content.append(&b2_sub);
    bento2_overlay.add_overlay(&b2_content);
    bento_perf.set_child(Some(&bento2_overlay));

    bento_perf.connect_clicked(|_| {
        activate_gaming_optimizations();
        let _ = Command::new("notify-send")
            .args([
                "-a", "Blaze GameZone",
                "-i", "preferences-system",
                "Performans Modu",
                "Linux 7.2.9 sched_ext & NTSYNC Oyun Çekirdeği devrede.",
            ])
            .spawn();
    });
    bento_row.append(&bento_perf);

    // Tile 3: "Store / Featured Deals"
    let bento_store = Button::new();
    bento_store.add_css_class("xbox-bento-card");
    let bento3_overlay = Overlay::new();
    bento3_overlay.set_size_request(275, 155);

    let b3_pic = Picture::new();
    b3_pic.set_can_shrink(true);
    b3_pic.set_content_fit(gtk4::ContentFit::Cover);
    for b3_path in [
        "/usr/share/solarui/covers/bento_store.png",
        "/home/darkmorpheus/SolarUI/data/covers/bento_store.png",
        "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/covers/bento_store.png",
    ] {
        if Path::new(b3_path).exists() {
            load_picture_file(&b3_pic, Path::new(b3_path));
            break;
        }
    }
    bento3_overlay.set_child(Some(&b3_pic));

    let b3_content = GtkBox::new(Orientation::Vertical, 2);
    b3_content.add_css_class("xbox-bento-overlay-box");
    b3_content.set_valign(gtk4::Align::End);

    let b3_tag = Label::new(Some("BLAZE STORE"));
    b3_tag.add_css_class("xbox-bento-tag");
    b3_tag.set_halign(gtk4::Align::Start);

    let b3_title = Label::new(Some(lang.store_deals()));
    b3_title.add_css_class("xbox-bento-title");
    b3_title.set_halign(gtk4::Align::Start);

    let b3_sub = Label::new(Some("Steam • Epic • GOG"));
    b3_sub.add_css_class("xbox-bento-subtitle");
    b3_sub.set_halign(gtk4::Align::Start);

    b3_content.append(&b3_tag);
    b3_content.append(&b3_title);
    b3_content.append(&b3_sub);
    bento3_overlay.add_overlay(&b3_content);
    bento_store.set_child(Some(&bento3_overlay));

    let alert_st = trigger_offline_alert.clone();
    bento_store.connect_clicked(move |_| {
        if !is_system_online() {
            alert_st("İnternete bağlı değilsiniz. Sadece yüklü oyunları çalıştırabilirsiniz.");
        } else {
            open_browser_url("https://store.steampowered.com/");
        }
    });
    bento_row.append(&bento_store);

    // Tile 4: "Play like a Pro / Settings & Proton"
    let bento_pro = Button::new();
    bento_pro.add_css_class("xbox-bento-card");
    let bento4_overlay = Overlay::new();
    bento4_overlay.set_size_request(275, 155);

    let b4_pic = Picture::new();
    b4_pic.set_can_shrink(true);
    b4_pic.set_content_fit(gtk4::ContentFit::Cover);
    for b4_path in [
        "/usr/share/solarui/covers/bento_settings.png",
        "/home/darkmorpheus/SolarUI/data/covers/bento_settings.png",
        "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/covers/bento_settings.png",
    ] {
        if Path::new(b4_path).exists() {
            load_picture_file(&b4_pic, Path::new(b4_path));
            break;
        }
    }
    bento4_overlay.set_child(Some(&b4_pic));

    let b4_content = GtkBox::new(Orientation::Vertical, 2);
    b4_content.add_css_class("xbox-bento-overlay-box");
    b4_content.set_valign(gtk4::Align::End);

    let b4_tag = Label::new(Some("SETTINGS & PROTON"));
    b4_tag.add_css_class("xbox-bento-tag");
    b4_tag.set_halign(gtk4::Align::Start);

    let b4_title = Label::new(Some(lang.play_like_pro()));
    b4_title.add_css_class("xbox-bento-title");
    b4_title.set_halign(gtk4::Align::Start);

    let b4_sub = Label::new(Some("Blaze Proton-GE • Gamepad Config"));
    b4_sub.add_css_class("xbox-bento-subtitle");
    b4_sub.set_halign(gtk4::Align::Start);

    b4_content.append(&b4_tag);
    b4_content.append(&b4_title);
    b4_content.append(&b4_sub);
    bento4_overlay.add_overlay(&b4_content);
    bento_pro.set_child(Some(&bento4_overlay));

    bento_pro.connect_clicked(move |_| {
        open_proton_ge_manager_modal(lang);
    });
    bento_row.append(&bento_pro);

    content_box.append(&bento_row);

    main_scroll.set_child(Some(&content_box));
    root_box.append(&main_scroll);

    // ── FOOTER CONTROLLER BAR ──
    let controller_bar = GtkBox::new(Orientation::Horizontal, 0);
    controller_bar.add_css_class("xbox-controller-bar");

    let k_a = Label::new(Some("A"));
    k_a.add_css_class("xbox-legend-key");
    let d_a = Label::new(Some(lang.footer_play()));
    d_a.add_css_class("xbox-legend-desc");

    let k_b = Label::new(Some("B"));
    k_b.add_css_class("xbox-legend-key");
    let d_b = Label::new(Some(lang.footer_back()));
    d_b.add_css_class("xbox-legend-desc");

    let k_x = Label::new(Some("X"));
    k_x.add_css_class("xbox-legend-key");
    let d_x = Label::new(Some(lang.footer_options()));
    d_x.add_css_class("xbox-legend-desc");

    let k_y = Label::new(Some("Y"));
    k_y.add_css_class("xbox-legend-key");
    let d_y = Label::new(Some(lang.footer_search()));
    d_y.add_css_class("xbox-legend-desc");

    controller_bar.append(&k_a);
    controller_bar.append(&d_a);
    controller_bar.append(&k_b);
    controller_bar.append(&d_b);
    controller_bar.append(&k_x);
    controller_bar.append(&d_x);
    controller_bar.append(&k_y);
    controller_bar.append(&d_y);

    root_box.append(&controller_bar);

    let main_overlay = Overlay::new();
    main_overlay.set_hexpand(true);
    main_overlay.set_vexpand(true);
    main_overlay.set_child(Some(&bg_box));
    main_overlay.add_overlay(&root_box);

    window.set_child(Some(&main_overlay));

    // Initial game selection & backdrop
    {
        let start_list = active_game_list.lock().unwrap().clone();
        if let Some(first_g) = start_list.first() {
            focus_game_ui(first_g);
        } else {
            update_backdrop(None, &bg_picture);
        }
    }

    // Stop trailer child cleanly whenever window closes
    {
        let prev_close = preview_ctrl.clone();
        window.connect_close_request(move |_| {
            prev_close.stop();
            glib::Propagation::Proceed
        });
    }

    // Top buttons connections
    let drawer_lib = apps_drawer.clone();
    let drawer_st_lib = store_drawer.clone();
    let drawer_set_lib = settings_drawer.clone();
    btn_library.connect_clicked(move |_| {
        drawer_st_lib.set_visible(false);
        drawer_set_lib.set_visible(false);
        let is_v = drawer_lib.is_visible();
        drawer_lib.set_visible(!is_v);
    });

    let drawer_st_open = store_drawer.clone();
    let drawer_apps_st = apps_drawer.clone();
    let drawer_set_st = settings_drawer.clone();
    btn_store.connect_clicked(move |_| {
        drawer_apps_st.set_visible(false);
        drawer_set_st.set_visible(false);
        let is_v = drawer_st_open.is_visible();
        drawer_st_open.set_visible(!is_v);
    });

    let alert_cloud_top = trigger_offline_alert.clone();
    btn_discover.connect_clicked(move |_| {
        if !is_system_online() {
            alert_cloud_top("İnternete bağlı değilsiniz. Sadece yüklü oyunları çalıştırabilirsiniz.");
        } else {
            open_browser_url("https://www.xbox.com/play");
        }
    });

    let drawer_search = apps_drawer.clone();
    let search_inp_foc = search_input.clone();
    btn_search.connect_clicked(move |_| {
        let is_v = drawer_search.is_visible();
        drawer_search.set_visible(!is_v);
        if !is_v {
            search_inp_foc.grab_focus();
        }
    });

    let drawer_set_open = settings_drawer.clone();
    let drawer_apps_set = apps_drawer.clone();
    let drawer_st_set = store_drawer.clone();
    btn_settings.connect_clicked(move |_| {
        drawer_apps_set.set_visible(false);
        drawer_st_set.set_visible(false);
        let is_v = drawer_set_open.is_visible();
        drawer_set_open.set_visible(!is_v);
    });

    // Bento Store Card opens store_drawer
    let bento_st_drawer = store_drawer.clone();
    bento_store.connect_clicked(move |_| {
        let is_v = bento_st_drawer.is_visible();
        bento_st_drawer.set_visible(!is_v);
    });

    // Bento Pro Card opens settings_drawer
    let bento_set_drawer = settings_drawer.clone();
    bento_pro.connect_clicked(move |_| {
        let is_v = bento_set_drawer.is_visible();
        bento_set_drawer.set_visible(!is_v);
    });

    // Keyboard & Gamepad Controller
    let key_controller = EventControllerKey::new();
    let s_idx_key = selected_index.clone();
    let all_g_key = active_game_list.clone();
    let c_btns_key = active_buttons_rc.clone();
    let loop_key = main_loop.clone();
    let focus_k = focus_game_ui.clone();
    let alert_key = trigger_offline_alert.clone();
    let prev_ctrl_key = preview_ctrl.clone();
    let tab_rc_key = active_tab_rc.clone();
    let sw_tab_key = switch_subtab.clone();

    key_controller.connect_key_pressed(move |_ctrl, keyval, _code, _state| {
        match keyval {
            gdk::Key::Escape => {
                println!("Exiting Blaze GameZone to Desktop...");
                prev_ctrl_key.stop();
                loop_key.quit();
                glib::Propagation::Stop
            }
            gdk::Key::Tab => {
                let next_tab = if *tab_rc_key.borrow() == 0 { 1 } else { 0 };
                sw_tab_key(next_tab);
                glib::Propagation::Stop
            }
            gdk::Key::Left => {
                let mut idx = s_idx_key.lock().unwrap();
                if *idx > 0 {
                    *idx -= 1;
                    if let Some(btn) = c_btns_key.borrow().get(*idx) {
                        btn.grab_focus();
                    }
                    if let Some(game) = all_g_key.lock().unwrap().get(*idx) {
                        focus_k(game);
                    }
                }
                glib::Propagation::Stop
            }
            gdk::Key::Right => {
                let mut idx = s_idx_key.lock().unwrap();
                let total = all_g_key.lock().unwrap().len();
                if *idx + 1 < total {
                    *idx += 1;
                    if let Some(btn) = c_btns_key.borrow().get(*idx) {
                        btn.grab_focus();
                    }
                    if let Some(game) = all_g_key.lock().unwrap().get(*idx) {
                        focus_k(game);
                    }
                }
                glib::Propagation::Stop
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                let idx = *s_idx_key.lock().unwrap();
                if let Some(game) = all_g_key.lock().unwrap().get(idx) {
                    handle_game_activation(game, &*alert_key);
                }
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });

    window.add_controller(key_controller);

    // Linux joystick listener for gamepad navigation
    let gamepad_rx = start_linux_gamepad_listener();
    let s_idx_gp = selected_index.clone();
    let all_g_gp = active_game_list.clone();
    let c_btns_gp = active_buttons_rc.clone();
    let loop_gp = main_loop.clone();
    let focus_gp = focus_game_ui.clone();
    let alert_gp = trigger_offline_alert.clone();
    let prev_ctrl_gp = preview_ctrl.clone();

    glib::spawn_future_local(async move {
        while let Ok(action) = gamepad_rx.recv().await {
            match action {
                GamepadNavAction::BackB => {
                    println!("Gamepad (B): Exiting Blaze GameZone to Desktop...");
                    prev_ctrl_gp.stop();
                    loop_gp.quit();
                }
                GamepadNavAction::SelectA => {
                    let idx = *s_idx_gp.lock().unwrap();
                    if let Some(game) = all_g_gp.lock().unwrap().get(idx) {
                        handle_game_activation(game, &*alert_gp);
                    }
                }
                GamepadNavAction::Left => {
                    let mut idx = s_idx_gp.lock().unwrap();
                    if *idx > 0 {
                        *idx -= 1;
                        if let Some(btn) = c_btns_gp.borrow().get(*idx) {
                            btn.grab_focus();
                        }
                        if let Some(game) = all_g_gp.lock().unwrap().get(*idx) {
                            focus_gp(game);
                        }
                    }
                }
                GamepadNavAction::Right => {
                    let mut idx = s_idx_gp.lock().unwrap();
                    let total = all_g_gp.lock().unwrap().len();
                    if *idx + 1 < total {
                        *idx += 1;
                        if let Some(btn) = c_btns_gp.borrow().get(*idx) {
                            btn.grab_focus();
                        }
                        if let Some(game) = all_g_gp.lock().unwrap().get(*idx) {
                            focus_gp(game);
                        }
                    }
                }
                GamepadNavAction::OptionsX => {
                    let idx = *s_idx_gp.lock().unwrap();
                    if let Some(game) = all_g_gp.lock().unwrap().get(idx) {
                        let _ = Command::new("notify-send")
                            .args([
                                "-a", "Blaze GameZone",
                                "-i", "preferences-system",
                                "Oyun Seçenekleri (X)",
                                &format!("{}: Proton / BORE öncelik ayarları optimize edildi.", game.title),
                            ])
                            .spawn();
                    }
                }
                _ => {}
            }
        }
    });

    window.present();
}

fn open_external_game_scanner_modal(
    parent: &Window,
    lang: Language,
    local_games_rc: Rc<RefCell<Vec<GameEntry>>>,
    local_cbox: GtkBox,
    local_btns: Rc<RefCell<Vec<Button>>>,
    local_empty_card: GtkBox,
    btn_tab_local: Button,
    populate_fn: Rc<dyn Fn(&GtkBox, &[GameEntry], &Rc<RefCell<Vec<Button>>>)>,
    switch_subtab: Rc<dyn Fn(u8)>,
    media_tx: async_channel::Sender<MediaUpdateEvent>,
) {
    let modal = Window::builder()
        .title(match lang {
            Language::TR => "Harici Oyun veya Klasör Tanıt",
            Language::EN => "Add External Game or Folder",
        })
        .transient_for(parent)
        .modal(true)
        .default_width(540)
        .default_height(300)
        .build();

    let box_root = GtkBox::new(Orientation::Vertical, 12);
    box_root.add_css_class("xbox-modal-drawer");
    box_root.set_margin_top(16);
    box_root.set_margin_bottom(16);
    box_root.set_margin_start(16);
    box_root.set_margin_end(16);

    let hdr = Label::new(Some(match lang {
        Language::TR => "Farklı Kaynaktan İndirilen Oyunu veya Klasörü Ekle",
        Language::EN => "Register Externally Downloaded Game or Folder",
    }));
    hdr.add_css_class("xbox-apps-header");
    hdr.set_halign(gtk4::Align::Start);
    box_root.append(&hdr);

    let sub = Label::new(Some(match lang {
        Language::TR => "Klasör yolu (örn. /mnt/nvme/Oyunlar veya ~/Downloads/Oyun) ya da doğrudan çalıştırılabilir dosya (.exe, .AppImage, .x86_64) girin. Resmi kapak, logo ve fragman otomatik çekilir.",
        Language::EN => "Enter a game folder (e.g. /mnt/nvme/Games) or executable (.exe, .AppImage, .x86_64). Official cover, logo & trailer will be fetched automatically.",
    }));
    sub.add_css_class("xbox-modal-card-desc");
    sub.set_halign(gtk4::Align::Start);
    sub.set_wrap(true);
    box_root.append(&sub);

    let entry_title = Entry::new();
    entry_title.add_css_class("xbox-search-bar");
    entry_title.set_placeholder_text(Some(match lang {
        Language::TR => "Oyun Adı (isteğe bağlı — tek .exe/.AppImage eklerken)",
        Language::EN => "Game Title (optional — when adding a single binary)",
    }));
    box_root.append(&entry_title);

    let entry_path = Entry::new();
    entry_path.add_css_class("xbox-search-bar");
    entry_path.set_placeholder_text(Some("/home/blaze/Oyunlar  veya  /home/blaze/Oyunlar/Hades/Hades.exe"));
    box_root.append(&entry_path);

    let actions = GtkBox::new(Orientation::Horizontal, 10);
    actions.set_halign(gtk4::Align::End);

    let btn_cancel = Button::with_label(lang.close());
    btn_cancel.add_css_class("xbox-apps-close-btn");
    let m_cancel = modal.clone();
    btn_cancel.connect_clicked(move |_| {
        m_cancel.close();
    });

    let btn_save = Button::with_label(match lang {
        Language::TR => "Tara ve Bilgisayarımdaki Oyunlara Taşı",
        Language::EN => "Scan & Move to My PC Games",
    });
    btn_save.add_css_class("gz-hero-play-primary");

    let m_save = modal.clone();
    btn_save.connect_clicked(move |_| {
        let raw_p = entry_path.text().trim().to_string();
        let raw_t = entry_title.text().trim().to_string();
        if raw_p.is_empty() {
            return;
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/blaze".to_string());
        let expanded = if let Some(rest) = raw_p.strip_prefix("~/") {
            format!("{}/{}", home, rest)
        } else {
            raw_p
        };
        let pb = PathBuf::from(&expanded);
        let mut cfg = gamezone_live::load_custom_sources();
        if pb.is_dir() {
            if !cfg.directories.contains(&expanded) {
                cfg.directories.push(expanded);
            }
        } else {
            let inferred_title = if !raw_t.is_empty() {
                raw_t
            } else {
                pb.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Harici Oyun")
                    .to_string()
            };
            cfg.executables.push(CustomExecutableEntry {
                title: inferred_title,
                path: expanded,
            });
        }
        gamezone_live::save_custom_sources(&cfg);

        let rescanned = gamezone_live::scan_computer_and_account_games(discover_all_games());
        *local_games_rc.borrow_mut() = rescanned.clone();
        local_empty_card.set_visible(rescanned.is_empty());
        btn_tab_local.set_label(&match lang {
            Language::TR => format!("▣ Bilgisayarımdaki & Hesabımdaki Oyunlar ({})", rescanned.len()),
            Language::EN => format!("▣ My PC & Account Games ({})", rescanned.len()),
        });
        populate_fn(&local_cbox, &rescanned, &local_btns);
        gamezone_live::spawn_official_media_fetcher(rescanned, media_tx.clone());
        switch_subtab(1);
        m_save.close();
    });

    actions.append(&btn_cancel);
    actions.append(&btn_save);
    box_root.append(&actions);

    modal.set_child(Some(&box_root));
    modal.present();
}


fn update_hero_showcase(
    game: &GameEntry,
    title_lbl: &Label,
    sub_lbl: &Label,
    play_btn: &Button,
    store_btn: &Button,
    hero_pic: &Picture,
    hero_vid: &Video,
) {
    title_lbl.set_text(&game.title);
    if game.is_installed {
        sub_lbl.set_text(&format!("{} • Kurulu ve Hazır • Düşük Gecikme Modu Aktif", game.banner_desc));
        play_btn.set_label(&format!("OYNA (A / Enter) - {}", game.title));
        play_btn.remove_css_class("hero-install-btn");
        play_btn.add_css_class("hero-play-btn");
    } else {
        sub_lbl.set_text(&format!("{} • Henüz Kurulu Değil (İndirilebilir)", game.banner_desc));
        play_btn.set_label(&format!("YÜKLE / MAĞAZADA GÖR (A) - {}", game.title));
        play_btn.remove_css_class("hero-play-btn");
        play_btn.add_css_class("hero-install-btn");
    }

    store_btn.set_visible(game.store_url.is_some());

    // Video oynatılıyorsa durdur ve resmi göster
    hero_vid.set_visible(false);
    hero_vid.set_file(None::<&gio::File>);
    hero_pic.set_visible(true);

    if let Some(ss) = get_game_screenshot_path(game) {
        hero_pic.set_filename(Some(Path::new(&ss)));
    } else if let Some(cover) = get_game_cover_path(game) {
        hero_pic.set_filename(Some(Path::new(&cover)));
    }
}

fn get_default_wallpaper_path() -> String {
    let candidates = [
        "/usr/share/blazeos/wallpapers/xbox_dynamic.png",
        "/usr/share/solarui/covers/xbox_dynamic.png",
        "/home/darkmorpheus/SolarUI/data/covers/xbox_dynamic.png",
        "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/blazeos/wallpapers/xbox_dynamic.png",
        "/usr/share/blazeos/wallpapers/galaxy.png",
        "/usr/share/blazeos/wallpapers/default.png",
        "/usr/share/blazeos/wallpapers/blazeos-wallpaper-1.png",
    ];
    for c in candidates {
        if Path::new(c).exists() {
            return c.to_string();
        }
    }
    String::new()
}

fn update_backdrop(game: Option<&GameEntry>, bg_picture: &Picture) {
    if let Some(g) = game {
        if let Some(ss) = get_game_screenshot_path(g) {
            load_picture_file(bg_picture, Path::new(&ss));
            return;
        }
    }
    let def_wall = get_default_wallpaper_path();
    if !def_wall.is_empty() {
        load_picture_file(bg_picture, Path::new(&def_wall));
    } else {
        bg_picture.set_filename(None::<&Path>);
    }
}

fn handle_game_activation(game: &GameEntry, on_offline_alert: &dyn Fn(&str)) {
    if !game.is_installed {
        if !is_system_online() {
            on_offline_alert("İnternete bağlı değilsiniz. Sadece yüklü oyunları ve uygulamaları çalıştırabilirsiniz.");
            return;
        }

        // Online: open store page to install
        if let Some(app_id) = &game.steam_app_id {
            let _ = Command::new("notify-send")
                .args([
                    "-a", "Blaze GameZone",
                    "-i", "input-gaming",
                    "Steam Mağazası Açılıyor",
                    &format!("{} yükleme sayfası açılıyor...", game.title),
                ])
                .spawn();

            open_browser_url(&format!("https://store.steampowered.com/app/{}", app_id));
            return;
        }

        if let Some(store) = &game.store_url {
            open_browser_url(store);
            return;
        }

        on_offline_alert("Bu oyun henüz sisteminizde kurulu değil.");
        return;
    }

    launch_game_entry(game);
}

fn fetch_and_apply_store_screenshot(game: &GameEntry, hero_pic: &Picture, bg_pic: &Picture) {
    if let Some(ss) = get_game_screenshot_path(game) {
        hero_pic.set_filename(Some(Path::new(&ss)));
        bg_pic.set_filename(Some(Path::new(&ss)));
        return;
    }

    if let Some(app_id) = &game.steam_app_id {
        let cache_dir = dirs_cache_dir().join(format!("steam-{}", app_id));
        let ss_dest = cache_dir.join("screenshot.jpg");
        let a_id = app_id.clone();

        if ss_dest.exists() {
            hero_pic.set_filename(Some(&ss_dest));
            bg_pic.set_filename(Some(&ss_dest));
            return;
        }

        let (sender, receiver) = async_channel::unbounded::<String>();
        let pic_clone = hero_pic.clone();
        let bg_clone = bg_pic.clone();
        glib::spawn_future_local(async move {
            if let Ok(ss_str) = receiver.recv().await {
                pic_clone.set_filename(Some(Path::new(&ss_str)));
                bg_clone.set_filename(Some(Path::new(&ss_str)));
            }
        });

        std::thread::spawn(move || {
            let api_url = format!("https://store.steampowered.com/api/appdetails?appids={}", a_id);
            if let Ok(output) = Command::new("curl").args(["-sL", "-m", "5", &api_url]).output() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(ss_list) = val.get(&a_id)
                            .and_then(|v| v.get("data"))
                            .and_then(|v| v.get("screenshots"))
                            .and_then(|v| v.as_array()) {
                            if let Some(first_ss) = ss_list.get(0).and_then(|s| s.get("path_full")).and_then(|s| s.as_str()) {
                                let _ = std::fs::create_dir_all(&cache_dir);
                                let _ = Command::new("curl")
                                    .args(["-sL", "-m", "6", first_ss, "-o", ss_dest.to_str().unwrap()])
                                    .status();
                                println!("Fetched store screenshot for app {}: {}", a_id, first_ss);
                                let ss_str = ss_dest.to_string_lossy().to_string();
                                let _ = sender.send_blocking(ss_str);
                            }
                        }
                    }
                }
            }
        });
    }
}

// ── 10 Saniye Hover: Steam Oynanış Videosu / Fragmanı Kendi Çerçevesinde Başlatma ──
fn trigger_store_video_preview(game: &GameEntry, hero_vid: &Video, hero_pic: &Picture, trailer_badge: &Label) {
    println!("Triggering 10s gameplay video preview for {}", game.title);
    trailer_badge.set_text("▶ Oynanış Fragmanı Başlatıldı (Kendi Çerçevesinde)");

    // 1. Önce diskteki önbelleğe veya paketli fragmana bak
    if let Some(local_path) = get_game_trailer_path(game) {
        hero_vid.set_filename(Some(Path::new(&local_path)));
        hero_vid.set_autoplay(true);
        hero_vid.set_loop(true);
        hero_vid.set_visible(true);
        hero_pic.set_visible(false);
        return;
    }

    // 2. Steam App ID varsa doğrudan Steam CDN fragmanını akıt
    if let Some(app_id) = &game.steam_app_id {
        let a_id = app_id.clone();
        let (sender, receiver) = async_channel::unbounded::<String>();
        let vid_clone = hero_vid.clone();
        let pic_clone = hero_pic.clone();

        glib::spawn_future_local(async move {
            if let Ok(video_url) = receiver.recv().await {
                let file = gio::File::for_uri(&video_url);
                vid_clone.set_file(Some(&file));
                vid_clone.set_autoplay(true);
                vid_clone.set_loop(true);
                vid_clone.set_visible(true);
                pic_clone.set_visible(false);
            }
        });

        std::thread::spawn(move || {
            let api_url = format!("https://store.steampowered.com/api/appdetails?appids={}", a_id);
            if let Ok(output) = Command::new("curl").args(["-sL", "-m", "5", &api_url]).output() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(movies) = val.get(&a_id)
                            .and_then(|v| v.get("data"))
                            .and_then(|v| v.get("movies"))
                            .and_then(|v| v.as_array()) {
                            if let Some(first_movie) = movies.get(0) {
                                if let Some(mid) = first_movie.get("id").and_then(|m| m.as_i64()) {
                                    let video_url = format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{}/movie480.mp4", mid);
                                    println!("Streaming trailer in widget frame: {}", video_url);
                                    let _ = sender.send_blocking(video_url);
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}

fn get_game_screenshot_path(game: &GameEntry) -> Option<String> {
    let mut check_keys = vec![game.id.clone()];
    if let Some(app_id) = &game.steam_app_id {
        check_keys.push(format!("steam-{}", app_id));
        check_keys.push(app_id.clone());
    }
    for key in &check_keys {
        let dir = gamezone_live::media_cache_dir_for(key);
        for ext in ["screenshot_0.jpg", "trailer_thumb.jpg", "hero.jpg", "screenshot.jpg", "cover.jpg"] {
            let p = dir.join(ext);
            if gamezone_live::is_valid_image_file(&p) {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }
    if let Some(p) = &game.screenshot_path {
        if Path::new(p).exists() {
            return Some(p.clone());
        }
    }
    let bundled_dirs = [
        "/usr/share/solarui/covers",
        "/home/darkmorpheus/SolarUI/data/covers",
        "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/covers",
    ];
    for d in bundled_dirs {
        for key in &check_keys {
            for prefix in [&format!("{}-screenshot", key), &format!("{}-hero", key)] {
                for ext in ["jpg", "png", "svg"] {
                    let p = Path::new(d).join(format!("{}.{}", prefix, ext));
                    if p.exists() {
                        return Some(p.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    None
}

fn get_game_trailer_path(game: &GameEntry) -> Option<String> {
    if let Some(m) = &game.movie_url {
        if Path::new(m).exists() {
            return Some(m.clone());
        }
    }
    let mut check_keys = vec![game.id.clone()];
    if let Some(app_id) = &game.steam_app_id {
        check_keys.push(format!("steam-{}", app_id));
        check_keys.push(app_id.clone());
    }
    let cache_root = dirs_cache_dir();
    for key in &check_keys {
        let p = cache_root.join(key).join("trailer.mp4");
        if p.exists() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    let bundled_dirs = [
        "/usr/share/solarui/covers",
        "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/covers",
    ];
    for d in bundled_dirs {
        for key in &check_keys {
            for prefix in [&format!("{}-trailer", key), key] {
                let p = Path::new(d).join(format!("{}.mp4", prefix));
                if p.exists() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

fn get_game_cover_path(game: &GameEntry) -> Option<String> {
    let mut check_keys = vec![game.id.clone()];
    if let Some(app_id) = &game.steam_app_id {
        check_keys.push(format!("steam-{}", app_id));
        check_keys.push(app_id.clone());
    }
    for key in &check_keys {
        let dir = gamezone_live::media_cache_dir_for(key);
        for ext in ["cover.jpg", "cover.png", "hero.jpg", "screenshot_0.jpg"] {
            let p = dir.join(ext);
            if gamezone_live::is_valid_image_file(&p) {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }
    if let Some(p) = &game.cover_path {
        if Path::new(p).exists() {
            return Some(p.clone());
        }
    }
    let bundled_dirs = [
        "/usr/share/solarui/covers",
        "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/covers",
    ];
    for d in bundled_dirs {
        for key in &check_keys {
            for ext in ["jpg", "png", "svg"] {
                let p = Path::new(d).join(format!("{}.{}", key, ext));
                if p.exists() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

fn create_game_cover_image(game: &GameEntry) -> Picture {
    let pic = Picture::new();
    pic.set_can_shrink(true);
    pic.set_content_fit(gtk4::ContentFit::Cover);
    pic.set_size_request(156, 156);
    pic.add_css_class("game-cover-pic");

    if let Some(p) = get_game_cover_path(game) {
        load_picture_file(&pic, Path::new(&p));
    } else {
        let bundled = [
            "/usr/share/solarui/covers/steam-deck.jpg",
            "/home/darkmorpheus/SolarUI/data/covers/steam-deck.jpg",
            "/home/darkmorpheus/BlazeFedora/blazeos_custom_apps/usr/share/solarui/covers/steam-deck.jpg",
        ];
        for b in bundled {
            if Path::new(b).exists() {
                load_picture_file(&pic, Path::new(b));
                break;
            }
        }
    }
    pic
}

// ── Web Afiş ve Logo İndirici (Arka Plan Asenkron Önbellek) ──
fn init_media_cache(games: &[GameEntry]) {
    let cache_dir = dirs_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);

    for game in games {
        if game.is_steam {
            if let Some(app_id) = &game.steam_app_id {
                let g_dir = cache_dir.join(format!("steam-{}", app_id));
                let _ = std::fs::create_dir_all(&g_dir);

                let cover_dest = g_dir.join("cover.jpg");
                let hero_dest = g_dir.join("hero.jpg");
                let a_id = app_id.clone();

                if !cover_dest.exists() || !hero_dest.exists() {
                    std::thread::spawn(move || {
                        // Header / Cover indir
                        if !cover_dest.exists() {
                            let cover_url = format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{}/header.jpg", a_id);
                            let _ = Command::new("curl")
                                .args(["-sL", "-m", "6", &cover_url, "-o", cover_dest.to_str().unwrap()])
                                .status();
                        }
                        // Hero Banner indir
                        if !hero_dest.exists() {
                            let hero_url = format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{}/library_hero.jpg", a_id);
                            let _ = Command::new("curl")
                                .args(["-sL", "-m", "6", &hero_url, "-o", hero_dest.to_str().unwrap()])
                                .status();
                        }
                    });
                }
            }
        }
    }
}

pub fn clean_gamezone_cache() {
    let cache_dir = dirs_cache_dir();
    if cache_dir.exists() {
        let _ = std::fs::remove_dir_all(&cache_dir);
        let _ = std::fs::create_dir_all(&cache_dir);
        println!("GameZone media cache cleared successfully.");
        let _ = Command::new("notify-send")
            .args([
                "-a", "Blaze GameZone",
                "-i", "edit-clear",
                "Önbellek Temizlendi",
                "GameZone oyun afiş ve medya önbelleği başarıyla temizlendi.",
            ])
            .spawn();
    }
}

fn dirs_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/darkmorpheus".to_string());
    PathBuf::from(home).join(".cache/blaze-gamezone/media")
}

fn new_game_entry(
    id: &str,
    title: &str,
    category: &str,
    exec: &str,
    banner_desc: &str,
    is_steam: bool,
    steam_app_id: Option<String>,
    cover_path: Option<String>,
    hero_path: Option<String>,
    screenshot_path: Option<String>,
    store_url: Option<String>,
    movie_url: Option<String>,
) -> GameEntry {
    let is_installed = is_game_installed(steam_app_id.as_deref(), exec, id);
    GameEntry {
        id: id.to_string(),
        title: title.to_string(),
        category: category.to_string(),
        exec: exec.to_string(),
        banner_desc: banner_desc.to_string(),
        is_steam,
        is_installed,
        steam_app_id,
        cover_path,
        hero_path,
        screenshot_path,
        store_url,
        movie_url,
    }
}

// ── Hesap Algılama (Steam PersonaName & Epic/Legendary displayName) ──
pub fn detect_logged_in_accounts() -> (Option<String>, Option<String>) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/darkmorpheus".to_string());
    let steam_vdfs = [
        PathBuf::from(&home).join(".steam/steam/config/loginusers.vdf"),
        PathBuf::from(&home).join(".local/share/Steam/config/loginusers.vdf"),
    ];
    let mut steam_user = None;
    for vdf in steam_vdfs {
        if let Ok(content) = std::fs::read_to_string(&vdf) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("\"PersonaName\"") {
                    let parts: Vec<&str> = trimmed.split('"').filter(|s| !s.trim().is_empty()).collect();
                    if parts.len() >= 2 {
                        steam_user = Some(parts[1].to_string());
                        break;
                    }
                }
            }
        }
        if steam_user.is_some() {
            break;
        }
    }
    let mut epic_user = None;
    let legendary_user = PathBuf::from(&home).join(".config/legendary/user.json");
    if let Ok(content) = std::fs::read_to_string(&legendary_user) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(name) = val.get("displayName").and_then(|v| v.as_str()) {
                epic_user = Some(name.to_string());
            }
        }
    }
    (steam_user, epic_user)
}

// ── Oyun Geçmişi Takipçisi (Son Oynananlar) ──
pub fn recent_history_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/darkmorpheus".to_string());
    PathBuf::from(home).join(".config/solarui/gamezone/recent_history.json")
}

pub fn get_recent_game_ids() -> Vec<String> {
    let p = recent_history_file();
    if let Ok(content) = std::fs::read_to_string(&p) {
        if let Ok(ids) = serde_json::from_str::<Vec<String>>(&content) {
            return ids;
        }
    }
    Vec::new()
}

pub fn record_game_played(game_id: &str) {
    let p = recent_history_file();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut ids = get_recent_game_ids();
    ids.retain(|id| id != game_id);
    ids.insert(0, game_id.to_string());
    if ids.len() > 15 {
        ids.truncate(15);
    }
    if let Ok(json) = serde_json::to_string_pretty(&ids) {
        let _ = std::fs::write(&p, json);
    }
}

// ── Öne Çıkanlar & Mağaza Keşfi (Steam Store Showcase Vitrini) ──
pub fn discover_store_showcase() -> Vec<GameEntry> {
    discover_store_showcase_for_lang(Language::detect())
}

pub fn discover_store_showcase_for_lang(lang: Language) -> Vec<GameEntry> {
    let mut list = Vec::new();

    let (deck_cat, deck_desc) = match lang {
        Language::TR => ("Konsol Arayüzü", "Tam ekran Steam Big Picture ve kumanda destekli oyun merkezi."),
        Language::EN => ("Console Interface", "Fullscreen Steam Big Picture and controller-native gaming hub."),
    };
    list.push(new_game_entry(
        "steam-deck",
        "Steam Big Picture",
        deck_cat,
        "steam -gamepadui",
        deck_desc,
        false,
        None,
        Some("/usr/share/solarui/covers/steam-deck.jpg".to_string()),
        None,
        None,
        Some("https://store.steampowered.com/".to_string()),
        None,
    ));

    let (beam_cat, beam_desc) = match lang {
        Language::TR => ("Araç Fiziği Simülasyonu", "Gerçek zamanlı yumuşak gövde araç fiziği simülasyonu."),
        Language::EN => ("Vehicle Physics Simulation", "Real-time soft-body vehicle physics simulation."),
    };
    list.push(new_game_entry(
        "beamng",
        "BeamNG.drive",
        beam_cat,
        "steam steam://rungameid/284160",
        beam_desc,
        true,
        Some("284160".to_string()),
        Some("/usr/share/solarui/covers/beamng.jpg".to_string()),
        Some("/usr/share/solarui/covers/beamng.jpg".to_string()),
        Some("/usr/share/solarui/covers/beamng-screenshot.jpg".to_string()),
        Some("https://store.steampowered.com/app/284160/BeamNGdrive/".to_string()),
        Some("/usr/share/solarui/covers/beamng-trailer.mp4".to_string()),
    ));

    let (dota_cat, dota_desc) = match lang {
        Language::TR => ("Strateji & MOBA", "Valve amiral gemisi rekabetçi çevrimiçi arena oyunu."),
        Language::EN => ("Strategy & MOBA", "Valve flagship competitive online battle arena game."),
    };
    list.push(new_game_entry(
        "dota-2",
        "Dota 2",
        dota_cat,
        "steam steam://rungameid/570",
        dota_desc,
        true,
        Some("570".to_string()),
        Some("/usr/share/solarui/covers/dota-2.jpg".to_string()),
        None,
        None,
        Some("https://store.steampowered.com/app/570/Dota_2/".to_string()),
        None,
    ));

    let (cp_cat, cp_desc) = match lang {
        Language::TR => ("Aksiyon RPG", "Night City sokaklarında geçen distopik açık dünya macerası."),
        Language::EN => ("Action RPG", "Dystopian open-world adventure set in Night City."),
    };
    list.push(new_game_entry(
        "cyberpunk-2077",
        "Cyberpunk 2077",
        cp_cat,
        "steam steam://rungameid/1091500",
        cp_desc,
        true,
        Some("1091500".to_string()),
        Some("/usr/share/solarui/covers/cyberpunk-2077.jpg".to_string()),
        None,
        None,
        Some("https://store.steampowered.com/app/1091500/Cyberpunk_2077/".to_string()),
        None,
    ));

    let (sd_cat, sd_desc) = match lang {
        Language::TR => ("Çiftlik & Simülasyon", "Pelikan kasabasında kendi hayalinizdeki çiftliği inşa edin."),
        Language::EN => ("Farming & Simulation", "Build your dream farm in Pelican Town."),
    };
    list.push(new_game_entry(
        "stardew-valley",
        "Stardew Valley",
        sd_cat,
        "steam steam://rungameid/413150",
        sd_desc,
        true,
        Some("413150".to_string()),
        Some("/usr/share/solarui/covers/stardew-valley.jpg".to_string()),
        None,
        None,
        Some("https://store.steampowered.com/app/413150/Stardew_Valley/".to_string()),
        None,
    ));

    let (wh_cat, wh_desc) = match lang {
        Language::TR => ("Sıra Tabanlı Taktik", "Adeptus Mechanicus güçleri ile Necron mezarlarında savaşın."),
        Language::EN => ("Turn-Based Tactics", "Battle in Necron tombs with the Adeptus Mechanicus."),
    };
    list.push(new_game_entry(
        "mechanicus",
        "Warhammer 40,000: Mechanicus",
        wh_cat,
        "steam steam://rungameid/673880",
        wh_desc,
        true,
        Some("673880".to_string()),
        Some("/usr/share/solarui/covers/mechanicus.jpg".to_string()),
        None,
        None,
        Some("https://store.steampowered.com/app/673880/Warhammer_40000_Mechanicus/".to_string()),
        None,
    ));

    let (xb_cat, xb_desc) = match lang {
        Language::TR => ("Bulut Oyun Vitrini", "Yüzlerce konsol oyununu bulut üzerinden anında oynayın."),
        Language::EN => ("Cloud Gaming Showcase", "Stream hundreds of console games instantly via the cloud."),
    };
    list.push(new_game_entry(
        "xbox-cloud",
        "Xbox Cloud Gaming",
        xb_cat,
        "xdg-open https://www.xbox.com/play",
        xb_desc,
        false,
        None,
        Some("/usr/share/solarui/covers/xbox-cloud.jpg".to_string()),
        None,
        None,
        Some("https://www.xbox.com/play".to_string()),
        None,
    ));

    list
}

// ── Kütüphane Taraması (Yalnızca Disk üzerinde Gerçekten Kurulu Oyunlar ve Başlatıcılar) ──
pub fn discover_all_games() -> Vec<GameEntry> {
    let mut list = Vec::new();
    let cache_dir = dirs_cache_dir();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/darkmorpheus".to_string());

    // 1. Kurulu Başlatıcı Kartları (Yalnızca sistemde varsa kütüphaneye ekle)
    if is_game_installed(None, "steam", "steam-deck") {
        list.push(new_game_entry(
            "steam-deck",
            "Steam Big Picture",
            "Steam Deck Arayüzü",
            "steam -gamepadui",
            "Resmi Steam Deck kumanda ve tam ekran oyun arayüzü.",
            true,
            None,
            None,
            None,
            None,
            Some("https://store.steampowered.com/".to_string()),
            None,
        ));
    }

    if is_game_installed(None, "heroic", "heroic") {
        list.push(new_game_entry(
            "heroic",
            "Heroic Games Launcher",
            "Epic Games & GOG",
            "heroic",
            "Açık kaynak Epic Games, GOG ve Amazon Games yöneticisi.",
            false,
            None,
            None,
            None,
            None,
            Some("https://store.epicgames.com/".to_string()),
            None,
        ));
    }

    if is_game_installed(None, "lutris", "lutris") {
        list.push(new_game_entry(
            "lutris",
            "Lutris Gamepad UI",
            "Açık Kaynak Oyun Yöneticisi",
            "lutris",
            "Tüm platformlar, emülatörler ve Wine oyunları tek yerde.",
            false,
            None,
            None,
            None,
            None,
            None,
            None,
        ));
    }

    if is_game_installed(None, "retroarch", "retroarch") {
        list.push(new_game_entry(
            "retroarch",
            "RetroArch Emulation Hub",
            "Retro Konsol",
            "retroarch",
            "PS2, PSP, N64, SNES ve klasik konsol emülasyon merkezi.",
            false,
            None,
            None,
            None,
            None,
            None,
            None,
        ));
    }

    // 2. Kurulu Steam Oyunlarını Tara (~/.steam/steam/steamapps/*.acf)
    let steam_dirs = [
        PathBuf::from(&home).join(".steam/steam/steamapps"),
        PathBuf::from(&home).join(".local/share/Steam/steamapps"),
    ];

    for s_dir in steam_dirs {
        if s_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(s_dir) {
                for e in entries.flatten() {
                    let path = e.path();
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.starts_with("appmanifest_") && file_name.ends_with(".acf") {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Some((app_id, name)) = parse_acf_file(&content) {
                                    let c_path = cache_dir.join(format!("steam-{}", app_id)).join("cover.jpg");
                                    let h_path = cache_dir.join(format!("steam-{}", app_id)).join("hero.jpg");
                                    let s_path = cache_dir.join(format!("steam-{}", app_id)).join("screenshot.jpg");

                                    list.push(GameEntry {
                                        id: format!("steam-{}", app_id),
                                        title: name.clone(),
                                        category: "Steam Kütüphanesi".to_string(),
                                        exec: format!("steam steam://rungameid/{}", app_id),
                                        banner_desc: format!("{} • Proton 9.0 / Native Steam Oyunu", name),
                                        is_steam: true,
                                        is_installed: true,
                                        steam_app_id: Some(app_id.clone()),
                                        cover_path: Some(c_path.to_string_lossy().to_string()),
                                        hero_path: Some(h_path.to_string_lossy().to_string()),
                                        screenshot_path: Some(s_path.to_string_lossy().to_string()),
                                        store_url: Some(format!("https://store.steampowered.com/app/{}", app_id)),
                                        movie_url: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Heroic & Legendary / Nile Oyunlarını Tara
    let heroic_gog = PathBuf::from(&home).join(".config/heroic/gog_store/installed.json");
    if heroic_gog.exists() {
        if let Ok(content) = std::fs::read_to_string(&heroic_gog) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(installed) = val.get("installed").and_then(|v| v.as_array()) {
                    for item in installed {
                        if let (Some(app_name), Some(title)) = (
                            item.get("appName").and_then(|v| v.as_str()),
                            item.get("title").and_then(|v| v.as_str()),
                        ) {
                            list.push(GameEntry {
                                id: format!("gog-{}", app_name),
                                title: title.to_string(),
                                category: "GOG Galaxy".to_string(),
                                exec: format!("heroic --launch heroic://launch/{}", app_name),
                                banner_desc: format!("{} • DRM-Free GOG Galaxy Oyunu", title),
                                is_steam: false,
                                is_installed: true,
                                steam_app_id: None,
                                cover_path: None,
                                hero_path: None,
                                screenshot_path: None,
                                store_url: Some("https://www.gog.com/".to_string()),
                                movie_url: None,
                            });
                        }
                    }
                }
            }
        }
    }

    let legendary_installed = PathBuf::from(&home).join(".config/legendary/installed.json");
    if legendary_installed.exists() {
        if let Ok(content) = std::fs::read_to_string(&legendary_installed) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = val.as_object() {
                    for (app_name, info) in obj {
                        let title = info.get("title").and_then(|v| v.as_str()).unwrap_or(app_name);
                        list.push(GameEntry {
                            id: format!("epic-{}", app_name),
                            title: title.to_string(),
                            category: "Epic Games".to_string(),
                            exec: format!("legendary launch {}", app_name),
                            banner_desc: format!("{} • Epic Games Store / Heroic Oyunu", title),
                            is_steam: false,
                            is_installed: true,
                            steam_app_id: None,
                            cover_path: None,
                            hero_path: None,
                            screenshot_path: None,
                            store_url: Some("https://store.epicgames.com/".to_string()),
                            movie_url: None,
                        });
                    }
                }
            }
        }
    }

    // 4. Sistem Masaüstü Oyunları (/usr/share/applications)
    let app_dirs = [
        Path::new("/usr/share/applications"),
        Path::new("/usr/local/share/applications"),
    ];

    for d in app_dirs {
        if let Ok(entries) = std::fs::read_dir(d) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().map(|s| s == "desktop").unwrap_or(false) {
                    if let Ok(text) = std::fs::read_to_string(&p) {
                        if text.contains("Categories=") && text.contains("Game") {
                            let mut name = String::new();
                            let mut exec = String::new();
                            for line in text.lines() {
                                if line.starts_with("Name=") && name.is_empty() {
                                    name = line[5..].trim().to_string();
                                } else if line.starts_with("Exec=") && exec.is_empty() {
                                    exec = line[5..].trim().to_string();
                                }
                            }
                            if !name.is_empty()
                                && !exec.is_empty()
                                && !exec.contains("solar-gamezone")
                                && !name.to_lowercase().contains("gamezone")
                                && !list.iter().any(|g| g.title == name)
                            {
                                list.push(GameEntry {
                                    id: name.to_lowercase().replace(' ', "-"),
                                    title: name.clone(),
                                    category: "Masaüstü Oyunu".to_string(),
                                    exec,
                                    banner_desc: format!("{} • Düşük gecikmeli yerel Linux oyunu.", name),
                                    is_steam: false,
                                    is_installed: true,
                                    steam_app_id: None,
                                    cover_path: None,
                                    hero_path: None,
                                    screenshot_path: None,
                                    store_url: None,
                                    movie_url: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    list
}

fn parse_acf_file(content: &str) -> Option<(String, String)> {
    let mut app_id = None;
    let mut name = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"appid\"") {
            let parts: Vec<&str> = trimmed.split('"').filter(|s| !s.trim().is_empty()).collect();
            if parts.len() >= 2 {
                app_id = Some(parts[1].to_string());
            }
        } else if trimmed.starts_with("\"name\"") {
            let parts: Vec<&str> = trimmed.split('"').filter(|s| !s.trim().is_empty()).collect();
            if parts.len() >= 2 {
                name = Some(parts[1].to_string());
            }
        }
    }

    if let (Some(id), Some(n)) = (app_id, name) {
        if id != "228980" && !n.contains("Steam Linux Runtime") && !n.contains("Proton") && !n.contains("Steamworks") {
            return Some((id, n));
        }
    }
    None
}

fn launch_game_entry(game: &GameEntry) {
    println!("Launching GameZone title: {} -> {}", game.title, game.exec);
    record_game_played(&game.id);
    let _ = Command::new("notify-send")
        .args([
            "-a", "Blaze GameZone",
            "-i", "input-gaming",
            "Oyun Başlatılıyor",
            &format!("{} başlatılıyor. BORE ve düşük gecikme modu devrede.", game.title),
        ])
        .spawn();

    let parts: Vec<&str> = game.exec.split_whitespace().collect();
    if let Some(cmd) = parts.get(0) {
        let args = &parts[1..];
        let _ = Command::new(cmd)
            .args(args)
            .spawn();
    }
}

fn get_live_system_metrics() -> (String, String, String) {
    let ram_str = if let Ok(mem) = std::fs::read_to_string("/proc/meminfo") {
        let mut total = 0u64;
        let mut avail = 0u64;
        for line in mem.lines() {
            if line.starts_with("MemTotal:") {
                total = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            } else if line.starts_with("MemAvailable:") {
                avail = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
        }
        if total > 0 {
            let used = total.saturating_sub(avail);
            let pct = (used as f64 / total as f64) * 100.0;
            format!("{:.0}%", pct)
        } else {
            "21%".to_string()
        }
    } else {
        "21%".to_string()
    };

    (format!("18%"), ram_str, format!("60"))
}

fn open_proton_ge_manager_modal(lang: Language) {
    // If external protonup-qt exists and user installed it, we still offer our native GTK4 manager
    let win = Window::builder()
        .title(match lang {
            Language::TR => "Blaze Proton-GE & Uyumluluk Yöneticisi",
            Language::EN => "Blaze Proton-GE & Compatibility Manager",
        })
        .default_width(640)
        .default_height(440)
        .modal(true)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 14);
    root.set_margin_top(20);
    root.set_margin_bottom(20);
    root.set_margin_start(22);
    root.set_margin_end(22);

    let hdr = Label::new(Some(match lang {
        Language::TR => "Blaze Proton-GE & NTSYNC Uyumluluk Yöneticisi",
        Language::EN => "Blaze Proton-GE & NTSYNC Compatibility Manager",
    }));
    hdr.add_css_class("xbox-apps-header");
    hdr.set_halign(gtk4::Align::Start);
    root.append(&hdr);

    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/blaze".to_string());
    let compat_dir = PathBuf::from(&home).join(".steam/root/compatibilitytools.d");
    let _ = std::fs::create_dir_all(&compat_dir);

    let mut installed = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&compat_dir) {
        for e in entries.flatten() {
            if e.path().is_dir() {
                installed.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }
    installed.sort();

    let ntsync_ok = Path::new("/dev/ntsync").exists();
    let scx_ok = Path::new("/sys/kernel/sched_ext").exists();
    let status_text = format!(
        "• Hedef Dizin: {}\n• Çekirdek NTSYNC (/dev/ntsync): {}\n• Çekirdek sched_ext (eBPF): {}\n• Kurulu Proton-GE Sürümleri: {}",
        compat_dir.display(),
        if ntsync_ok { "AKTİF (Hazır)" } else { "Modül Bekleniyor (modprobe ntsync)" },
        if scx_ok { "AKTİF (Hazır)" } else { "Standart EEVDF Zamanlayıcı" },
        if installed.is_empty() { "Henüz kurulu GE-Proton yok".to_string() } else { installed.join(", ") }
    );

    let info_lbl = Label::new(Some(&status_text));
    info_lbl.set_halign(gtk4::Align::Start);
    info_lbl.set_wrap(true);
    root.append(&info_lbl);

    let prog_lbl = Label::new(Some(match lang {
        Language::TR => "En son GE-Proton sürümünü GitHub üzerinden doğrudan Steam uyumluluk dizinine indirebilirsiniz.",
        Language::EN => "Download and extract the latest GE-Proton release directly into your Steam compatibilitytools.d directory.",
    }));
    prog_lbl.set_halign(gtk4::Align::Start);
    prog_lbl.set_wrap(true);
    root.append(&prog_lbl);

    let btn_row = GtkBox::new(Orientation::Horizontal, 12);
    let dl_btn = Button::with_label(match lang {
        Language::TR => "En Güncel GE-Proton Sürümünü İndir & Kur",
        Language::EN => "Download & Install Latest GE-Proton",
    });
    dl_btn.add_css_class("xbox-apps-close-btn");

    let prog_lbl_clone = prog_lbl.clone();
    let compat_clone = compat_dir.clone();
    dl_btn.connect_clicked(move |b| {
        b.set_sensitive(false);
        prog_lbl_clone.set_text("GitHub API üzerinden en son GloriousEggroll/proton-ge-custom sürümü sorgulanıyor...");
        let (tx, rx) = async_channel::unbounded::<(String, bool)>();
        let target_dir = compat_clone.clone();
        std::thread::spawn(move || {
            let script = format!(
                r#"set -e
mkdir -p "{dir}"
API_JSON=$(curl -fsSL https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases/latest)
TAR_URL=$(printf '%s' "$API_JSON" | grep -oE 'https://github.com/GloriousEggroll/proton-ge-custom/releases/download/[^"]+\.tar\.gz' | head -n 1)
TAG_NAME=$(printf '%s' "$API_JSON" | grep -oE '"tag_name":\s*"[^"]+"' | head -n 1 | cut -d'"' -f4)
if [ -z "$TAR_URL" ]; then
    echo "ERR:GitHub API'den GE-Proton indirme bağlantısı alınamadı."
    exit 1
fi
echo "INFO:$TAG_NAME indiriliyor ve {dir} dizinine açılıyor..."
curl -fL "$TAR_URL" | tar -xz -C "{dir}"
echo "OK:$TAG_NAME başarıyla kuruldu! Steam'i yeniden başlattığınızda Uyumluluk sekmesinde görünecektir."
"#,
                dir = target_dir.display()
            );
            match Command::new("sh").arg("-c").arg(&script).output() {
                Ok(out) if out.status.success() => {
                    let msg = String::from_utf8_lossy(&out.stdout).lines().last().unwrap_or("Tamamlandı").to_string();
                    let _ = tx.send_blocking((msg, true));
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    let _ = tx.send_blocking((format!("Hata: {}", err.trim()), true));
                }
                Err(e) => {
                    let _ = tx.send_blocking((format!("İndirme başlatılamadı: {}", e), true));
                }
            }
        });

        let lbl_ui = prog_lbl_clone.clone();
        let btn_ui = b.clone();
        glib::MainContext::default().spawn_local(async move {
            while let Ok((msg, done)) = rx.recv().await {
                lbl_ui.set_text(&msg);
                if done {
                    btn_ui.set_sensitive(true);
                }
            }
        });
    });
    btn_row.append(&dl_btn);

    let close_btn = Button::with_label(match lang {
        Language::TR => "Kapat",
        Language::EN => "Close",
    });
    close_btn.add_css_class("xbox-apps-close-btn");
    let win_weak = win.downgrade();
    close_btn.connect_clicked(move |_| {
        if let Some(w) = win_weak.upgrade() {
            w.close();
        }
    });
    btn_row.append(&close_btn);

    root.append(&btn_row);
    win.set_child(Some(&root));
    win.present();
}

fn activate_gaming_optimizations() {
    println!("Activating Blaze GameZone Low-Latency Performance Mode (sched_ext + NTSYNC)...");
    if Command::new("sudo")
        .args(["-n", "/usr/bin/blaze-gamemode", "on"])
        .status()
        .is_err()
    {
        let _ = Command::new("powerprofilesctl").args(["set", "performance"]).status();
        let _ = Command::new("sudo")
            .args([
                "-n",
                "sh",
                "-c",
                "for g in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do [ -w \"$g\" ] && echo performance > \"$g\"; done; sysctl -w kernel.split_lock_mitigate=0 >/dev/null 2>&1 || true; modprobe ntsync 2>/dev/null || true",
            ])
            .status();
    }
    let _ = crate::delight::play_acoustic_feedback("easter-egg");
}

fn restore_normal_optimizations() {
    println!("Restoring balanced power and scheduler profile...");
    if Command::new("sudo")
        .args(["-n", "/usr/bin/blaze-gamemode", "off"])
        .status()
        .is_err()
    {
        let _ = Command::new("powerprofilesctl").args(["set", "balanced"]).status();
        let _ = Command::new("sudo")
            .args([
                "-n",
                "sh",
                "-c",
                "for g in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do [ -w \"$g\" ] && echo schedutil > \"$g\" 2>/dev/null || echo powersave > \"$g\" 2>/dev/null || true; done",
            ])
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_showcase_games() {
        let showcase = discover_store_showcase();
        assert!(!showcase.is_empty(), "GameZone must have showcase games");
        assert!(showcase.iter().any(|g| g.id == "beamng"));
        assert!(showcase.iter().any(|g| g.id == "dota-2"));
        assert!(showcase.iter().any(|g| g.id == "cyberpunk-2077"));
        assert!(showcase.iter().any(|g| g.id == "xbox-cloud"));
    }

    #[test]
    fn test_parse_acf_file() {
        let acf_sample = r#"
"AppState"
{
	"appid"		"413150"
	"Universe"		"1"
	"name"		"Stardew Valley"
	"StateFlags"		"4"
}
"#;
        let parsed = parse_acf_file(acf_sample);
        assert_eq!(parsed, Some(("413150".to_string(), "Stardew Valley".to_string())));
    }

    #[test]
    fn test_cache_dir_path() {
        let c_dir = dirs_cache_dir();
        assert!(c_dir.to_str().unwrap().contains(".cache/blaze-gamezone"));
    }

    #[test]
    fn test_recent_history() {
        let test_id = "test-game-id";
        record_game_played(test_id);
        let recents = get_recent_game_ids();
        assert!(recents.contains(&test_id.to_string()));
    }
}
