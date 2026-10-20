//! Demand-driven PokéIdle Pokémon artwork for Overview cards.
//!
//! Acquisition reference: DiuleJR/pokeidle-manager (GPL-3.0-only),
//! src-tauri/src/assets.rs and src/inventory/asset-components.tsx.
//! Owned Pokémon have looktypes, but wild opponents only have species IDs.
//! Resolve species through the game catalogs, use the marker atlas first,
//! and fall back to verified per-looktype outfit manifests.
//! Only cropped, small PNGs enter egui. Sheets live in the disk cache.
use super::{valid_image, Kind};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

const ORIGIN: &str = "https://pokeidle.io/";
const MAX_JSON_BYTES: u64 = 3 * 1024 * 1024;
const MAX_SHEET_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SHEET_PIXELS: u64 = 36_000_000;
const MAX_SPRITE_EDGE: u32 = 512;

fn download_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

// Refuse untrusted hosts, traversal and dynamic paths from remote manifests.
fn asset_path(raw: &str) -> Option<String> {
    let raw = raw.split('?').next()?.trim_start_matches('/');
    let path = if let Some(tail) = raw.strip_prefix("assets-packs/") {
        format!("assets/asset-packs/{tail}")
    } else if let Some(tail) = raw.strip_prefix("asset-packs/") {
        format!("assets/asset-packs/{tail}")
    } else if raw.starts_with("assets/") {
        raw.to_owned()
    } else if raw.starts_with("site/") {
        format!("assets/{raw}")
    } else { return None };
    if path.contains("..") || path.contains('\\') || path.contains('#') ||
        !path.bytes().all(|byte| byte.is_ascii_alphanumeric() ||
            matches!(byte, b'/' | b'.' | b'_' | b'-')) ||
        !(path.ends_with(".json") || path.ends_with(".png") || path.ends_with(".gif")) {
        return None;
    }
    Some(path)
}

// File keys are generated internally; the server never controls disk paths.
fn cached_bytes(client: &Client, root: &Path, file: &str,
                url_path: &str, limit: u64) -> Option<Vec<u8>> {
    let url_path = asset_path(url_path)?;
    let target = root.join(file);
    let valid = |bytes: Vec<u8>| -> Option<Vec<u8>> {
        (!bytes.is_empty() && bytes.len() as u64 <= limit).then_some(bytes)
    };
    if let Ok(bytes) = std::fs::read(&target) {
        if let Some(bytes) = valid(bytes) { return Some(bytes); }
    }
    // Multiple cards request the same atlas. Only one worker downloads it.
    let _guard = download_lock().lock().ok()?;
    if let Ok(bytes) = std::fs::read(&target) {
        if let Some(bytes) = valid(bytes) { return Some(bytes); }
    }
    let response = client.get(format!("{ORIGIN}{url_path}"))
        .send().and_then(|response| response.error_for_status()).ok()?;
    if response.content_length().is_some_and(|length| length > limit) { return None; }
    let mut bytes = Vec::new();
    response.take(limit + 1).read_to_end(&mut bytes).ok()?;
    let bytes = valid(bytes)?;
    let temporary = target.with_extension("tmp");
    if std::fs::write(&temporary, &bytes).is_ok() {
        let _ = std::fs::rename(temporary, target);
    }
    Some(bytes)
}

fn catalog(client: &Client, root: &Path, name: &str, path: &str) -> Option<Value> {
    let bytes = cached_bytes(client, root, name, path, MAX_JSON_BYTES)?;
    serde_json::from_slice(&bytes).ok()
}

fn merge_species(map: &mut BTreeMap<u64, u64>, document: &Value, key: &str) {
    let Some(entries) = document.get(key).and_then(Value::as_array) else { return };
    for entry in entries {
        if let (Some(species), Some(looktype)) = (
            entry.get("pokeId").and_then(Value::as_u64),
            entry.get("looktype").and_then(Value::as_u64),
        ) {
            if species > 0 && looktype > 1 && looktype < 100_000 {
                map.insert(species, looktype);
            }
        }
    }
}

fn species_looktype(client: &Client, root: &Path, species: u64) -> Option<u64> {
    if species == 0 { return None; }
    // The checked-in upstream identity table covers every known PokéIdle
    // species, including special 2xxx / 13xxx / 14xxx variants. Do not
    // require live game manifests to determine those looktypes.
    if let Some(looktype) = crate::native_hunts::game_species_looktype(species) {
        return Some(looktype);
    }
    static INDEX: OnceLock<Mutex<Option<BTreeMap<u64, u64>>>> = OnceLock::new();
    let mut index = INDEX.get_or_init(|| Mutex::new(None)).lock().ok()?;
    if index.is_none() {
        let mut resolved = BTreeMap::new();
        for (cache, url) in [
            ("creatures-base.json", "/assets/creatures.json"),
            ("creatures-current.json", "/assets/creatures-novos.json"),
            ("creatures-outland.json", "/assets/creatures-outland-novos.json"),
            ("creatures-forms.json", "/assets/creatures-formas.json"),
        ] {
            if let Some(document) = catalog(client, root, cache, url) {
                if document.get("creatures").is_some_and(Value::is_array) {
                    merge_species(&mut resolved, &document, "creatures");
                }
            }
        }
        if let Some(patches) = catalog(client, root, "creatures-sprite-patches.json",
            "/assets/creatures-sprites-lab.json") {
            merge_species(&mut resolved, &patches, "patches");
        }
        // Upstream's base creature JSONs can be empty while the verified
        // sprite-patch catalog is populated. Those patches are sufficient.
        if resolved.is_empty() { return None; }
        *index = Some(resolved);
    }
    index.as_ref()?.get(&species).copied()
}

fn marker_slot(atlas: &Value, looktype: u64) -> Option<(String, u32, u32, u32)> {
    let image = asset_path(atlas.get("image")?.as_str()?)?;
    let cell = u32::try_from(atlas.get("cell")?.as_u64()?).ok()?;
    if !(1..=MAX_SPRITE_EDGE).contains(&cell) { return None; }
    let slot = atlas.get("slots")?.get(looktype.to_string())?.as_array()?;
    let (x, y) = (u32::try_from(slot.first()?.as_u64()?).ok()?,
                  u32::try_from(slot.get(1)?.as_u64()?).ok()?);
    Some((image, x.checked_mul(cell)?, y.checked_mul(cell)?, cell))
}

// Crop only the first valid still frame from an outfit sheet. Decode the
// source at most once, and never allocate PNGs for unused animation frames.
// Reject oversized sheets and invalid rectangles before cropping.
fn crop_frames_png(bytes: &[u8], rectangles: &[[u32; 4]]) -> Option<Vec<Vec<u8>>> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format().ok()?;
    let (sheet_width, sheet_height) = reader.into_dimensions().ok()?;
    if u64::from(sheet_width) * u64::from(sheet_height) > MAX_SHEET_PIXELS {
        return None;
    }
    let sheet = image::load_from_memory(bytes).ok()?;
    let mut frames = Vec::new();
    for [x, y, width, height] in rectangles.iter().take(12).copied() {
        if width == 0 || height == 0 ||
            width > MAX_SPRITE_EDGE || height > MAX_SPRITE_EDGE ||
            x.checked_add(width).is_none_or(|right| right > sheet_width) ||
            y.checked_add(height).is_none_or(|bottom| bottom > sheet_height) {
            continue;
        }
        let mut output = Cursor::new(Vec::new());
        if sheet.crop_imm(x, y, width, height)
            .write_to(&mut output, image::ImageFormat::Png).is_ok() {
            let png = output.into_inner();
            if valid_image(&png) { frames.push(png); break; }
        }
    }
    (!frames.is_empty()).then_some(frames)
}

fn crop_png(bytes: &[u8], x: u32, y: u32, width: u32, height: u32) -> Option<Vec<u8>> {
    crop_frames_png(bytes, &[[x, y, width, height]])?.into_iter().next()
}

fn marker_sprite(client: &Client, root: &Path, looktype: u64) -> Option<Vec<u8>> {
    let atlas = catalog(client, root, "marker-atlas.json",
        "/assets/site/assets/maps/marker-atlas.json")?;
    let (image, x, y, cell) = marker_slot(&atlas, looktype)?;
    let sheet = cached_bytes(client, root, "marker-atlas-sheet.img", &image, MAX_SHEET_BYTES)?;
    crop_png(&sheet, x, y, cell, cell)
}

/// Exact frame rectangles taken from the game outfit manifest, not inferred
/// from a grid or synthesized from the Pokémon species ID. Keep a bounded
/// fallback list to find the first valid still frame.
fn outfit_frames(manifest: &Value) -> Option<(String, Vec<[u32; 4]>)> {
    let categories = manifest.get("categories")?.as_object()?;
    let category = categories.values().next()?;
    let page = category.get("pages")?.as_array()?.first()?;
    let image = asset_path(page.get("image")?.as_str()?)?;
    let page_width = u32::try_from(page.get("width")?.as_u64()?).ok()?;
    let page_height = u32::try_from(page.get("height")?.as_u64()?).ok()?;
    if page_width == 0 || page_height == 0 { return None; }
    let frames = manifest.get("assets")?.as_object()?
        .iter().find(|(name, _)| name.ends_with("/1_1_1_3.png"))?
        .1.get("frames")?.as_array()?;
    let mut rectangles = Vec::new();
    for frame in frames.iter().take(12) {
        let num = |key: &str| u32::try_from(frame.get(key)?.as_u64()?).ok();
        let (Some(x), Some(y), Some(w), Some(h)) =
            (num("x"), num("y"), num("w"), num("h")) else { continue };
        if w == 0 || h == 0 || w > MAX_SPRITE_EDGE || h > MAX_SPRITE_EDGE ||
            x.checked_add(w).is_none_or(|right| right > page_width) ||
            y.checked_add(h).is_none_or(|bottom| bottom > page_height) {
            continue;
        }
        rectangles.push([x, y, w, h]);
    }
    (!rectangles.is_empty()).then_some((image, rectangles))
}

fn outfit_sprites(client: &Client, root: &Path, looktype: u64) -> Option<Vec<Vec<u8>>> {
    let index = catalog(client, root, "outfits-index.json",
        "/assets/asset-packs/outfits-index.json")?;
    let manifest_path = index.get("outfits")?.get(looktype.to_string())?
        .get("manifest")?.as_str()?;
    let manifest = catalog(client, root, &format!("outfit-{looktype}.json"), manifest_path)?;
    let (image, frames) = outfit_frames(&manifest)?;
    let sheet = cached_bytes(client, root, &format!("outfit-{looktype}.img"),
        &image, MAX_SHEET_BYTES)?;
    crop_frames_png(&sheet, &frames)
}

// Small, conventional Pokédex sprites are a reliable last-resort visual when
// PokéIdle's dynamic outfit manifests are empty or a looktype has no atlas slot.
// Fetch only the two Pokémon used by Overview on demand, then reuse the disk
// cache. No sprite database or artwork is packaged in the application.
fn static_sprite_url(kind: Kind, id: u64) -> Option<String> {
    let (species, shiny) = match kind {
        // The game species ID is not the Pokédex ID (e.g. PokéIdle 2006 is
        // Furious Scyther; its documented base looktype is 60123).
        Kind::Pokemon => (crate::native_hunts::game_species_dex_sprite(id)?, false),
        // The upstream creatures-sprites-lab catalog documents these two
        // looktype families as 60000+dex and 70000+dex.
        Kind::PokemonLooktype if (60001..=61025).contains(&id) => (id - 60000, false),
        Kind::PokemonLooktype if (70001..=71025).contains(&id) => (id - 70000, true),
        _ => return None,
    };
    let variant = if shiny { "shiny/" } else { "" };
    Some(format!(
        "https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/{variant}{species}.png"
    ))
}

fn static_sprite(client: &Client, root: &Path, kind: Kind, id: u64) -> Option<Vec<u8>> {
    let primary = static_sprite_url(kind, id)?;
    // A number of players can reach the game but not raw.githubusercontent.com.
    // Use a second public mirror of the same PokeAPI sprite path before
    // falling back to the game's looktype manifests.
    let mirror = primary.replacen(
        "https://raw.githubusercontent.com/PokeAPI/sprites/master/",
        "https://cdn.jsdelivr.net/gh/PokeAPI/sprites@master/", 1);
    for url in [primary, mirror] {
        let Ok(response) = client.get(url).send()
            .and_then(|response| response.error_for_status()) else { continue };
        if response.content_length().is_some_and(|length|
            length > super::MAX_IMAGE_BYTES) { continue; }
        let mut bytes = Vec::new();
        if response.take(super::MAX_IMAGE_BYTES + 1).read_to_end(&mut bytes).is_err()
            || !valid_image(&bytes) { continue; }
        let path = root.join(format!("{}-{id}.img", kind.key()));
        let temporary = path.with_extension("tmp");
        if std::fs::write(&temporary, &bytes).is_ok() {
            let _ = std::fs::rename(&temporary, &path);
        }
        return Some(bytes);
    }
    None
}

/// Use the first cached frame immediately. Prefer a small, guaranteed
/// national-dex PNG for known species and documented 60k/70k looktypes.
/// Retain the game's outfits and marker atlas for all other looktypes.
pub(super) fn load(client: &Client, root: &Path, kind: Kind, id: u64) -> Option<Vec<Vec<u8>>> {
    if id == 0 { return None; }
    let output = root.join(format!("{}-{id}.img", kind.key()));
    if let Ok(bytes) = std::fs::read(&output) {
        if valid_image(&bytes) { return Some(vec![bytes]); }
        let _ = std::fs::remove_file(&output);
    }
    if let Some(bytes) = static_sprite(client, root, kind, id) {
        return Some(vec![bytes]);
    }
    let looktype = match kind {
        Kind::Pokemon => species_looktype(client, root, id),
        Kind::PokemonLooktype => Some(id),
        _ => None,
    }?;
    let frames = outfit_sprites(client, root, looktype)
        .or_else(|| marker_sprite(client, root, looktype).map(|bytes| vec![bytes]))?;
    if let Some(first) = frames.first() {
        let temporary = output.with_extension("tmp");
        if std::fs::write(&temporary, first).is_ok() {
            let _ = std::fs::rename(&temporary, &output);
        }
    }
    Some(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dex_sprite_fallback_is_bounded_and_only_used_for_verified_id_families() {
        assert_eq!(static_sprite_url(Kind::Pokemon, 25).as_deref(),
            Some("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/25.png"));
        assert_eq!(static_sprite_url(Kind::PokemonLooktype, 60025),
            static_sprite_url(Kind::Pokemon, 25));
        assert_eq!(static_sprite_url(Kind::PokemonLooktype, 70025).as_deref(),
            Some("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/shiny/25.png"));
        assert!(static_sprite_url(Kind::Pokemon, 0).is_none());
        assert!(static_sprite_url(Kind::Pokemon, 1026).is_none());
        // Forms and Megas have their OWN looktypes, not the national dex art.
        assert_eq!(crate::native_hunts::game_species_looktype(4019),Some(64019));
        assert_eq!(crate::native_hunts::game_species_looktype(5052),Some(65052));
        assert_eq!(crate::native_hunts::game_species_looktype(9006),Some(83006));
        assert_eq!(crate::native_hunts::game_species_looktype(3998),Some(80998));
        assert!(static_sprite_url(Kind::Pokemon,4019).is_none());
        assert!(static_sprite_url(Kind::Pokemon,9006).is_none());
        assert_eq!(static_sprite_url(Kind::Pokemon, 2006).as_deref(),
            Some("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/123.png"));
        assert_eq!(static_sprite_url(Kind::Pokemon, 14009).as_deref(),
            Some("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/9.png"));
        assert_eq!(static_sprite_url(Kind::Pokemon, 14448).as_deref(),
            Some("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/448.png"));
        assert!(static_sprite_url(Kind::PokemonLooktype, 47861).is_none());
    }

    #[test]
    fn paths_remain_on_game_origin() {
        assert_eq!(asset_path("/assets-packs/pokemon/a.png?rev=1").as_deref(),
            Some("assets/asset-packs/pokemon/a.png"));
        assert_eq!(asset_path("site/assets/maps/a.png").as_deref(),
            Some("assets/site/assets/maps/a.png"));
        assert!(asset_path("https://evil.example/a.png").is_none());
        assert!(asset_path("/assets/../private.png").is_none());
        assert!(asset_path("/assets/b.png#fragment").is_none());
    }
    #[test]
    fn species_patch_wins_and_invalid_ids_are_not_mapped() {
        let mut map = BTreeMap::new();
        merge_species(&mut map, &serde_json::json!({"creatures": [
            {"pokeId": 25, "looktype": 131}, {"pokeId": 1, "looktype": 1}]}), "creatures");
        merge_species(&mut map, &serde_json::json!({"patches": [
            {"pokeId": 25, "looktype": 132}]}), "patches");
        assert_eq!(map.get(&25), Some(&132));
        assert!(!map.contains_key(&1));
    }
    #[test]
    fn atlas_coordinates_and_sprite_frame_are_grounded() {
        let atlas = serde_json::json!({"image":"/assets/site/assets/maps/m.png",
            "cell":64,"slots":{"131":[2,3]}});
        assert_eq!(marker_slot(&atlas, 131),
            Some(("assets/site/assets/maps/m.png".into(), 128, 192, 64)));
        assert!(marker_slot(&atlas, 999).is_none());
        let manifest = serde_json::json!({"categories":{"main":{"pages":[
            {"image":"/assets-packs/mon/page.png"}]}} , "assets": {
            "test/1_1_1_3.png":{"frames":[{"x":1,"y":2,"w":32,"h":40}]}}});
        assert_eq!(outfit_frames(&serde_json::json!({
            "categories":{"main":{"pages":[
                {"image":"/assets-packs/mon/page.png","width":256,"height":256}]}},
            "assets":{"test/1_1_1_3.png":{"frames":[
                {"x":1,"y":2,"w":32,"h":40},
                {"x":33,"y":2,"w":32,"h":40}
            ]}}
        })),
            Some(("assets/asset-packs/mon/page.png".into(),
                vec![[1,2,32,40],[33,2,32,40]])));
        assert!(outfit_frames(&manifest).is_none());
    }
    #[test]
    fn static_outfit_crops_only_one_frame() {
        let mut sheet=Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(32,16)
            .write_to(&mut sheet,image::ImageFormat::Png).unwrap();
        let frames=crop_frames_png(&sheet.into_inner(),
            &[[0,0,16,16],[16,0,16,16]]).unwrap();
        assert_eq!(frames.len(),1);
    }
    #[test]
    fn rejects_frames_outside_sheet_without_losing_valid_first_frame() {
        let manifest = serde_json::json!({
            "categories":{"main":{"pages":[{
                "image":"/assets-packs/mon/a.png","width":64,"height":64}]}},
            "assets":{"some/1_1_1_3.png":{"frames":[
                {"x":0,"y":0,"w":16,"h":16},
                {"x":63,"y":63,"w":32,"h":32}
            ]}}
        });
        assert_eq!(outfit_frames(&manifest).unwrap().1, vec![[0,0,16,16]]);
        let mut sheet = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(64,64)
            .write_to(&mut sheet, image::ImageFormat::Png).unwrap();
        assert_eq!(crop_frames_png(&sheet.into_inner(), &[[0,0,16,16],[63,63,32,32]])
            .unwrap().len(), 1);
    }

}
