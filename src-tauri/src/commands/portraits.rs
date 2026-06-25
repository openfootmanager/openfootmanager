use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;
use tauri::{AppHandle, Manager};

const GENERATOR_VERSION: &str = "runtime-component-recipe-rust-v1";
const SIZE: u32 = 384;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPortraitRequest {
    pub player_id: String,
    pub full_name: Option<String>,
    pub match_name: Option<String>,
    pub nationality: Option<String>,
    pub date_of_birth: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPortraitResponse {
    pub generator: &'static str,
    pub cache_key: String,
    pub source_id: String,
    pub cache_path: String,
    pub data_url: Option<String>,
    pub generated: bool,
    pub render_ms: f64,
    pub elapsed_ms: f64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrewarmPlayerPortraitRecord {
    pub player_id: String,
    pub cache_key: String,
    pub source_id: String,
    pub cache_path: String,
    pub generated: bool,
    pub render_ms: f64,
    pub elapsed_ms: f64,
    pub data_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrewarmPlayerPortraitsResponse {
    pub generator: &'static str,
    pub requested_count: usize,
    pub generated_count: usize,
    pub cached_count: usize,
    pub failed_count: usize,
    pub render_ms: f64,
    pub elapsed_ms: f64,
    pub records: Vec<PrewarmPlayerPortraitRecord>,
}

struct PortraitSource {
    id: &'static str,
    image: RgbaImage,
}

#[derive(Clone, Copy)]
struct Recipe {
    shirt_rgb: [u8; 3],
    hair_rgb: [u8; 3],
    skin_warmth: f32,
    exposure: f32,
    contrast: f32,
    head_width: f32,
    jaw_width: f32,
    head_height: f32,
    shift_x: f32,
    shirt_strength: f32,
    hair_strength: f32,
    beard_strength: f32,
}

#[derive(Clone, Copy)]
struct PixelF {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

static SOURCES: OnceLock<Result<Vec<PortraitSource>, String>> = OnceLock::new();
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

const SOURCE_BYTES: &[(&str, &[u8])] = &[
    (
        "chroma-01-mediterranean",
        include_bytes!("../../assets/portrait-sources/chroma-01-mediterranean.webp"),
    ),
    (
        "chroma-02-west-african",
        include_bytes!("../../assets/portrait-sources/chroma-02-west-african.webp"),
    ),
    (
        "chroma-04-west-european-bald",
        include_bytes!("../../assets/portrait-sources/chroma-04-west-european-bald.webp"),
    ),
    (
        "chroma-05-east-asian",
        include_bytes!("../../assets/portrait-sources/chroma-05-east-asian.webp"),
    ),
    (
        "chroma-06-south-asian",
        include_bytes!("../../assets/portrait-sources/chroma-06-south-asian.webp"),
    ),
    (
        "chroma-07-latin-american",
        include_bytes!("../../assets/portrait-sources/chroma-07-latin-american.webp"),
    ),
    (
        "chroma-08-middle-eastern",
        include_bytes!("../../assets/portrait-sources/chroma-08-middle-eastern.webp"),
    ),
    (
        "chroma-09-caribbean",
        include_bytes!("../../assets/portrait-sources/chroma-09-caribbean.webp"),
    ),
    (
        "chroma-10-southeast-asian",
        include_bytes!("../../assets/portrait-sources/chroma-10-southeast-asian.webp"),
    ),
    (
        "chroma-11-indigenous-andean",
        include_bytes!("../../assets/portrait-sources/chroma-11-indigenous-andean.webp"),
    ),
    (
        "chroma-12-polynesian",
        include_bytes!("../../assets/portrait-sources/chroma-12-polynesian.webp"),
    ),
];

#[tauri::command]
pub async fn generate_player_portrait(
    app: AppHandle,
    request: PlayerPortraitRequest,
) -> Result<PlayerPortraitResponse, String> {
    let cache_dir = portrait_cache_dir(&app)?;

    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        let (record, _) = ensure_portrait_file(&cache_dir, &request, false)?;
        let response = PlayerPortraitResponse {
            generator: GENERATOR_VERSION,
            cache_key: record.cache_key,
            source_id: record.source_id,
            cache_path: record.cache_path,
            data_url: None,
            generated: record.generated,
            render_ms: record.render_ms,
            elapsed_ms: round3(elapsed_ms(started)),
            width: SIZE,
            height: SIZE,
        };

        log::debug!(
            "[portraits] generated single portrait player_id={} generated={} render_ms={} elapsed_ms={}",
            request.player_id,
            response.generated,
            response.render_ms,
            response.elapsed_ms
        );

        Ok(response)
    })
    .await
    .map_err(|error| format!("portrait generation task failed: {error}"))?
}

#[tauri::command]
pub async fn prewarm_player_portraits(
    app: AppHandle,
    requests: Vec<PlayerPortraitRequest>,
) -> Result<PrewarmPlayerPortraitsResponse, String> {
    let cache_dir = portrait_cache_dir(&app)?;

    tauri::async_runtime::spawn_blocking(move || {
        let result = prewarm_player_portraits_to_dir(&cache_dir, &requests)?;
        log::debug!(
            "[portraits] prewarmed portrait batch requested={} generated={} cached={} failed={} render_ms={} elapsed_ms={}",
            result.requested_count,
            result.generated_count,
            result.cached_count,
            result.failed_count,
            result.render_ms,
            result.elapsed_ms
        );
        Ok(result)
    })
    .await
    .map_err(|error| format!("portrait prewarm task failed: {error}"))?
}

fn portrait_cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data dir: {error}"))?
        .join("generated-player-portraits")
        .join(GENERATOR_VERSION))
}

fn prewarm_player_portraits_to_dir(
    cache_dir: &Path,
    requests: &[PlayerPortraitRequest],
) -> Result<PrewarmPlayerPortraitsResponse, String> {
    let started = Instant::now();
    let mut records = Vec::with_capacity(requests.len());
    let mut failed_count = 0usize;

    for request in requests {
        match ensure_portrait_file(cache_dir, request, false) {
            Ok((record, _)) => records.push(record),
            Err(error) => {
                failed_count += 1;
                log::warn!(
                    "[portraits] failed to prewarm portrait for player_id={}: {}",
                    request.player_id,
                    error
                );
            }
        }
    }

    let generated_count = records.iter().filter(|record| record.generated).count();
    let cached_count = records.len().saturating_sub(generated_count);
    let render_ms = records.iter().map(|record| record.render_ms).sum();

    Ok(PrewarmPlayerPortraitsResponse {
        generator: GENERATOR_VERSION,
        requested_count: requests.len(),
        generated_count,
        cached_count,
        failed_count,
        render_ms: round3(render_ms),
        elapsed_ms: round3(elapsed_ms(started)),
        records,
    })
}

fn ensure_portrait_file(
    cache_dir: &Path,
    request: &PlayerPortraitRequest,
    include_bytes: bool,
) -> Result<(PrewarmPlayerPortraitRecord, Option<Vec<u8>>), String> {
    let started = Instant::now();
    let seed = portrait_seed(request);
    let sources = portrait_sources()?;
    let source = select_source(sources, seed);
    let recipe = build_recipe(seed, source.id);
    let cache_key = cache_key(seed, source.id);
    fs::create_dir_all(cache_dir)
        .map_err(|error| format!("failed to create portrait cache: {error}"))?;
    let cache_path = cache_dir.join(format!("{cache_key}.webp"));

    if cache_path.exists() {
        let bytes = if include_bytes {
            Some(
                fs::read(&cache_path)
                    .map_err(|error| format!("failed to read portrait cache: {error}"))?,
            )
        } else {
            None
        };
        return Ok((
            PrewarmPlayerPortraitRecord {
                player_id: request.player_id.clone(),
                cache_key,
                source_id: source.id.to_string(),
                cache_path: cache_path.to_string_lossy().to_string(),
                generated: false,
                render_ms: 0.0,
                elapsed_ms: round3(elapsed_ms(started)),
                data_url: None,
            },
            bytes,
        ));
    }

    let render_started = Instant::now();
    let portrait = render_recipe(&source.image, &recipe);
    let render_ms = elapsed_ms(render_started);
    let bytes = encode_webp(&portrait, 88.0)?;
    write_cache_file_atomically(&cache_path, &bytes)?;

    Ok((
        PrewarmPlayerPortraitRecord {
            player_id: request.player_id.clone(),
            cache_key,
            source_id: source.id.to_string(),
            cache_path: cache_path.to_string_lossy().to_string(),
            generated: true,
            render_ms: round3(render_ms),
            elapsed_ms: round3(elapsed_ms(started)),
            data_url: None,
        },
        include_bytes.then_some(bytes),
    ))
}

fn write_cache_file_atomically(cache_path: &Path, bytes: &[u8]) -> Result<(), String> {
    let file_name = cache_path
        .file_name()
        .ok_or_else(|| "portrait cache path has no file name".to_string())?;
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut temp_file_name = file_name.to_os_string();
    temp_file_name.push(format!(".{}.tmp", counter));
    let temp_path = cache_path.with_file_name(temp_file_name);

    if let Err(error) = fs::write(&temp_path, bytes) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("failed to write portrait cache temp file: {error}"));
    }

    match fs::rename(&temp_path, cache_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists && cache_path.exists() => {
            let _ = fs::remove_file(&temp_path);
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            Err(format!("failed to publish portrait cache: {error}"))
        }
    }
}

fn portrait_sources() -> Result<&'static [PortraitSource], String> {
    SOURCES
        .get_or_init(load_sources)
        .as_ref()
        .map(|sources| sources.as_slice())
        .map_err(Clone::clone)
}

fn load_sources() -> Result<Vec<PortraitSource>, String> {
    SOURCE_BYTES
        .iter()
        .map(|(id, bytes)| {
            let image = image::load_from_memory(bytes)
                .map_err(|error| format!("failed to decode portrait source {id}: {error}"))?
                .to_rgba8();
            if image.width() != SIZE || image.height() != SIZE {
                return Err(format!(
                    "portrait source {id} is {}x{}, expected {}x{}",
                    image.width(),
                    image.height(),
                    SIZE,
                    SIZE
                ));
            }
            Ok(PortraitSource { id, image })
        })
        .collect()
}

fn select_source(sources: &'static [PortraitSource], seed: u64) -> &'static PortraitSource {
    let mut state = seed ^ 0x7283_7f41_2bcb_901du64;
    &sources[(next_u64(&mut state) as usize) % sources.len()]
}

fn build_recipe(seed: u64, source_id: &str) -> Recipe {
    let mut state = stable_hash_bytes(format!("{seed}:{source_id}:{GENERATOR_VERSION}").as_bytes());
    const SHIRTS: &[[u8; 3]] = &[
        [196, 39, 44],
        [31, 84, 146],
        [245, 245, 245],
        [37, 111, 73],
        [235, 191, 41],
        [72, 69, 84],
        [128, 43, 131],
        [21, 29, 42],
    ];
    const HAIR: &[[u8; 3]] = &[
        [32, 24, 19],
        [74, 47, 29],
        [118, 83, 45],
        [169, 129, 71],
        [116, 66, 43],
        [184, 181, 172],
    ];

    Recipe {
        shirt_rgb: SHIRTS[(next_u64(&mut state) as usize) % SHIRTS.len()],
        hair_rgb: HAIR[(next_u64(&mut state) as usize) % HAIR.len()],
        skin_warmth: rand_range(&mut state, -0.065, 0.070),
        exposure: rand_range(&mut state, -0.070, 0.075),
        contrast: rand_range(&mut state, 0.92, 1.12),
        head_width: rand_range(&mut state, -0.09, 0.10),
        jaw_width: rand_range(&mut state, -0.10, 0.14),
        head_height: rand_range(&mut state, -0.08, 0.10),
        shift_x: rand_range(&mut state, -7.5, 7.5),
        shirt_strength: rand_range(&mut state, 0.56, 0.84),
        hair_strength: rand_range(&mut state, 0.30, 0.86),
        beard_strength: if next_u64(&mut state) % 100 < 34 {
            rand_range(&mut state, 0.12, 0.52)
        } else {
            0.0
        },
    }
}

fn render_recipe(source: &RgbaImage, recipe: &Recipe) -> RgbaImage {
    let mut output = RgbaImage::new(SIZE, SIZE);
    let width = SIZE as f32;
    let height = SIZE as f32;
    let cx = width * 0.5 + recipe.shift_x;
    let cy = height * 0.46;
    let scale_x = 1.0 + recipe.head_width * 0.48 + recipe.jaw_width * 0.20;
    let scale_y = 1.0 + recipe.head_height * 0.45;
    let inv_scale_x = 1.0 / scale_x;
    let inv_scale_y = 1.0 / scale_y;
    let c = cx - (cx * inv_scale_x);
    let f = cy - (cy * inv_scale_y);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let src_x = x as f32 * inv_scale_x + c;
            let src_y = y as f32 * inv_scale_y + f;
            let sampled = bilinear_sample(source, src_x, src_y);
            let color = apply_recipe_color(sampled, x as f32, y as f32, recipe);
            output.put_pixel(
                x,
                y,
                Rgba([
                    to_u8(color.r),
                    to_u8(color.g),
                    to_u8(color.b),
                    to_u8(color.a),
                ]),
            );
        }
    }
    output
}

fn bilinear_sample(source: &RgbaImage, x: f32, y: f32) -> PixelF {
    if x < 0.0 || y < 0.0 || x > (SIZE - 1) as f32 || y > (SIZE - 1) as f32 {
        return PixelF {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };
    }

    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(SIZE - 1);
    let y1 = (y0 + 1).min(SIZE - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;

    let c00 = pixel_to_f(source.get_pixel(x0, y0));
    let c10 = pixel_to_f(source.get_pixel(x1, y0));
    let c01 = pixel_to_f(source.get_pixel(x0, y1));
    let c11 = pixel_to_f(source.get_pixel(x1, y1));
    lerp_pixel(lerp_pixel(c00, c10, tx), lerp_pixel(c01, c11, tx), ty)
}

fn apply_recipe_color(mut pixel: PixelF, x: f32, y: f32, recipe: &Recipe) -> PixelF {
    let width = SIZE as f32;
    let height = SIZE as f32;
    let alpha = pixel.a / 255.0;
    let luma = pixel.r * 0.2126 + pixel.g * 0.7152 + pixel.b * 0.0722;
    let subject = if alpha > 0.08 { 1.0 } else { 0.0 };
    let face = ellipse_mask(
        x,
        y,
        width * 0.5 + recipe.shift_x,
        height * 0.43,
        width * 0.242,
        height * 0.320,
        0.18,
    );
    let head = ellipse_mask(
        x,
        y,
        width * 0.5 + recipe.shift_x,
        height * 0.40,
        width * 0.289,
        height * 0.383,
        0.18,
    );
    let shirt = subject
        * clamp01((y - height * 0.62) / (height * 0.18))
        * clamp01((height * 0.98 - y) / (height * 0.20));
    let hair =
        subject * head * if y < height * 0.43 { 1.0 } else { 0.0 } * clamp01((155.0 - luma) / 90.0);
    let beard_jaw = ellipse_mask(
        x,
        y,
        width * 0.5 + recipe.shift_x,
        height * 0.61,
        width * 0.205,
        height * 0.155,
        0.42,
    );
    let moustache = ellipse_mask(
        x,
        y,
        width * 0.5 + recipe.shift_x,
        height * 0.515,
        width * 0.135,
        height * 0.045,
        0.75,
    );
    let beard = subject * face * beard_jaw.max(moustache);
    let skin = subject * face * clamp01((205.0 - (luma - 145.0).abs()) / 145.0);

    pixel.r = ((pixel.r - 128.0) * recipe.contrast + 128.0) * (1.0 + recipe.exposure);
    pixel.g = ((pixel.g - 128.0) * recipe.contrast + 128.0) * (1.0 + recipe.exposure);
    pixel.b = ((pixel.b - 128.0) * recipe.contrast + 128.0) * (1.0 + recipe.exposure);

    let warmth = recipe.skin_warmth * 255.0;
    pixel.r += skin * warmth;
    pixel.g += skin * warmth * 0.18;
    pixel.b -= skin * warmth * 0.72;

    let shirt_luma = (luma / 118.0).clamp(0.55, 1.45);
    let shirt_mix = shirt * recipe.shirt_strength;
    pixel.r = mix(pixel.r, recipe.shirt_rgb[0] as f32 * shirt_luma, shirt_mix);
    pixel.g = mix(pixel.g, recipe.shirt_rgb[1] as f32 * shirt_luma, shirt_mix);
    pixel.b = mix(pixel.b, recipe.shirt_rgb[2] as f32 * shirt_luma, shirt_mix);

    let hair_luma = (luma / 70.0).clamp(0.55, 1.55);
    let hair_mix = hair * recipe.hair_strength;
    pixel.r = mix(pixel.r, recipe.hair_rgb[0] as f32 * hair_luma, hair_mix);
    pixel.g = mix(pixel.g, recipe.hair_rgb[1] as f32 * hair_luma, hair_mix);
    pixel.b = mix(pixel.b, recipe.hair_rgb[2] as f32 * hair_luma, hair_mix);

    if recipe.beard_strength > 0.03 {
        let beard_mix = beard * recipe.beard_strength;
        pixel.r = mix(pixel.r, 35.0, beard_mix);
        pixel.g = mix(pixel.g, 26.0, beard_mix);
        pixel.b = mix(pixel.b, 20.0, beard_mix);
    }

    pixel.r = pixel.r.clamp(0.0, 255.0);
    pixel.g = pixel.g.clamp(0.0, 255.0);
    pixel.b = pixel.b.clamp(0.0, 255.0);
    pixel.a = pixel.a.clamp(0.0, 255.0);
    pixel
}

fn encode_webp(image: &RgbaImage, quality: f32) -> Result<Vec<u8>, String> {
    let encoder = webp::Encoder::from_rgba(image.as_raw(), image.width(), image.height());
    let encoded = encoder.encode(quality);
    let bytes: &[u8] = encoded.as_ref();
    Ok(bytes.to_vec())
}

fn pixel_to_f(pixel: &Rgba<u8>) -> PixelF {
    PixelF {
        r: pixel.0[0] as f32,
        g: pixel.0[1] as f32,
        b: pixel.0[2] as f32,
        a: pixel.0[3] as f32,
    }
}

fn lerp_pixel(a: PixelF, b: PixelF, t: f32) -> PixelF {
    PixelF {
        r: mix(a.r, b.r, t),
        g: mix(a.g, b.g, t),
        b: mix(a.b, b.b, t),
        a: mix(a.a, b.a, t),
    }
}

fn ellipse_mask(x: f32, y: f32, cx: f32, cy: f32, rx: f32, ry: f32, softness: f32) -> f32 {
    let dist = ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2);
    let inner = 1.0 - softness;
    let outer = 1.0 + softness;
    clamp01((outer - dist) / (outer - inner))
}

fn portrait_seed(request: &PlayerPortraitRequest) -> u64 {
    stable_hash_bytes(
        format!(
            "{}|{}|{}|{}|{}|{}",
            GENERATOR_VERSION,
            request.player_id.trim().to_lowercase(),
            request
                .full_name
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_lowercase(),
            request
                .match_name
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_lowercase(),
            request
                .nationality
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_lowercase(),
            request
                .date_of_birth
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_lowercase()
        )
        .as_bytes(),
    )
}

fn cache_key(seed: u64, source_id: &str) -> String {
    format!(
        "{:016x}",
        stable_hash_bytes(format!("{GENERATOR_VERSION}:{seed}:{source_id}").as_bytes())
    )
}

fn rand_unit(state: &mut u64) -> f32 {
    let value = next_u64(state) >> 11;
    (value as f64 / ((1u64 << 53) - 1) as f64) as f32
}

fn rand_range(state: &mut u64, min: f32, max: f32) -> f32 {
    min + (max - min) * rand_unit(state)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 255.0) + 0.5) as u8
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_only_male_eligible_sources() {
        let sources = portrait_sources().expect("portrait sources should load");

        assert_eq!(sources.len(), 11);
        assert!(sources
            .iter()
            .all(|source| source.id != "chroma-03-northern-european"));
        assert!(sources.iter().all(|source| source.image.width() == SIZE));
        assert!(sources.iter().all(|source| source.image.height() == SIZE));
        assert!(SOURCE_BYTES.iter().all(|(_, bytes)| !bytes.is_empty()));
    }

    #[test]
    fn same_player_identity_produces_same_cache_key() {
        let request = PlayerPortraitRequest {
            player_id: "p-123".to_string(),
            full_name: Some("Mateus Ribeiro".to_string()),
            match_name: Some("Ribeiro".to_string()),
            nationality: Some("PT".to_string()),
            date_of_birth: Some("2003-04-12".to_string()),
        };

        let seed = portrait_seed(&request);
        let source = select_source(portrait_sources().unwrap(), seed);

        assert_eq!(seed, portrait_seed(&request));
        assert_eq!(cache_key(seed, source.id), cache_key(seed, source.id));
    }

    #[test]
    fn renders_transparent_webp_bytes() {
        let sources = portrait_sources().expect("portrait sources should load");
        let source = &sources[0];
        let recipe = build_recipe(42, source.id);
        let portrait = render_recipe(&source.image, &recipe);
        let bytes = encode_webp(&portrait, 88.0).expect("portrait should encode as webp");

        assert_eq!(portrait.width(), SIZE);
        assert_eq!(portrait.height(), SIZE);
        assert_eq!(portrait.get_pixel(0, 0).0[3], 0);
        assert!(bytes.starts_with(b"RIFF"));
        assert_eq!(&bytes[8..12], b"WEBP");
        assert!(bytes.len() > 1_000);
    }

    #[test]
    fn atomic_cache_write_publishes_final_file_without_temp_leftovers() {
        let temp = std::env::temp_dir().join(format!(
            "ofm-portrait-atomic-{}",
            stable_hash_bytes(format!("{:?}", std::time::SystemTime::now()).as_bytes())
        ));
        fs::create_dir_all(&temp).expect("temp cache dir should be created");
        let cache_path = temp.join("portrait.webp");

        write_cache_file_atomically(&cache_path, b"portrait-bytes")
            .expect("atomic cache write should succeed");

        assert_eq!(fs::read(&cache_path).unwrap(), b"portrait-bytes");
        let temp_leftovers = fs::read_dir(&temp)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
            .count();
        assert_eq!(temp_leftovers, 0);

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn batch_prewarm_writes_cache_without_image_payloads() {
        let temp = std::env::temp_dir().join(format!(
            "ofm-portrait-prewarm-{}",
            stable_hash_bytes(format!("{:?}", std::time::SystemTime::now()).as_bytes())
        ));
        let requests = vec![
            PlayerPortraitRequest {
                player_id: "p-1".to_string(),
                full_name: Some("Mateus Ribeiro".to_string()),
                match_name: Some("Ribeiro".to_string()),
                nationality: Some("PT".to_string()),
                date_of_birth: Some("2003-04-12".to_string()),
            },
            PlayerPortraitRequest {
                player_id: "p-2".to_string(),
                full_name: Some("Elliot Harper".to_string()),
                match_name: Some("Harper".to_string()),
                nationality: Some("GB-ENG".to_string()),
                date_of_birth: Some("1998-11-04".to_string()),
            },
        ];

        let result = prewarm_player_portraits_to_dir(&temp, &requests)
            .expect("batch prewarm should write cache files");

        assert_eq!(result.requested_count, 2);
        assert_eq!(result.generated_count, 2);
        assert_eq!(result.cached_count, 0);
        assert_eq!(result.failed_count, 0);
        assert!(result.render_ms > 0.0);
        assert!(result.elapsed_ms > 0.0);
        assert_eq!(result.records.len(), 2);
        assert!(result
            .records
            .iter()
            .all(|record| record.data_url.is_none()));
        assert!(result
            .records
            .iter()
            .all(|record| Path::new(&record.cache_path).exists()));

        let rerun = prewarm_player_portraits_to_dir(&temp, &requests)
            .expect("second batch prewarm should reuse cache files");
        assert_eq!(rerun.requested_count, 2);
        assert_eq!(rerun.generated_count, 0);
        assert_eq!(rerun.cached_count, 2);
        assert_eq!(rerun.failed_count, 0);

        let _ = fs::remove_dir_all(temp);
    }
}
