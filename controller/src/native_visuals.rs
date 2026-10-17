//! Shared, lazy native sprite renderer. Item and ball artwork is bundled;
//! unknown items and Pokémon retain the on-demand disk cache. Unknown art
//! can be fetched once into a per-user disk cache on a dedicated worker.
//! No image decoding, HTTP, or filesystem reads happen on ordinary UI frames.
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::BTreeMap;
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
        Self { textures: BTreeMap::new(), attempted: BTreeMap::new(), request, results }
    }
}

impl VisualCache {
    fn put(&mut self, ctx: &egui::Context, key: (Kind, u64), frames: &[Vec<u8>]) {
        let mut textures = Vec::with_capacity(frames.len().min(12));
        for (frame, bytes) in frames.iter().take(12).enumerate() {
            let Ok(img) = image::load_from_memory(bytes) else { continue };
            if img.width() == 0 || img.height() == 0 ||
                img.width() > MAX_EDGE || img.height() > MAX_EDGE { continue; }
            let rgba = img.into_rgba8();
            let pixels = ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
            textures.push(ctx.load_texture(
                format!("native-visual-{}-{}-{frame}", key.0.key(), key.1),
                pixels, TextureOptions::NEAREST));
        }
        if !textures.is_empty() { self.textures.insert(key, textures); }
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(reply) = self.results.try_recv() {
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
        self.ensure(ui, kind, id, icon_hint);
        self.textures.get(&(kind, id))
            .and_then(|frames| frames.first().map(TextureHandle::id))
    }

    /// Animation runs only when a Pokémon is actually painted by a focused
    /// window. The four-frame-per-second repaint is independent of game
    /// polling, and an atlas marker or offline fallback remains static.
    pub fn pokemon_texture_id(&mut self, ui: &mut egui::Ui, kind: Kind, id: u64)
        -> Option<egui::TextureId>
    {
        if !matches!(kind, Kind::Pokemon | Kind::PokemonLooktype) { return None; }
        self.ensure(ui, kind, id, None);
        let frames = self.textures.get(&(kind, id))?;
        let frame = if frames.len() > 1 && ui.input(|input| input.focused) {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
            ui.input(|input| (input.time.max(0.0) * 4.0) as usize) % frames.len()
        } else { 0 };
        frames.get(frame).map(TextureHandle::id)
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
