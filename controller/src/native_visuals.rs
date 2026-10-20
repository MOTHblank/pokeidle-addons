//! Shared, lazy native sprite renderer. Item and ball artwork is bundled;
//! unknown items and Pokémon retain the on-demand disk cache. Unknown art
//! can be fetched once into a per-user disk cache on a dedicated worker.
//! No image decoding, HTTP, or filesystem reads happen on ordinary UI frames.
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::mpsc::{self, Receiver, Sender};

mod pokemon_art;

mod generated {
    include!("native_visual_assets.generated.rs");
}
const MAX_IMAGE_BYTES: u64 = 1_048_576;
const MAX_EDGE: u32 = 1024;
// Dynamic Pokémon sprite textures, not bundled item icons, are budgeted.
// Resident dynamic sprites are optional; the original art remains on disk.
// A 2M-pixel RGBA budget is ~8 MiB of CPU-side decoded pixel data, rather
// than the previous 16M-pixel (~64 MiB) allowance.
// Four overview cards show at most two Pokémon each.
const MAX_POKEMON_TEXTURES: usize = 8;
const MAX_POKEMON_TEXTURE_PIXELS: usize = 2 * 1024 * 1024;
const MAX_POKEMON_TEXTURE_EDGE: u32 = 128;

fn pokemon_texture_size(width:u32, height:u32)->(u32,u32) {
    let longest=width.max(height);
    if longest <= MAX_POKEMON_TEXTURE_EDGE || longest==0 {
        return (width,height)
    }
    let scale=MAX_POKEMON_TEXTURE_EDGE as u64;
    let longest=longest as u64;
    (((width as u64*scale)/longest).max(1) as u32,
     ((height as u64*scale)/longest).max(1) as u32)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind { Ball, Item, Pokemon, PokemonLooktype }
impl Kind {
    fn key(self) -> &'static str {
        match self { Self::Ball => "ball", Self::Item => "item", Self::Pokemon => "pokemon", Self::PokemonLooktype => "pokemon-looktype" }
    }
}

struct Request { kind: Kind, id: u64, urls: Vec<String> }
struct Response { key: (Kind, u64), frames: Option<Vec<Vec<u8>>> }

pub struct VisualCache {
    textures: BTreeMap<(Kind, u64), Vec<TextureHandle>>,
    attempted: BTreeMap<(Kind, u64), std::time::Instant>,
    last_used: BTreeMap<(Kind, u64), u64>,
    use_sequence: u64,
    pokemon_pixels: usize,
    texture_pixels: BTreeMap<(Kind, u64), usize>,
    // Track only the artwork actually displayed in the current UI frame.
    // Requested candidates can outnumber visible sprites (shiny/looktype
    // fallback), but are not allowed to remain resident indefinitely.
    pokemon_requested: BTreeSet<(Kind, u64)>,
    pokemon_visible: BTreeSet<(Kind, u64)>,
    request: Sender<Request>,
    results: Receiver<Response>,
}

fn cache_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(base) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(base).join("Moth").join("PokeIdle").join("VisualCache");
        }
    }
    if let Some(base) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(base).join("moth-controller").join("visual");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".cache").join("moth-controller").join("visual");
    }
    std::env::temp_dir().join("moth-controller-visual")
}

// Checked-in copy of assets/items-icons.json from pokeidle-upstream. The
// exported image-index.json contained zero image IDs, so this authoritative
// game item ID -> public art path table supplies the item mappings instead.
fn upstream_item_icons() -> &'static BTreeMap<u64, String> {
    static ICONS: OnceLock<BTreeMap<u64, String>> = OnceLock::new();
    ICONS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/visual/upstream-item-icons.json"))
            .unwrap_or_default()
    })
}

fn upstream_item_url(id: u64) -> Option<String> {
    let raw = upstream_item_icons().get(&id)?;
    // Never use arbitrary item metadata as a request URL: only these public
    // image directory roots from the upstream game snapshot are allowed.
    if raw.is_empty() || raw.contains("..") || raw.contains('\\') ||
        raw.contains('?') || raw.contains('#') ||
        !raw.bytes().all(|byte| byte.is_ascii_alphanumeric() ||
            matches!(byte, b'/' | b'.' | b'_' | b'-')) ||
        !(raw.ends_with(".png") || raw.ends_with(".gif")) { return None; }
    let url = if raw.starts_with("site/assets/") {
        format!("/assets/{raw}")
    } else if raw.starts_with("img/") {
        format!("/{raw}")
    } else {
        return None;
    };
    trusted_url(&url)
}

fn asset_url(kind: Kind, id: u64) -> Option<String> {
    let fixed = match (kind, id) {
        (Kind::Ball, 1) => Some("/assets/site/assets/ui/ball-poke.png"),
        (Kind::Ball, 2) => Some("/assets/site/assets/ui/ball-great.png"),
        (Kind::Ball, 3) => Some("/img/ball-super.png"),
        (Kind::Ball, 4) => Some("/assets/site/assets/ui/ball-ultra.png"),
        (Kind::Ball, 5) => Some("/img/ball-beast.png"),
        (Kind::Item, 70070) => Some("/img/itens/golden-potion.png"),
        _ => None,
    };
    fixed.and_then(trusted_url).or_else(|| match kind {
        Kind::Item => upstream_item_url(id),
        _ => None,
    })
}

fn image_urls(kind: Kind, id: u64, icon_hint: Option<&str>) -> Vec<String> {
    let mut urls = Vec::new();
    for candidate in [icon_hint.and_then(trusted_url), asset_url(kind, id)] {
        let Some(source) = candidate else { continue; };
        if !urls.contains(&source) { urls.push(source.clone()); }
        // The game publishes some healing items from either location.
        const SHORT: &str = "https://pokeidle.io/assets/markitems/";
        const LONG: &str = "https://pokeidle.io/assets/site/assets/markitems/";
        let alternative = if source.starts_with(SHORT) {
            Some(source.replacen(SHORT, LONG, 1))
        } else if source.starts_with(LONG) {
            Some(source.replacen(LONG, SHORT, 1))
        } else { None };
        if let Some(alt) = alternative {
            if !urls.contains(&alt) { urls.push(alt); }
        }

    }
    urls
}

// Reject arbitrary hosts, scheme-relative paths, traversal, and non-image URLs.
// The public items.json may point at unrelated third-party image hosts, which
// are handled only by the explicit developer-side synchronizer.
fn trusted_url(input: &str) -> Option<String> {
    let candidate = input.trim();
    if candidate.is_empty() || candidate.contains("..") ||
        candidate.contains('?') || candidate.contains('#') ||
        candidate.contains('\\') || candidate.chars().any(char::is_control) {
        return None;
    }
    if candidate.starts_with('/') && !candidate.starts_with("//") {
        return Some(format!("https://pokeidle.io{candidate}"));
    }
    if candidate.starts_with("https://pokeidle.io/") {
        return Some(candidate.to_owned());
    }
    None
}

fn valid_image(bytes: &[u8]) -> bool {
    if bytes.len() > MAX_IMAGE_BYTES as usize || bytes.is_empty() { return false; }
    let Ok(img) = image::load_from_memory(bytes) else { return false };
    img.width() > 0 && img.height() > 0 &&
        img.width() <= MAX_EDGE && img.height() <= MAX_EDGE
}

fn load_worker_image(client: &reqwest::blocking::Client, root: &std::path::Path,
                     request: &Request) -> Option<Vec<u8>> {
    let path = root.join(format!("{}-{}.img", request.kind.key(), request.id));
    if let Ok(bytes) = std::fs::read(&path) {
        if valid_image(&bytes) { return Some(bytes); }
        let _ = std::fs::remove_file(&path);
    }
    for url in &request.urls {
        let Ok(response) = client.get(url).send().and_then(|r|r.error_for_status())
            else { continue };
        if response.content_length().is_some_and(|n| n > MAX_IMAGE_BYTES) {
            continue;
        }
        let mut bytes = Vec::new();
        if response.take(MAX_IMAGE_BYTES + 1).read_to_end(&mut bytes).is_err() ||
            !valid_image(&bytes) { continue; }
        // A failure to write the cache must never hide a successfully
        // downloaded texture from this session.
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, &bytes).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
        return Some(bytes);
    }
    None
}

impl Default for VisualCache {
    fn default() -> Self {
        let (request, jobs) = mpsc::channel::<Request>();
        let (output, results) = mpsc::channel::<Response>();
        // Four small workers allow the selected balls, potions and revives
        // to load concurrently. HTTP and decoding still stay off UI frames.
        let jobs = Arc::new(Mutex::new(jobs));
        for worker in 0..4 {
            let jobs = Arc::clone(&jobs);
            let output = output.clone();
            std::thread::Builder::new()
                .name(format!("moth-visual-{worker}"))
                .spawn(move || {
                    let root = cache_dir();
                    let _ = std::fs::create_dir_all(&root);
                    let Ok(client) = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(8))
                        .connect_timeout(std::time::Duration::from_secs(3))
                        .redirect(reqwest::redirect::Policy::none())
                        .build() else { return; };
                    loop {
                        let job = match jobs.lock() {
                            Ok(receiver) => receiver.recv(),
                            Err(_) => break,
                        };
                        let Ok(job) = job else { break };
                        let frames = match job.kind {
                            Kind::Pokemon | Kind::PokemonLooktype =>
                                pokemon_art::load(&client, &root, job.kind, job.id),
                            _ => load_worker_image(&client, &root, &job)
                                .map(|bytes| vec![bytes]),
                        };
                        if frames.is_none() &&
                            matches!(job.kind, Kind::Pokemon | Kind::PokemonLooktype) {
                            crate::logging::warn(&format!(
                                "Pokémon sprite unavailable: kind={:?} id={}; retrying later",
                                job.kind, job.id
                            ));
                        }
                        if output.send(Response {
                            key: (job.kind, job.id), frames,
                        }).is_err() { break; }
                    }
                }).ok();
        }
        Self { textures: BTreeMap::new(), attempted: BTreeMap::new(),
            last_used: BTreeMap::new(), use_sequence: 0, pokemon_pixels: 0,
            texture_pixels: BTreeMap::new(),
            pokemon_requested: BTreeSet::new(), pokemon_visible: BTreeSet::new(),
            request, results }
    }
}

impl VisualCache {
    /// Called after poll() and before drawing the next UI frame.
    pub fn begin_frame(&mut self) {
        self.pokemon_requested.clear();
        self.pokemon_visible.clear();
    }

    /// Keep only the Pokémon textures painted by the Overview in this frame.
    /// This also drops all GPU sprite handles when switching to Hunt Atlas,
    /// Automation or any other page. Bundled item/ball textures are unaffected.
    pub fn end_frame(&mut self) {
        let expired: Vec<_> = self.textures.keys().filter(|key|
            matches!(key.0, Kind::Pokemon | Kind::PokemonLooktype)
                && !self.pokemon_visible.contains(*key)
        ).copied().collect();
        for key in expired {
            self.textures.remove(&key);
            self.last_used.remove(&key);
            self.pokemon_pixels=self.pokemon_pixels.saturating_sub(
                self.texture_pixels.remove(&key).unwrap_or(0));
            // A sprite needed again later can be loaded from the disk cache.
            self.attempted.remove(&key);
        }
        self.trim_pokemon_textures();
    }

    fn touch(&mut self, key: (Kind,u64)) {
        if !matches!(key.0, Kind::Pokemon | Kind::PokemonLooktype) { return; }
        self.use_sequence=self.use_sequence.wrapping_add(1);
        self.last_used.insert(key,self.use_sequence);
    }

    fn trim_pokemon_textures(&mut self) {
        // Dropping TextureHandles releases egui-managed GPU textures. Keep at
        // least one sprite even when an unusually large animation exceeds
        // the pixel budget by itself.
        while self.last_used.len()>1 &&
            (self.last_used.len()>MAX_POKEMON_TEXTURES ||
             self.pokemon_pixels>MAX_POKEMON_TEXTURE_PIXELS) {
            let Some(key)=self.last_used.iter()
                .min_by_key(|(_,sequence)|**sequence).map(|(key,_)|*key)
                else {break};
            self.last_used.remove(&key);
            self.textures.remove(&key);
            self.pokemon_pixels=self.pokemon_pixels.saturating_sub(
                self.texture_pixels.remove(&key).unwrap_or(0));
            // Reopening the species can enqueue its already disk-cached art.
            self.attempted.remove(&key);
        }
    }

    fn put(&mut self, ctx: &egui::Context, key: (Kind, u64), frames: &[Vec<u8>]) {
        // Only one still frame is ever uploaded for a Pokémon or item icon.
        let mut textures = Vec::with_capacity(1);
        let mut pixels = 0usize;
        for (frame, bytes) in frames.iter().take(1).enumerate() {
            let Ok(img) = image::load_from_memory(bytes) else { continue };
            if img.width() == 0 || img.height() == 0 ||
                img.width() > MAX_EDGE || img.height() > MAX_EDGE { continue; }
            let rgba = img.into_rgba8();
            let rgba=if matches!(key.0,Kind::Pokemon|Kind::PokemonLooktype) {
                let (w,h)=pokemon_texture_size(rgba.width(),rgba.height());
                if w!=rgba.width() || h!=rgba.height() {
                    image::imageops::resize(&rgba,w,h,image::imageops::FilterType::Nearest)
                } else {rgba}
            } else {rgba};
            pixels=pixels.saturating_add(rgba.width() as usize * rgba.height() as usize);
            let color_image = ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
            textures.push(ctx.load_texture(
                format!("native-visual-{}-{}-{frame}", key.0.key(), key.1),
                color_image, TextureOptions::NEAREST));
        }
        if !textures.is_empty() {
            self.textures.insert(key, textures);
            if matches!(key.0, Kind::Pokemon | Kind::PokemonLooktype) {
                self.pokemon_pixels=self.pokemon_pixels.saturating_sub(
                    self.texture_pixels.insert(key,pixels).unwrap_or(0));
                self.pokemon_pixels=self.pokemon_pixels.saturating_add(pixels);
                self.touch(key);
                // Only evict after all Overview cards have been painted:
                // mid-frame eviction can invalidate an image already queued
                // for display by egui.
            }
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        if self.attempted.len()>2048 {
            let now=std::time::Instant::now();
            let textures=&self.textures;
            self.attempted.retain(|key,at|
                textures.contains_key(key) ||
                now.saturating_duration_since(*at)<std::time::Duration::from_secs(120));
        }
        while let Ok(reply) = self.results.try_recv() {
            if matches!(reply.key.0, Kind::Pokemon | Kind::PokemonLooktype)
                && !self.pokemon_requested.contains(&reply.key) {
                // The user switched accounts/pages while the sprite worker
                // was reading disk or the CDN. Skip decode and GPU upload.
                self.attempted.remove(&reply.key);
                continue;
            }
            if let Some(frames) = reply.frames {
                self.put(ctx, reply.key, &frames);
                ctx.request_repaint();
            }
        }
    }

    fn ensure(&mut self, ui: &mut egui::Ui, kind: Kind, id: u64,
              icon_hint: Option<&str>) {
        if id == 0 { return; }
        let key = (kind, id);
        // Network failures must not create permanent missing sprites.
        // Attempts are throttled, including failures while the game CDN is
        // unavailable. The same bound applies to all icon families.
        let now = std::time::Instant::now();
        if self.textures.contains_key(&key) ||
            self.attempted.get(&key).is_some_and(|at|
                now.saturating_duration_since(*at) < std::time::Duration::from_secs(120)) {
            return;
        }
        self.attempted.insert(key, now);
        if let Some(bytes) = generated::bytes(kind.key(), id) {
            if valid_image(bytes) { self.put(ui.ctx(), key, &[bytes.to_vec()]); }
        }
        if !self.textures.contains_key(&key) {
            let urls = image_urls(kind, id, icon_hint);
            if !urls.is_empty() || matches!(kind, Kind::Pokemon | Kind::PokemonLooktype) {
                if self.request.send(Request { kind, id, urls }).is_err() {
                    self.attempted.remove(&key);
                }
            }
        }
    }

    /// The simple texture API stays static for settings, balls and items.
    pub fn texture_id(&mut self, ui: &mut egui::Ui, kind: Kind, id: u64,
                      icon_hint: Option<&str>) -> Option<egui::TextureId> {
        if matches!(kind, Kind::Pokemon | Kind::PokemonLooktype) {
            self.pokemon_requested.insert((kind,id));
        }
        self.ensure(ui, kind, id, icon_hint);
        if self.textures.contains_key(&(kind,id)) {self.touch((kind,id));}
        let texture=self.textures.get(&(kind, id))
            .and_then(|frames| frames.first().map(TextureHandle::id));
        if texture.is_some() && matches!(kind, Kind::Pokemon | Kind::PokemonLooktype) {
            self.pokemon_visible.insert((kind,id));
        }
        texture
    }

    /// Always display a static first frame. Pokémon artwork never schedules
    /// animation repaints or uploads additional texture frames.
    pub fn pokemon_texture_id(&mut self, ui: &mut egui::Ui, kind: Kind, id: u64)
        -> Option<egui::TextureId>
    {
        if !matches!(kind, Kind::Pokemon | Kind::PokemonLooktype) { return None; }
        self.pokemon_requested.insert((kind,id));
        self.ensure(ui, kind, id, None);
        if self.textures.contains_key(&(kind,id)) {self.touch((kind,id));}
        let texture=self.textures.get(&(kind,id))
            .and_then(|frames|frames.first().map(TextureHandle::id));
        if texture.is_some() {self.pokemon_visible.insert((kind,id));}
        texture
    }

    /// Returns true if an icon was drawn. Always fall back to text if false.
    /// Pokémon textures are resolved from verified game looktypes and sprite
    /// atlases on the worker, never by guessing an image URL from species ID.
    pub fn show(&mut self, ui: &mut egui::Ui, kind: Kind, id: u64,
                icon_hint: Option<&str>, size: f32) -> bool {
        if let Some(texture) = self.texture_id(ui, kind, id, icon_hint) {
            ui.image((texture, egui::vec2(size, size)));
            true
        } else { false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_eight_pokemon_textures_can_survive_a_frame() {
        assert_eq!(MAX_POKEMON_TEXTURES,8);
        let mut cache=VisualCache::default();
        cache.pokemon_requested.insert((Kind::Pokemon,25));
        cache.pokemon_visible.insert((Kind::Pokemon,25));
        cache.begin_frame();
        assert!(cache.pokemon_requested.is_empty());
        assert!(cache.pokemon_visible.is_empty());
        cache.end_frame();
        assert_eq!(cache.pokemon_pixels,0);
    }

    #[test]
    fn pokemon_textures_bound_decoded_pixels_without_distorting_aspect_ratio() {
        assert_eq!(pokemon_texture_size(512,256),(128,64));
        assert_eq!(pokemon_texture_size(128,256),(64,128));
        assert_eq!(pokemon_texture_size(32,64),(32,64));
        assert_eq!(pokemon_texture_size(1,512),(1,128));
        assert!(MAX_POKEMON_TEXTURE_PIXELS<=2*1024*1024);
    }

    #[test]
    fn image_urls_never_accept_untrusted_domains_or_traversal() {
        assert!(trusted_url("https://evil.example/icon.png").is_none());
        assert!(trusted_url("//evil.example/a.png").is_none());
        assert!(trusted_url("/assets/../secrets").is_none());
        assert!(trusted_url("https://pokeidle.io/assets/a.png?token=a").is_none());
        assert_eq!(trusted_url("/assets/a.png").unwrap(),
            "https://pokeidle.io/assets/a.png");
    }
    #[test]
    fn all_native_restock_choices_are_bundled_and_valid() {
        for id in 1..=5 {
            let bytes = generated::bytes("ball", id).expect("missing embedded Poké Ball");
            assert!(valid_image(bytes), "invalid embedded Poké Ball {id}");
        }
        for id in [200, 201, 202, 203, 204, 205, 206, 70070] {
            let bytes = generated::bytes("item", id).expect("missing embedded restock item");
            assert!(valid_image(bytes), "invalid embedded restock image {id}");
        }
        assert!(generated::bytes("item", 66).is_some());
        assert_eq!(generated::bytes("item", 59200), generated::bytes("item", 59247));
        // Pokémon artwork remains dynamic and is never included in the item bundle.
        assert!(generated::bytes("pokemon", 25).is_none());
        assert!(generated::bytes("pokemon-looktype", 60025).is_none());
    }
    #[test]
    fn item_fallback_is_public_game_only() {
        let urls = image_urls(Kind::Ball, 1, None);
        assert_eq!(urls.first().map(String::as_str),
            Some("https://pokeidle.io/assets/site/assets/ui/ball-poke.png"));
        assert!(urls.iter().all(|url| !url.contains("raw.githubusercontent.com")));
        assert!(image_urls(Kind::Item, 999999, None).is_empty());
        assert!(image_urls(Kind::Pokemon, 999999, None).is_empty());
    }
    #[test]
    fn known_asset_paths_are_versioned_explicitly() {
        assert!(asset_url(Kind::Ball, 1).is_some());
        assert!(asset_url(Kind::Item, 70070).is_some());
        assert!(asset_url(Kind::Item, 205).is_some());
        assert!(asset_url(Kind::Item, 206).is_some());
        assert!(asset_url(Kind::Pokemon, 1).is_none());
        assert!(upstream_item_icons().len() >= 400);
        assert_eq!(upstream_item_url(200).as_deref(),
            Some("https://pokeidle.io/assets/site/assets/markitems/small_potion.png"));
        assert_eq!(upstream_item_url(66).as_deref(),
            Some("https://pokeidle.io/assets/site/assets/items/heart_stone.gif"));
        assert_eq!(upstream_item_url(100_000_000), None);
    }
}
