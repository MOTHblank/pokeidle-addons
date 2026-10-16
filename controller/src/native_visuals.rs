//! Shared, lazy native sprite renderer. Bundled art is preferred; unknown art
//! can be fetched once into a per-user disk cache on a dedicated worker.
//! No image decoding, HTTP, or filesystem reads happen on ordinary UI frames.
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

mod generated {
    include!("native_visual_assets.generated.rs");
}
const MAX_IMAGE_BYTES: u64 = 1_048_576;
const MAX_EDGE: u32 = 1024;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind { Ball, Item, Pokemon }
impl Kind {
    fn key(self) -> &'static str {
        match self { Self::Ball => "ball", Self::Item => "item", Self::Pokemon => "pokemon" }
    }
}

struct Request { kind: Kind, id: u64, urls: Vec<String> }
struct Response { key: (Kind, u64), bytes: Option<Vec<u8>> }

pub struct VisualCache {
    textures: BTreeMap<(Kind, u64), TextureHandle>,
    attempted: BTreeSet<(Kind, u64)>,
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

fn asset_url(kind: Kind, id: u64) -> Option<&'static str> {
    match (kind, id) {
        (Kind::Ball, 1) => Some("/assets/site/assets/ui/ball-poke.png"),
        (Kind::Ball, 2) => Some("/assets/site/assets/ui/ball-great.png"),
        (Kind::Ball, 3) => Some("/img/ball-super.png"),
        (Kind::Ball, 4) => Some("/assets/site/assets/ui/ball-ultra.png"),
        (Kind::Ball, 5) => Some("/img/ball-beast.png"),
        (Kind::Item, 200) => Some("/assets/markitems/small_potion.png"),
        (Kind::Item, 201) => Some("/assets/markitems/great_potion.png"),
        (Kind::Item, 202) => Some("/assets/markitems/ultra_potion.png"),
        (Kind::Item, 203) => Some("/assets/markitems/hyper_potion.png"),
        (Kind::Item, 204) => Some("/assets/markitems/ultimate_potion.png"),
        (Kind::Item, 205) => Some("/assets/markitems/revive.png"),
        (Kind::Item, 206) => Some("/assets/markitems/max_revive.png"),
        (Kind::Item, 70070) => Some("/img/itens/golden-potion.png"),
        _ => None,
    }
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
        std::thread::Builder::new()
            .name("moth-visual-cache".into())
            .spawn(move || {
                let root = cache_dir();
                let _ = std::fs::create_dir_all(&root);
                let Ok(client) = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(8))
                    .connect_timeout(std::time::Duration::from_secs(3))
                    .redirect(reqwest::redirect::Policy::none())
                    .build() else { return; };
                for job in jobs {
                    let bytes = load_worker_image(&client, &root, &job);
                    if output.send(Response { key: (job.kind, job.id), bytes }).is_err() { break; }
                }
            }).ok();
        Self { textures: BTreeMap::new(), attempted: BTreeSet::new(), request, results }
    }
}

impl VisualCache {
    fn put(&mut self, ctx: &egui::Context, key: (Kind, u64), bytes: &[u8]) {
        let Ok(img) = image::load_from_memory(bytes) else { return };
        if img.width() == 0 || img.height() == 0 ||
            img.width() > MAX_EDGE || img.height() > MAX_EDGE { return; }
        let rgba = img.into_rgba8();
        let pixels = ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
        let handle = ctx.load_texture(
            format!("native-visual-{}-{}", key.0.key(), key.1),
            pixels, TextureOptions::NEAREST);
        self.textures.insert(key, handle);
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(reply) = self.results.try_recv() {
            if let Some(bytes) = reply.bytes {
                self.put(ctx, reply.key, &bytes);
                ctx.request_repaint();
            }
        }
    }

    /// Returns true if an icon was drawn. Always fall back to text if false.
    /// Unknown Pokémon can be added through the generated sprite manifest;
    /// no guessed Pokémon CDN paths are constructed here.
    pub fn show(&mut self, ui: &mut egui::Ui, kind: Kind, id: u64,
                icon_hint: Option<&str>, size: f32) -> bool {
        if id == 0 { return false; }
        let key = (kind, id);
        if !self.attempted.contains(&key) {
            self.attempted.insert(key);
            if let Some(bytes) = generated::bytes(kind.key(), id) {
                if valid_image(bytes) { self.put(ui.ctx(), key, bytes); }
            }
            if !self.textures.contains_key(&key) {
                let mut urls = Vec::<String>::new();
                for candidate in [icon_hint, asset_url(kind, id)] {
                    let Some(source) = candidate.and_then(trusted_url) else {continue};
                    if !urls.contains(&source) { urls.push(source.clone()); }
                    // Some game builds serve items.json paths under
                    // /assets/site. Probe both, but only on the same trusted
                    // public game host and only from the background worker.
                    const SHORT: &str = "https://pokeidle.io/assets/markitems/";
                    const LONG: &str = "https://pokeidle.io/assets/site/assets/markitems/";
                    let alternative = if source.starts_with(SHORT) {
                        Some(source.replacen(SHORT,LONG,1))
                    } else if source.starts_with(LONG) {
                        Some(source.replacen(LONG,SHORT,1))
                    } else { None };
                    if let Some(alternative) = alternative {
                        if !urls.contains(&alternative) { urls.push(alternative); }
                    }
                }
                if !urls.is_empty() {
                    let _ = self.request.send(Request { kind, id, urls });
                }
            }
        }
        if let Some(texture) = self.textures.get(&key) {
            ui.image((texture.id(), egui::vec2(size, size)));
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
    fn known_asset_paths_are_versioned_explicitly() {
        assert!(asset_url(Kind::Ball, 1).is_some());
        assert!(asset_url(Kind::Item, 70070).is_some());
        assert!(asset_url(Kind::Pokemon, 1).is_none());
    }
}
