// ==============================================================================
// Blaze GameZone — Live Official Media, Steam Featured Showcase & Multi-Source
// Local / Account Game Discovery Engine
//
// Provides:
// 1. Live Steam Featured & Top Showcase API client (`store.steampowered.com/api/featured`)
//    with offline-first JSON caching (`~/.cache/blaze-gamezone/featured_cache.json`).
// 2. Official Steam CDN + AppDetails Media Pipeline:
//    - Real cover artwork (`library_600x900.jpg` / hashed `capsule_616x353.jpg` / `header.jpg`)
//    - Real official transparent PNG game logo (`logo.png`)
//    - Real 1080p store screenshots & hero banners (`library_hero.jpg`, `screenshots[*].path_full`)
//    - Real gameplay video trailers (`hls_h264` / `mp4` / `webm` from `appdetails` `movies[0]`)
//    - Automatic Steam `storesearch` AppID resolution for non-Steam / externally downloaded games!
// 3. Zero-GStreamer-Dependency Live MJPEG Frame Trailer Player (`ffmpeg` -> `gdk::Texture`)
//    with atomic cancellation, zero leaked child processes, and automatic screenshot slideshow fallback.
// 4. Multi-Source Computer & Linked Account Game Detector:
//    - Steam multi-drive `libraryfolders.vdf` + `appmanifest_*.acf`
//    - Signed-in Steam account (`loginusers.vdf`, `avatarcache`, `localconfig.vdf`)
//    - Heroic Games Launcher (Epic Games, GOG, Amazon Prime, Sideloaded apps + official art URLs)
//    - Lutris (`~/.config/lutris/games/*.yml`), Bottles, Flatpak & `.desktop` games
//    - Standalone / externally downloaded games (`~/Games`, `~/Oyunlar`, `~/Downloads`,
//      `~/İndirilenler`, `/opt/games`, plus user-configured custom directories & executables)
// ==============================================================================

use gtk4::gdk;
use gtk4::glib;
use gtk4::{Label, Picture};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::gamezone_gui::{GameEntry, Language};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedFeaturedItem {
    pub app_id: String,
    pub title: String,
    pub header_url: String,
    pub capsule_url: String,
    pub discount_percent: i64,
    pub final_price_str: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TrailerMetadata {
    pub title: String,
    pub short_description: String,
    pub stream_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub screenshot_urls: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CustomGameSources {
    pub directories: Vec<String>,
    pub executables: Vec<CustomExecutableEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomExecutableEntry {
    pub title: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub enum MediaUpdateEvent {
    CoverReady { game_id: String, path: PathBuf },
    LogoReady { game_id: String, path: PathBuf },
    HeroReady { game_id: String, path: PathBuf },
    DetailsReady {
        game_id: String,
        app_id: String,
        meta: TrailerMetadata,
        screenshot_paths: Vec<PathBuf>,
    },
}

pub fn gamezone_cache_root() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/blaze".to_string());
    PathBuf::from(home).join(".cache/blaze-gamezone")
}

pub fn media_cache_dir_for(key: &str) -> PathBuf {
    gamezone_cache_root().join("media").join(sanitize_key(key))
}

fn sanitize_key(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

pub fn custom_sources_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/blaze".to_string());
    PathBuf::from(home).join(".config/solarui/gamezone/custom_sources.json")
}

pub fn load_custom_sources() -> CustomGameSources {
    let p = custom_sources_path();
    if let Ok(txt) = std::fs::read_to_string(&p) {
        if let Ok(parsed) = serde_json::from_str::<CustomGameSources>(&txt) {
            return parsed;
        }
    }
    CustomGameSources::default()
}

pub fn save_custom_sources(cfg: &CustomGameSources) {
    let p = custom_sources_path();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(&p, json);
    }
}

/// Validates that a downloaded image file actually starts with JPEG/PNG/WebP magic bytes
/// and isn't an HTML 404/redirect error page.
pub fn is_valid_image_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if meta.len() < 512 {
        return false;
    }
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let mut hdr = [0u8; 12];
    if f.read_exact(&mut hdr).is_err() {
        return false;
    }
    // JPEG: FF D8 FF | PNG: 89 50 4E 47 | RIFF....WEBP
    let is_jpeg = hdr[0] == 0xFF && hdr[1] == 0xD8 && hdr[2] == 0xFF;
    let is_png = hdr[0] == 0x89 && hdr[1] == b'P' && hdr[2] == b'N' && hdr[3] == b'G';
    let is_webp = &hdr[0..4] == b"RIFF" && &hdr[8..12] == b"WEBP";
    is_jpeg || is_png || is_webp
}

fn download_url_verified(url: &str, dest: &Path, timeout_secs: u32) -> bool {
    if url.trim().is_empty() {
        return false;
    }
    if is_valid_image_file(dest) {
        return true;
    }
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = dest.with_extension("part");
    let ok = Command::new("curl")
        .args([
            "-fsSL",
            "-A",
            "Mozilla/5.0 (X11; Linux x86_64) BlazeGameZone/1.0",
            "-m",
            &timeout_secs.to_string(),
            url,
            "-o",
            &tmp.to_string_lossy(),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok && is_valid_image_file(&tmp) {
        let _ = std::fs::rename(&tmp, dest);
        true
    } else {
        let _ = std::fs::remove_file(&tmp);
        false
    }
}

fn curl_json(url: &str, timeout_secs: u32) -> Option<serde_json::Value> {
    let out = Command::new("curl")
        .args([
            "-fsSL",
            "-A",
            "Mozilla/5.0 (X11; Linux x86_64) BlazeGameZone/1.0",
            "-m",
            &timeout_secs.to_string(),
            url,
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

fn url_encode(input: &str) -> String {
    let mut out = String::new();
    for b in input.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                out.push(*b as char);
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{:02X}", other)),
        }
    }
    out
}

/// Loads cached Steam Featured games immediately if present on disk.
pub fn load_cached_featured_games(lang: Language, installed_ids: &HashSet<String>) -> Vec<GameEntry> {
    let cache_file = gamezone_cache_root().join("featured_cache.json");
    if let Ok(txt) = std::fs::read_to_string(&cache_file) {
        if let Ok(items) = serde_json::from_str::<Vec<CachedFeaturedItem>>(&txt) {
            if !items.is_empty() {
                return featured_items_to_entries(&items, lang, installed_ids);
            }
        }
    }
    Vec::new()
}

/// Fetches live Steam Featured games from `https://store.steampowered.com/api/featured/`.
pub fn fetch_live_steam_featured_blocking(
    lang: Language,
    installed_ids: &HashSet<String>,
) -> Option<Vec<GameEntry>> {
    let (cc, l_param) = match lang {
        Language::TR => ("tr", "turkish"),
        Language::EN => ("us", "english"),
    };
    let url = format!(
        "https://store.steampowered.com/api/featured/?cc={}&l={}",
        cc, l_param
    );
    let val = curl_json(&url, 8)?;

    let mut seen = HashSet::new();
    let mut items = Vec::new();

    for key in ["large_capsules", "featured_linux", "featured_win"] {
        if let Some(arr) = val.get(key).and_then(|v| v.as_array()) {
            for entry in arr {
                let Some(id_num) = entry.get("id").and_then(|v| v.as_i64()) else {
                    continue;
                };
                let app_id = id_num.to_string();
                if !seen.insert(app_id.clone()) {
                    continue;
                }
                let Some(name) = entry.get("name").and_then(|v| v.as_str()) else {
                    continue;
                };
                let header_url = entry
                    .get("header_image")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let capsule_url = entry
                    .get("large_capsule_image")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&header_url)
                    .to_string();
                let discount_percent = entry
                    .get("discount_percent")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let final_price_str = entry
                    .get("final_price")
                    .and_then(|v| v.as_i64())
                    .map(|cents| {
                        if cents == 0 {
                            match lang {
                                Language::TR => "Ücretsiz Oynanabilir".to_string(),
                                Language::EN => "Free to Play".to_string(),
                            }
                        } else {
                            let currency = entry
                                .get("currency")
                                .and_then(|c| c.as_str())
                                .unwrap_or("USD");
                            format!("{:.2} {}", cents as f64 / 100.0, currency)
                        }
                    });

                items.push(CachedFeaturedItem {
                    app_id,
                    title: name.to_string(),
                    header_url,
                    capsule_url,
                    discount_percent,
                    final_price_str,
                });

                if items.len() >= 12 {
                    break;
                }
            }
        }
        if items.len() >= 12 {
            break;
        }
    }

    if items.is_empty() {
        return None;
    }

    let cache_root = gamezone_cache_root();
    let _ = std::fs::create_dir_all(&cache_root);
    if let Ok(json) = serde_json::to_string_pretty(&items) {
        let _ = std::fs::write(cache_root.join("featured_cache.json"), json);
    }

    // Seed each item's known Akamai URLs so cover download always has exact hashed fallback URLs
    for it in &items {
        let mdir = media_cache_dir_for(&format!("steam-{}", it.app_id));
        let _ = std::fs::create_dir_all(&mdir);
        if let Ok(ujson) = serde_json::to_string(it) {
            let _ = std::fs::write(mdir.join("featured_urls.json"), ujson);
        }
    }

    Some(featured_items_to_entries(&items, lang, installed_ids))
}

fn featured_items_to_entries(
    items: &[CachedFeaturedItem],
    lang: Language,
    installed_ids: &HashSet<String>,
) -> Vec<GameEntry> {
    items
        .iter()
        .map(|it| {
            let game_id = format!("steam-{}", it.app_id);
            let mdir = media_cache_dir_for(&game_id);
            let is_inst = installed_ids.contains(&it.app_id)
                || crate::gamezone_gui::is_game_installed(
                    Some(&it.app_id),
                    &format!("steam steam://rungameid/{}", it.app_id),
                    &game_id,
                );

            let category = if it.discount_percent > 0 {
                match lang {
                    Language::TR => format!("Steam Öne Çıkan • -%{} İndirim", it.discount_percent),
                    Language::EN => format!("Steam Featured • -{}% Off", it.discount_percent),
                }
            } else {
                match lang {
                    Language::TR => "Steam Öne Çıkanlar".to_string(),
                    Language::EN => "Steam Featured".to_string(),
                }
            };

            let price_info = it
                .final_price_str
                .clone()
                .unwrap_or_else(|| "Steam Resmi Mağaza".to_string());

            let desc = match lang {
                Language::TR => format!("{} • {} • Resmi Steam Vitrini", it.title, price_info),
                Language::EN => format!("{} • {} • Official Steam Showcase", it.title, price_info),
            };

            let cover_p = mdir.join("cover.jpg");
            let hero_p = mdir.join("hero.jpg");
            let ss_p = mdir.join("screenshot_0.jpg");

            GameEntry {
                id: game_id,
                title: it.title.clone(),
                category,
                exec: format!("steam steam://rungameid/{}", it.app_id),
                banner_desc: desc,
                is_steam: true,
                is_installed: is_inst,
                steam_app_id: Some(it.app_id.clone()),
                cover_path: if is_valid_image_file(&cover_p) {
                    Some(cover_p.to_string_lossy().to_string())
                } else {
                    None
                },
                hero_path: if is_valid_image_file(&hero_p) {
                    Some(hero_p.to_string_lossy().to_string())
                } else {
                    None
                },
                screenshot_path: if is_valid_image_file(&ss_p) {
                    Some(ss_p.to_string_lossy().to_string())
                } else {
                    None
                },
                store_url: Some(format!("https://store.steampowered.com/app/{}", it.app_id)),
                movie_url: None,
            }
        })
        .collect()
}

/// Resolves a Steam AppID by searching the official Steam StoreSearch API for non-Steam or
/// externally downloaded games (`~/Games/...`, Heroic, Lutris, `.exe`, `.AppImage`).
pub fn resolve_steam_appid_by_title(title: &str) -> Option<String> {
    let clean: String = title
        .replace(['_', '-', '.'], " ")
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join(" ");
    if clean.len() < 2 {
        return None;
    }
    let cache_map_dir = gamezone_cache_root().join("resolved_ids");
    let _ = std::fs::create_dir_all(&cache_map_dir);
    let cache_key_path = cache_map_dir.join(format!("{}.txt", sanitize_key(&clean.to_lowercase())));
    if let Ok(saved) = std::fs::read_to_string(&cache_key_path) {
        let t = saved.trim();
        if !t.is_empty() && t != "NONE" {
            return Some(t.to_string());
        } else if t == "NONE" {
            return None;
        }
    }

    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&l=turkish&cc=tr",
        url_encode(&clean)
    );
    let val = curl_json(&url, 5)?;
    if let Some(first) = val
        .get("items")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
    {
        if let Some(id_num) = first.get("id").and_then(|i| i.as_i64()) {
            let s = id_num.to_string();
            let _ = std::fs::write(&cache_key_path, &s);
            return Some(s);
        }
    }
    let _ = std::fs::write(&cache_key_path, "NONE");
    None
}

/// Downloads official cover, transparent logo, hero banner, store screenshots, and trailer stream URL
/// for any game on parallel background worker threads and streams ready notifications back to GTK UI.
pub fn spawn_official_media_fetcher(
    games: Vec<GameEntry>,
    tx: async_channel::Sender<MediaUpdateEvent>,
) {
    let shared_games = Arc::new(games);
    let next_idx = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let worker_count = 4usize;

    for _ in 0..worker_count {
        let games_ref = Arc::clone(&shared_games);
        let idx_ref = Arc::clone(&next_idx);
        let tx_worker = tx.clone();

        std::thread::spawn(move || {
            loop {
                let i = idx_ref.fetch_add(1, Ordering::SeqCst);
                if i >= games_ref.len() {
                    break;
                }
                let game = &games_ref[i];

                // Skip synthetic launchers that aren't actual games
                if game.id == "steam-deck" || game.id == "xbox-cloud" {
                    continue;
                }

                let app_id_opt = game
                    .steam_app_id
                    .clone()
                    .or_else(|| resolve_steam_appid_by_title(&game.title));

                let Some(app_id) = app_id_opt else {
                    continue;
                };

                let mdir = media_cache_dir_for(&game.id);
                let steam_mdir = media_cache_dir_for(&format!("steam-{}", app_id));
                let _ = std::fs::create_dir_all(&mdir);
                let _ = std::fs::create_dir_all(&steam_mdir);

                // Check if we saved exact hashed Akamai URLs from Featured API
                let mut feat_capsule = String::new();
                let mut feat_header = String::new();
                if let Ok(ftxt) = std::fs::read_to_string(steam_mdir.join("featured_urls.json")) {
                    if let Ok(fit) = serde_json::from_str::<CachedFeaturedItem>(&ftxt) {
                        feat_capsule = fit.capsule_url;
                        feat_header = fit.header_url;
                    }
                }

                // 1. Official Cover Artwork (prefer crisp 600x900 vertical box art or 616x353 capsule)
                let cover_dest = mdir.join("cover.jpg");
                if !is_valid_image_file(&cover_dest) {
                    let candidates = [
                        format!(
                            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{}/library_600x900.jpg",
                            app_id
                        ),
                        feat_capsule.clone(),
                        feat_header.clone(),
                        format!(
                            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{}/header.jpg",
                            app_id
                        ),
                    ];
                    for c_url in &candidates {
                        if download_url_verified(c_url, &cover_dest, 6) {
                            break;
                        }
                    }
                }
                if is_valid_image_file(&cover_dest) {
                    let _ = tx_worker.send_blocking(MediaUpdateEvent::CoverReady {
                        game_id: game.id.clone(),
                        path: cover_dest.clone(),
                    });
                }

                // 2. Official Transparent Game Logo (`logo.png`)
                let logo_dest = mdir.join("logo.png");
                let logo_url = format!(
                    "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{}/logo.png",
                    app_id
                );
                if download_url_verified(&logo_url, &logo_dest, 5) {
                    let _ = tx_worker.send_blocking(MediaUpdateEvent::LogoReady {
                        game_id: game.id.clone(),
                        path: logo_dest,
                    });
                }

                // 3. Official 1920x620 Library Hero Banner
                let hero_dest = mdir.join("hero.jpg");
                let hero_candidates = [
                    format!(
                        "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{}/library_hero.jpg",
                        app_id
                    ),
                    feat_capsule,
                    feat_header,
                ];
                if !is_valid_image_file(&hero_dest) {
                    for h_url in &hero_candidates {
                        if download_url_verified(h_url, &hero_dest, 6) {
                            break;
                        }
                    }
                }
                if is_valid_image_file(&hero_dest) {
                    let _ = tx_worker.send_blocking(MediaUpdateEvent::HeroReady {
                        game_id: game.id.clone(),
                        path: hero_dest,
                    });
                }
            }
        });
    }
}

/// Fetches detailed store screenshots + official gameplay trailer URL (`hls_h264` / `mp4` / `webm`)
/// on demand when a game is focused/selected, caching metadata locally for instant reuse.
pub fn fetch_game_details_and_trailer_async(
    game: GameEntry,
    tx: async_channel::Sender<MediaUpdateEvent>,
) {
    std::thread::spawn(move || {
        if game.id == "steam-deck" || game.id == "xbox-cloud" {
            return;
        }
        let Some(app_id) = game
            .steam_app_id
            .clone()
            .or_else(|| resolve_steam_appid_by_title(&game.title))
        else {
            return;
        };

        let mdir = media_cache_dir_for(&game.id);
        let _ = std::fs::create_dir_all(&mdir);
        let meta_path = mdir.join("trailer_meta.json");

        let meta: TrailerMetadata = if let Ok(saved) = std::fs::read_to_string(&meta_path) {
            serde_json::from_str(&saved).unwrap_or_default()
        } else {
            let api_url = format!(
                "https://store.steampowered.com/api/appdetails?appids={}&l=turkish",
                app_id
            );
            let mut extracted = TrailerMetadata::default();
            if let Some(val) = curl_json(&api_url, 6) {
                if let Some(data) = val.get(&app_id).and_then(|v| v.get("data")) {
                    extracted.title = data
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&game.title)
                        .to_string();
                    extracted.short_description = data
                        .get("short_description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .replace("&quot;", "\"")
                        .replace("&amp;", "&");

                    if let Some(ss_arr) = data.get("screenshots").and_then(|v| v.as_array()) {
                        for s in ss_arr.iter().take(3) {
                            if let Some(u) = s
                                .get("path_full")
                                .or_else(|| s.get("path_thumbnail"))
                                .and_then(|v| v.as_str())
                            {
                                extracted.screenshot_urls.push(u.to_string());
                            }
                        }
                    }

                    if let Some(movies) = data.get("movies").and_then(|v| v.as_array()) {
                        if let Some(m0) = movies.first() {
                            extracted.thumbnail_url = m0
                                .get("thumbnail")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            // Support 2026 Steam HLS (`hls_h264`) as well as classic `mp4.480` / `webm.480`
                            let hls = m0
                                .get("hls_h264")
                                .and_then(|v| v.as_str())
                                .map(resolve_fast_hls_variant);
                            let mp4 = m0
                                .get("mp4")
                                .and_then(|v| v.get("480").or_else(|| v.get("max")))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let webm = m0
                                .get("webm")
                                .and_then(|v| v.get("480").or_else(|| v.get("max")))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            extracted.stream_url = mp4.or(hls).or(webm);
                        }
                    }
                }
            }
            if !extracted.screenshot_urls.is_empty() || extracted.stream_url.is_some() {
                if let Ok(j) = serde_json::to_string_pretty(&extracted) {
                    let _ = std::fs::write(&meta_path, j);
                }
            }
            extracted
        };

        let mut ss_paths = Vec::new();
        if let Some(ref thumb) = meta.thumbnail_url {
            let tp = mdir.join("trailer_thumb.jpg");
            if download_url_verified(thumb, &tp, 5) {
                ss_paths.push(tp);
            }
        }
        for (idx, u) in meta.screenshot_urls.iter().take(3).enumerate() {
            let sp = mdir.join(format!("screenshot_{}.jpg", idx));
            if download_url_verified(u, &sp, 6) {
                ss_paths.push(sp);
            }
        }

        let _ = tx.send_blocking(MediaUpdateEvent::DetailsReady {
            game_id: game.id.clone(),
            app_id,
            meta,
            screenshot_paths: ss_paths,
        });
    });
}

/// Converts a multi-variant Steam `hls_264_master.m3u8` URL into the direct 480p/360p video
/// variant playlist (`hls_264_2_video.m3u8`) so `ffmpeg` starts streaming in ~1.0s without
/// probing 1080p/1440p high-bitrate variants.
pub fn resolve_fast_hls_variant(master_url: &str) -> String {
    if !master_url.contains("hls_264_master.m3u8") {
        return master_url.to_string();
    }
    if let Ok(out) = Command::new("curl")
        .args(["-fsSL", "-m", "4", master_url])
        .output()
    {
        if out.status.success() {
            let txt = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = txt.lines().map(|l| l.trim()).collect();
            let mut chosen: Option<&str> = None;
            for i in 0..lines.len() {
                if lines[i].starts_with("#EXT-X-STREAM-INF")
                    && (lines[i].contains("854x480") || lines[i].contains("640x360"))
                {
                    if let Some(next_uri) = lines.get(i + 1).copied() {
                        if !next_uri.is_empty() && !next_uri.starts_with('#') {
                            chosen = Some(next_uri);
                            break;
                        }
                    }
                }
            }
            if chosen.is_none() {
                chosen = lines
                    .iter()
                    .copied()
                    .find(|l| !l.is_empty() && !l.starts_with('#') && l.contains("video"));
            }
            if let Some(sub) = chosen {
                if sub.starts_with("http") {
                    return sub.to_string();
                }
                if let Some((base, tail)) = master_url.split_once("hls_264_master.m3u8") {
                    return format!("{}{}{}", base, sub, tail);
                }
            }
        }
    }
    master_url.replace("hls_264_master.m3u8", "hls_264_2_video.m3u8")
}

// ==============================================================================
// Live In-Widget Video Trailer & Screenshot Preview Controller
// ==============================================================================

#[derive(Clone)]
pub struct LivePreviewController {
    generation: Arc<AtomicU64>,
    active_child: Arc<Mutex<Option<Child>>>,
}

impl LivePreviewController {
    pub fn new() -> Self {
        Self {
            generation: Arc::new(AtomicU64::new(0)),
            active_child: Arc::new(Mutex::new(None)),
        }
    }

    pub fn stop(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut guard) = self.active_child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    pub fn current_generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Starts streaming live video frames (`hls_h264` / `mp4` / `webm`) via `ffmpeg` MJPEG pipe
    /// directly into `target_pic` at 15 FPS, or cycles official HD store screenshots if `ffmpeg`
    /// or stream URL is unavailable.
    pub fn start_video_or_slideshow(
        &self,
        stream_url: Option<String>,
        screenshots: Vec<PathBuf>,
        target_pic: Picture,
        status_badge: Label,
    ) {
        self.stop();
        let gen_token = self.generation.fetch_add(1, Ordering::SeqCst) + 1;

        // Show first screenshot immediately so preview box is never blank
        if let Some(first) = screenshots.first() {
            load_texture_into_picture(&target_pic, first);
        }

        let has_ffmpeg = Path::new("/usr/bin/ffmpeg").exists()
            || Command::new("which")
                .arg("ffmpeg")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

        if let Some(url) = stream_url.filter(|u| !u.is_empty() && has_ffmpeg) {
            status_badge.set_text("▶ CANLI FRAGMAN");
            let (frame_tx, frame_rx) = async_channel::bounded::<Vec<u8>>(2);
            let gen_ui = self.generation.clone();
            let pic_ui = target_pic.clone();

            glib::spawn_future_local(async move {
                while let Ok(jpeg_bytes) = frame_rx.recv().await {
                    if gen_ui.load(Ordering::SeqCst) != gen_token {
                        break;
                    }
                    let gbytes = glib::Bytes::from(&jpeg_bytes);
                    if let Ok(tex) = gdk::Texture::from_bytes(&gbytes) {
                        pic_ui.set_paintable(Some(&tex));
                    }
                }
            });

            let gen_bg = self.generation.clone();
            let child_slot = self.active_child.clone();
            std::thread::spawn(move || {
                let spawned = Command::new("ffmpeg")
                    .args([
                        "-nostdin",
                        "-loglevel",
                        "quiet",
                        "-reconnect",
                        "1",
                        "-reconnect_streamed",
                        "1",
                        "-reconnect_delay_max",
                        "2",
                        "-i",
                        &url,
                        "-t",
                        "28",
                        "-an",
                        "-vf",
                        "scale=480:-2,fps=15",
                        "-f",
                        "image2pipe",
                        "-vcodec",
                        "mjpeg",
                        "-q:v",
                        "5",
                        "-",
                    ])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn();

                let Ok(mut child) = spawned else {
                    return;
                };
                let Some(mut stdout) = child.stdout.take() else {
                    let _ = child.kill();
                    return;
                };

                if let Ok(mut guard) = child_slot.lock() {
                    *guard = Some(child);
                }

                let mut buf = [0u8; 16384];
                let mut accum: Vec<u8> = Vec::with_capacity(65536);

                loop {
                    if gen_bg.load(Ordering::SeqCst) != gen_token {
                        break;
                    }
                    let Ok(n) = stdout.read(&mut buf) else {
                        break;
                    };
                    if n == 0 {
                        break;
                    }
                    accum.extend_from_slice(&buf[..n]);

                    // Extract complete JPEG frames bounded by SOI (FF D8) and EOI (FF D9)
                    while let Some(soi) = find_marker(&accum, 0xD8) {
                        if soi > 0 {
                            accum.drain(..soi);
                        }
                        if let Some(eoi) = find_marker(&accum[2..], 0xD9) {
                            let end = 2 + eoi + 2;
                            let frame: Vec<u8> = accum.drain(..end).collect();
                            let _ = frame_tx.try_send(frame);
                        } else {
                            break;
                        }
                    }

                    if accum.len() > 1_500_000 {
                        accum.clear();
                    }
                }

                if let Ok(mut guard) = child_slot.lock() {
                    if let Some(mut c) = guard.take() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                }
            });
        } else if screenshots.len() > 1 {
            status_badge.set_text("◉ MAĞAZA ÖN İZLEME");
            let gen_ss = self.generation.clone();
            let pic_ss = target_pic.clone();
            let mut idx = 0usize;
            glib::timeout_add_local(Duration::from_millis(2400), move || {
                if gen_ss.load(Ordering::SeqCst) != gen_token {
                    return glib::ControlFlow::Break;
                }
                idx = (idx + 1) % screenshots.len();
                load_texture_into_picture(&pic_ss, &screenshots[idx]);
                glib::ControlFlow::Continue
            });
        } else {
            status_badge.set_text("◉ RESMİ ÖN İZLEME");
        }
    }
}

fn find_marker(data: &[u8], second_byte: u8) -> Option<usize> {
    data.windows(2)
        .position(|w| w[0] == 0xFF && w[1] == second_byte)
}

pub fn load_texture_into_picture(pic: &Picture, path: &Path) {
    if let Ok(bytes) = std::fs::read(path) {
        let gbytes = glib::Bytes::from(&bytes);
        if let Ok(tex) = gdk::Texture::from_bytes(&gbytes) {
            pic.set_paintable(Some(&tex));
        }
    }
}

// ==============================================================================
// Multi-Source Computer & Linked Account Game Scanner
// ==============================================================================

#[derive(Clone, Debug)]
pub struct AccountSummary {
    pub steam_persona: Option<String>,
    pub steam_avatar_path: Option<PathBuf>,
    pub epic_display_name: Option<String>,
}

pub fn detect_accounts_with_avatar() -> AccountSummary {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/blaze".to_string());
    let steam_roots = [
        PathBuf::from(&home).join(".steam/steam"),
        PathBuf::from(&home).join(".local/share/Steam"),
        PathBuf::from(&home).join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ];

    let mut steam_persona = None;
    let mut steam_id64 = None;
    let mut steam_avatar_path = None;

    for root in &steam_roots {
        let login_vdf = root.join("config/loginusers.vdf");
        if let Ok(txt) = std::fs::read_to_string(&login_vdf) {
            for line in txt.lines() {
                let t = line.trim();
                if t.starts_with("\"7656") && t.ends_with('"') && steam_id64.is_none() {
                    steam_id64 = Some(t.trim_matches('"').to_string());
                } else if t.starts_with("\"PersonaName\"") {
                    let parts: Vec<&str> =
                        t.split('"').filter(|s| !s.trim().is_empty()).collect();
                    if parts.len() >= 2 {
                        steam_persona = Some(parts[1].to_string());
                        break;
                    }
                }
            }
        }
        if let Some(ref sid) = steam_id64 {
            let av = root.join("config/avatarcache").join(format!("{}.png", sid));
            if is_valid_image_file(&av) {
                steam_avatar_path = Some(av);
            }
        }
        if steam_persona.is_some() {
            break;
        }
    }

    let mut epic_display_name = None;
    for ep_user in [
        PathBuf::from(&home).join(".config/legendary/user.json"),
        PathBuf::from(&home).join(".config/heroic/legendaryConfig/legendary/user.json"),
        PathBuf::from(&home)
            .join(".var/app/com.heroicgameslauncher.hgl/config/heroic/legendaryConfig/legendary/user.json"),
    ] {
        if let Ok(txt) = std::fs::read_to_string(&ep_user) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                if let Some(n) = val.get("displayName").and_then(|v| v.as_str()) {
                    epic_display_name = Some(n.to_string());
                    break;
                }
            }
        }
    }

    AccountSummary {
        steam_persona,
        steam_avatar_path,
        epic_display_name,
    }
}

/// Scans ALL games installed on the PC (across Steam multi-drive libraries, Heroic/Epic/GOG/Amazon,
/// Lutris, Flatpak, system `.desktop` games, AND standalone downloaded folders/executables in
/// `~/Games`, `~/Oyunlar`, `~/Downloads`, `~/İndirilenler`, `/opt/games`, and custom user paths)
/// plus games associated with signed-in accounts!
pub fn scan_computer_and_account_games(base_games: Vec<GameEntry>) -> Vec<GameEntry> {
    let mut results: Vec<GameEntry> = Vec::new();
    let mut seen_titles: HashSet<String> = HashSet::new();
    let mut seen_appids: HashSet<String> = HashSet::new();

    for g in base_games {
        // Keep real games (skip generic launcher cards if we want pure games, or keep launchers at end)
        if g.id == "steam-deck" {
            continue;
        }
        if let Some(ref aid) = g.steam_app_id {
            seen_appids.insert(aid.clone());
        }
        seen_titles.insert(g.title.to_lowercase());
        results.push(g);
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/blaze".to_string());

    // 1. Multi-Drive Steam `libraryfolders.vdf` scanner (finds games on secondary NVMe/SSD/HDDs!)
    let mut steam_apps_dirs = vec![
        PathBuf::from(&home).join(".steam/steam/steamapps"),
        PathBuf::from(&home).join(".local/share/Steam/steamapps"),
        PathBuf::from(&home).join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps"),
    ];
    for sdir in steam_apps_dirs.clone() {
        let lf = sdir.join("libraryfolders.vdf");
        if let Ok(txt) = std::fs::read_to_string(&lf) {
            for line in txt.lines() {
                let t = line.trim();
                if t.starts_with("\"path\"") {
                    let parts: Vec<&str> =
                        t.split('"').filter(|s| !s.trim().is_empty()).collect();
                    if parts.len() >= 2 {
                        let extra = PathBuf::from(parts[1].replace("\\\\", "/")).join("steamapps");
                        if !steam_apps_dirs.contains(&extra) {
                            steam_apps_dirs.push(extra);
                        }
                    }
                }
            }
        }
    }

    for sdir in &steam_apps_dirs {
        let Ok(entries) = std::fs::read_dir(sdir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            let Some(fname) = p.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if fname.starts_with("appmanifest_") && fname.ends_with(".acf") {
                if let Ok(txt) = std::fs::read_to_string(&p) {
                    if let Some((aid, title)) = parse_steam_acf(&txt) {
                        if seen_appids.insert(aid.clone())
                            && seen_titles.insert(title.to_lowercase())
                        {
                            let gid = format!("steam-{}", aid);
                            let mdir = media_cache_dir_for(&gid);
                            results.push(GameEntry {
                                id: gid,
                                title: title.clone(),
                                category: "Steam • Bilgisayarda Kurulu".to_string(),
                                exec: format!("steam steam://rungameid/{}", aid),
                                banner_desc: format!("{} • Yerel Steam Kütüphanesi (Kurulu)", title),
                                is_steam: true,
                                is_installed: true,
                                steam_app_id: Some(aid.clone()),
                                cover_path: valid_path_opt(&mdir.join("cover.jpg")),
                                hero_path: valid_path_opt(&mdir.join("hero.jpg")),
                                screenshot_path: valid_path_opt(&mdir.join("screenshot_0.jpg")),
                                store_url: Some(format!(
                                    "https://store.steampowered.com/app/{}",
                                    aid
                                )),
                                movie_url: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. Heroic Sideloaded & Epic/Amazon/GOG Libraries (including Flatpak Heroic)
    let heroic_sideloads = [
        PathBuf::from(&home).join(".config/heroic/sideload_apps/library.json"),
        PathBuf::from(&home)
            .join(".var/app/com.heroicgameslauncher.hgl/config/heroic/sideload_apps/library.json"),
    ];
    for sl in &heroic_sideloads {
        if let Ok(txt) = std::fs::read_to_string(sl) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                if let Some(arr) = val.get("games").and_then(|v| v.as_array()) {
                    for item in arr {
                        let Some(title) = item.get("title").and_then(|v| v.as_str()) else {
                            continue;
                        };
                        let app_name = item
                            .get("app_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(title);
                        if seen_titles.insert(title.to_lowercase()) {
                            let gid = format!("heroic-{}", sanitize_key(app_name));
                            let mdir = media_cache_dir_for(&gid);
                            results.push(GameEntry {
                                id: gid,
                                title: title.to_string(),
                                category: "Heroic • Harici Oyun".to_string(),
                                exec: format!("heroic --launch heroic://launch/{}", app_name),
                                banner_desc: format!("{} • Heroic Kütüphanesi", title),
                                is_steam: false,
                                is_installed: true,
                                steam_app_id: None,
                                cover_path: valid_path_opt(&mdir.join("cover.jpg")),
                                hero_path: valid_path_opt(&mdir.join("hero.jpg")),
                                screenshot_path: valid_path_opt(&mdir.join("screenshot_0.jpg")),
                                store_url: None,
                                movie_url: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // 3. Lutris YAML Game Configurations (`~/.config/lutris/games/*.yml`)
    for lutris_dir in [
        PathBuf::from(&home).join(".config/lutris/games"),
        PathBuf::from(&home).join(".var/app/net.lutris.Lutris/config/lutris/games"),
    ] {
        if let Ok(entries) = std::fs::read_dir(&lutris_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("yml") {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        // Strip trailing timestamp slug e.g. `hollow-knight-17123456`
                        let clean_slug = stem
                            .rsplit_once('-')
                            .filter(|(_, tail)| tail.chars().all(|c| c.is_ascii_digit()))
                            .map(|(head, _)| head)
                            .unwrap_or(stem);
                        let pretty = prettify_slug(clean_slug);
                        if !pretty.is_empty() && seen_titles.insert(pretty.to_lowercase()) {
                            let gid = format!("lutris-{}", sanitize_key(clean_slug));
                            let mdir = media_cache_dir_for(&gid);
                            results.push(GameEntry {
                                id: gid,
                                title: pretty.clone(),
                                category: "Lutris • Yerel Kurulum".to_string(),
                                exec: format!("lutris lutris:rungame/{}", clean_slug),
                                banner_desc: format!("{} • Lutris / Wine Oyunu", pretty),
                                is_steam: false,
                                is_installed: true,
                                steam_app_id: None,
                                cover_path: valid_path_opt(&mdir.join("cover.jpg")),
                                hero_path: valid_path_opt(&mdir.join("hero.jpg")),
                                screenshot_path: valid_path_opt(&mdir.join("screenshot_0.jpg")),
                                store_url: None,
                                movie_url: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // 4. Flatpak Exported Games (`/var/lib/flatpak/exports/share/applications`, `~/.local/share/flatpak/...`)
    for fp_apps in [
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from(&home).join(".local/share/flatpak/exports/share/applications"),
        PathBuf::from(&home).join(".local/share/applications"),
    ] {
        scan_desktop_game_dir(&fp_apps, &mut results, &mut seen_titles);
    }

    // 5. Externally Downloaded / Standalone / Portable Game Folders & Custom User Sources!
    let custom = load_custom_sources();
    for custom_exe in &custom.executables {
        let exe_path = Path::new(&custom_exe.path);
        if exe_path.exists() && seen_titles.insert(custom_exe.title.to_lowercase()) {
            let gid = format!("custom-{}", sanitize_key(&custom_exe.title.to_lowercase()));
            let mdir = media_cache_dir_for(&gid);
            results.push(GameEntry {
                id: gid,
                title: custom_exe.title.clone(),
                category: "Harici Kaynak • Bilgisayarda".to_string(),
                exec: build_launch_command_for_binary(exe_path),
                banner_desc: format!("{} • {}", custom_exe.title, exe_path.display()),
                is_steam: false,
                is_installed: true,
                steam_app_id: None,
                cover_path: valid_path_opt(&mdir.join("cover.jpg")),
                hero_path: valid_path_opt(&mdir.join("hero.jpg")),
                screenshot_path: valid_path_opt(&mdir.join("screenshot_0.jpg")),
                store_url: None,
                movie_url: None,
            });
        }
    }

    let mut scan_roots = vec![
        PathBuf::from(&home).join("Games"),
        PathBuf::from(&home).join("Oyunlar"),
        PathBuf::from("/opt/games"),
        PathBuf::from(&home).join("Downloads/Games"),
        PathBuf::from(&home).join("İndirilenler/Oyunlar"),
    ];
    for cd in &custom.directories {
        let pb = PathBuf::from(cd);
        if !scan_roots.contains(&pb) {
            scan_roots.push(pb);
        }
    }

    for root in &scan_roots {
        scan_standalone_game_folder(root, 0, &mut results, &mut seen_titles);
    }

    results
}

fn valid_path_opt(p: &Path) -> Option<String> {
    if is_valid_image_file(p) {
        Some(p.to_string_lossy().to_string())
    } else {
        None
    }
}

fn parse_steam_acf(content: &str) -> Option<(String, String)> {
    let mut app_id = None;
    let mut name = None;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("\"appid\"") {
            let parts: Vec<&str> = t.split('"').filter(|s| !s.trim().is_empty()).collect();
            if parts.len() >= 2 {
                app_id = Some(parts[1].to_string());
            }
        } else if t.starts_with("\"name\"") {
            let parts: Vec<&str> = t.split('"').filter(|s| !s.trim().is_empty()).collect();
            if parts.len() >= 2 {
                name = Some(parts[1].to_string());
            }
        }
    }
    if let (Some(id), Some(n)) = (app_id, name) {
        if id != "228980"
            && !n.contains("Steam Linux Runtime")
            && !n.contains("Proton")
            && !n.contains("Steamworks")
        {
            return Some((id, n));
        }
    }
    None
}

fn prettify_slug(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn scan_desktop_game_dir(
    dir: &Path,
    results: &mut Vec<GameEntry>,
    seen_titles: &mut HashSet<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("desktop") {
            continue;
        }
        let Ok(txt) = std::fs::read_to_string(&p) else {
            continue;
        };
        if !txt.contains("Categories=") || !txt.contains("Game") {
            continue;
        }
        let mut name = String::new();
        let mut exec = String::new();
        for line in txt.lines() {
            if line.starts_with("Name=") && name.is_empty() {
                name = line[5..].trim().to_string();
            } else if line.starts_with("Exec=") && exec.is_empty() {
                exec = line[5..]
                    .replace("%u", "")
                    .replace("%U", "")
                    .replace("%f", "")
                    .replace("%F", "")
                    .trim()
                    .to_string();
            }
        }
        let lower = name.to_lowercase();
        if name.is_empty()
            || exec.is_empty()
            || lower.contains("gamezone")
            || lower == "steam"
            || lower.contains("lutris")
            || lower.contains("heroic")
            || !seen_titles.insert(lower)
        {
            continue;
        }
        let gid = format!("desktop-{}", sanitize_key(&name.to_lowercase()));
        let mdir = media_cache_dir_for(&gid);
        results.push(GameEntry {
            id: gid,
            title: name.clone(),
            category: "Yerel / Flatpak Oyunu".to_string(),
            exec,
            banner_desc: format!("{} • Bilgisayarda Kurulu", name),
            is_steam: false,
            is_installed: true,
            steam_app_id: None,
            cover_path: valid_path_opt(&mdir.join("cover.jpg")),
            hero_path: valid_path_opt(&mdir.join("hero.jpg")),
            screenshot_path: valid_path_opt(&mdir.join("screenshot_0.jpg")),
            store_url: None,
            movie_url: None,
        });
    }
}

fn is_ignored_helper_binary(filename_lower: &str) -> bool {
    const IGNORED: &[&str] = &[
        "unins",
        "crashhandler",
        "unitycrash",
        "vc_redist",
        "vcredist",
        "dxsetup",
        "dotnet",
        "prereq",
        "ue4prereq",
        "ueprereq",
        "easyanticheat",
        "battleye",
        "benchmark",
        "config.exe",
        "updater",
        "patcher",
    ];
    IGNORED.iter().any(|pat| filename_lower.contains(pat))
}

pub fn build_launch_command_for_binary(bin_path: &Path) -> String {
    let ext = bin_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let p_str = bin_path.to_string_lossy();
    if ext == "exe" {
        if Path::new("/usr/bin/umu-run").exists() {
            format!("umu-run {}", p_str)
        } else {
            format!("wine {}", p_str)
        }
    } else {
        p_str.to_string()
    }
}

/// Scans standalone game directories (e.g. `~/Games/Celeste`, `~/Oyunlar/Hollow Knight`, `.AppImage`, `.exe`)
/// up to depth 2 so games downloaded from itch.io, GOG offline installers, or portable archives are
/// automatically detected and added to "Bilgisayarımdaki Oyunlar".
pub fn scan_standalone_game_folder(
    dir: &Path,
    depth: usize,
    results: &mut Vec<GameEntry>,
    seen_titles: &mut HashSet<String>,
) {
    if depth > 2 || !dir.exists() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    let mut subdirs = Vec::new();
    let mut candidate_bin: Option<PathBuf> = None;

    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            subdirs.push(p);
            continue;
        }
        let Some(fname) = p.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let fl = fname.to_lowercase();
        if is_ignored_helper_binary(&fl) {
            continue;
        }
        if fl.ends_with(".appimage") || fl.ends_with(".x86_64") || fl.ends_with(".exe") || fl == "start.sh" {
            // Prefer top-level game executable inside a game folder
            if depth >= 1 || fl.ends_with(".appimage") {
                candidate_bin = Some(p);
                break;
            }
        }
    }

    if let Some(bin) = candidate_bin {
        let raw_title = if depth >= 1 {
            dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Harici Oyun")
        } else {
            bin.file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("Harici Oyun")
        };
        let title = prettify_slug(raw_title);
        if !title.is_empty() && seen_titles.insert(title.to_lowercase()) {
            let gid = format!("external-{}", sanitize_key(&title.to_lowercase()));
            let mdir = media_cache_dir_for(&gid);
            let is_win = bin
                .extension()
                .and_then(|x| x.to_str())
                .map(|s| s.eq_ignore_ascii_case("exe"))
                .unwrap_or(false);
            let cat = if is_win {
                "Harici İndirilen • Proton/Wine"
            } else {
                "Harici İndirilen • Yerel Linux"
            };
            results.push(GameEntry {
                id: gid,
                title: title.clone(),
                category: cat.to_string(),
                exec: build_launch_command_for_binary(&bin),
                banner_desc: format!("{} • Diskten Otomatik Algılandı ({})", title, bin.display()),
                is_steam: false,
                is_installed: true,
                steam_app_id: None,
                cover_path: valid_path_opt(&mdir.join("cover.jpg")),
                hero_path: valid_path_opt(&mdir.join("hero.jpg")),
                screenshot_path: valid_path_opt(&mdir.join("screenshot_0.jpg")),
                store_url: None,
                movie_url: None,
            });
            return; // Found executable for this game folder; don't duplicate inner subfolders
        }
    }

    for sd in subdirs {
        scan_standalone_game_folder(&sd, depth + 1, results, seen_titles);
    }
}

pub fn is_featured_cache_fresh() -> bool {
    let p = gamezone_cache_root().join("featured_cache.json");
    let Ok(meta) = std::fs::metadata(&p) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .map(|d| d < Duration::from_secs(7200))
        .unwrap_or(false)
}
