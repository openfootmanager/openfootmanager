//! The game's side of OFMHub's bug-report endpoint (`docs/api/bug-reports.md`).
//!
//! Pure except `submit`, which reads the files it is given: the real HTTP client (`report::http`)
//! is a thin shell behind [`Transport`], so everything here is tested without a network.

use std::path::{Path, PathBuf};

use rand::RngExt;

/// The production OFMHub API base. Its `/config` says `enabled: false` until the maintainer turns
/// intake on, and the game then offers only the GitHub form.
pub const PRODUCTION_API_BASE: &str = "https://openfootmanager.com/api/v1";
pub const STAGING_API_BASE: &str = "https://staging.openfootmanager.com/api/v1";

/// Debug builds only: points one run somewhere else. A shipped game ignores it, so an
/// environment variable cannot redirect players' logs and saves.
pub const HUB_URL_VARIABLE: &str = "OFM_BUG_REPORT_URL";

const REPORTS_PATH: &str = "/bug-reports";
const CONFIG_PATH: &str = "/bug-reports/config";

pub const MAX_REPORT_JSON_BYTES: usize = 64 * 1024;

// What the player is told, by translation key. The endpoint's `message` is English written for a
// log; the contract says to branch on `code`, and the player never sees the other.
/// The bundle could not be built or read back for sending. Not the export's `bundleFailed`, whose
/// words are about a folder the player chose — the upload never asks for one.
pub const UPLOAD_PREPARE_FAILED: &str = "be.error.report.upload.prepareFailed";
pub const UPLOAD_CONSENT_REQUIRED: &str = "be.error.report.upload.consentRequired";
pub const UPLOAD_NOT_CONFIGURED: &str = "be.error.report.upload.notConfigured";
pub const UPLOAD_DISABLED: &str = "be.error.report.upload.disabled";
pub const UPLOAD_TOO_LARGE: &str = "be.error.report.upload.tooLarge";
pub const UPLOAD_TOO_MANY_FILES: &str = "be.error.report.upload.tooManyFiles";
pub const UPLOAD_RATE_LIMITED: &str = "be.error.report.upload.rateLimited";
pub const UPLOAD_CAPACITY: &str = "be.error.report.upload.capacity";
pub const UPLOAD_IMAGES_UNAVAILABLE: &str = "be.error.report.upload.imagesUnavailable";
pub const UPLOAD_REJECTED: &str = "be.error.report.upload.rejected";
pub const UPLOAD_SERVER_ERROR: &str = "be.error.report.upload.serverError";
pub const UPLOAD_UNREACHABLE: &str = "be.error.report.upload.unreachable";
/// The request may have arrived; the game cannot tell.
///
/// Sending the identical report again is safe: the server answers a byte-identical request from
/// the same address within 24 hours with the first report's reference. The wording on the other
/// side of this key says so.
pub const UPLOAD_UNCONFIRMED: &str = "be.error.report.upload.unconfirmed";
/// Attachment paths from the webview are refused until a native picker supplies them.
pub const UPLOAD_ATTACHMENTS_UNAVAILABLE: &str = "be.error.report.upload.attachmentsUnavailable";
/// Another report is still being sent; the held build belongs to that upload.
pub const UPLOAD_ALREADY_SENDING: &str = "be.error.report.upload.alreadySending";

/// What went wrong, for the frontend to branch on without matching translation keys.
/// `Other` carries a key from outside this module (a missing save, say).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UploadFailureKind {
    PrepareFailed,
    ConsentRequired,
    NotConfigured,
    Disabled,
    TooLarge,
    TooManyFiles,
    RateLimited,
    Capacity,
    ImagesUnavailable,
    Rejected,
    ServerError,
    Unreachable,
    Unconfirmed,
    AttachmentsUnavailable,
    AlreadySending,
    Other,
}

const KEYS_BY_KIND: [(&str, UploadFailureKind); 15] = [
    (UPLOAD_PREPARE_FAILED, UploadFailureKind::PrepareFailed),
    (UPLOAD_CONSENT_REQUIRED, UploadFailureKind::ConsentRequired),
    (UPLOAD_NOT_CONFIGURED, UploadFailureKind::NotConfigured),
    (UPLOAD_DISABLED, UploadFailureKind::Disabled),
    (UPLOAD_TOO_LARGE, UploadFailureKind::TooLarge),
    (UPLOAD_TOO_MANY_FILES, UploadFailureKind::TooManyFiles),
    (UPLOAD_RATE_LIMITED, UploadFailureKind::RateLimited),
    (UPLOAD_CAPACITY, UploadFailureKind::Capacity),
    (
        UPLOAD_IMAGES_UNAVAILABLE,
        UploadFailureKind::ImagesUnavailable,
    ),
    (UPLOAD_REJECTED, UploadFailureKind::Rejected),
    (UPLOAD_SERVER_ERROR, UploadFailureKind::ServerError),
    (UPLOAD_UNREACHABLE, UploadFailureKind::Unreachable),
    (UPLOAD_UNCONFIRMED, UploadFailureKind::Unconfirmed),
    (
        UPLOAD_ATTACHMENTS_UNAVAILABLE,
        UploadFailureKind::AttachmentsUnavailable,
    ),
    (UPLOAD_ALREADY_SENDING, UploadFailureKind::AlreadySending),
];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UploadFailure {
    pub kind: UploadFailureKind,
    pub key: String,
    pub retry_after_seconds: Option<u64>,
}

impl UploadFailure {
    pub fn key(key: &str) -> Self {
        let kind = KEYS_BY_KIND
            .iter()
            .find(|(known, _)| *known == key)
            .map_or(UploadFailureKind::Other, |(_, kind)| *kind);
        Self {
            kind,
            key: key.to_owned(),
            retry_after_seconds: None,
        }
    }

    fn rate_limited(retry_after_seconds: Option<u64>) -> Self {
        Self {
            retry_after_seconds,
            ..Self::key(UPLOAD_RATE_LIMITED)
        }
    }
}

/// Where reports go, already checked.
///
/// Only an `https://` base is accepted. The bundle is the player's logs and possibly their whole
/// career; sending it in clear text because someone typed `http://` into an override is not a
/// failure mode worth having, so a bad URL disables the upload rather than downgrading it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubEndpoint {
    reports_url: String,
    config_url: String,
}

impl HubEndpoint {
    pub fn from_base_url(base: &str) -> Option<Self> {
        let host_and_path = base.strip_prefix("https://")?.trim_end_matches('/');
        let host = host_and_path.split('/').next().unwrap_or_default();
        // A base is a host and perhaps a path prefix; a query, a fragment or whitespace means
        // something was pasted wrong, and guessing at what was meant is worse than refusing.
        if host.is_empty()
            || host_and_path
                .chars()
                .any(|c| c.is_whitespace() || c == '?' || c == '#')
        {
            return None;
        }
        Some(Self {
            reports_url: format!("https://{host_and_path}{REPORTS_PATH}"),
            config_url: format!("https://{host_and_path}{CONFIG_PATH}"),
        })
    }

    /// Release builds send to production; debug builds to staging, or to `override_url`.
    ///
    /// An override that is set but invalid does **not** fall back to the default: whoever set it
    /// meant to send reports elsewhere, and sending them to the default is the one outcome they
    /// did not ask for. An exported-but-empty variable is how a shell says "unset".
    pub fn resolve(override_url: Option<&str>, debug_build: bool) -> Option<Self> {
        let override_url = override_url
            .map(str::trim)
            .filter(|url| debug_build && !url.is_empty());
        match override_url {
            Some(url) => Self::from_base_url(url),
            None if debug_build => Self::from_base_url(STAGING_API_BASE),
            None => Self::from_base_url(PRODUCTION_API_BASE),
        }
    }

    fn reports_url(&self) -> &str {
        &self.reports_url
    }

    fn config_url(&self) -> &str {
        &self.config_url
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubResponse {
    pub status: u16,
    pub body: Vec<u8>,
    /// The `Retry-After` header in seconds, when it was present and numeric.
    pub retry_after_seconds: Option<u64>,
}

/// The detail is for the log, never for the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportFailure {
    Unreachable(String),
    /// The connection was open when it failed, so the server may have the report.
    Interrupted(String),
}

pub trait Transport {
    fn get(&self, url: &str) -> Result<HubResponse, TransportFailure>;

    fn post_multipart(
        &self,
        url: &str,
        game_version: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<HubResponse, TransportFailure>;
}

/// The limits and kill switch from `GET /bug-reports/config`. Never hard-coded: they come from the
/// server so they can change without a release.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct UploadConfig {
    pub enabled: bool,
    pub max_total_bytes: u64,
    pub max_bundle_bytes: u64,
    pub max_screenshot_bytes: u64,
    pub max_package_bytes: u64,
    pub max_files: usize,
    pub accepted_types: AcceptedTypes,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct AcceptedTypes {
    pub screenshots: Vec<String>,
    pub packages: Vec<String>,
}

fn io_failure_to_prepare(what: &str, error: &std::io::Error) -> UploadFailure {
    log::error!("[report] could not read {what} to upload: {error}");
    UploadFailure::key(UPLOAD_PREPARE_FAILED)
}

fn failure_of(failure: TransportFailure) -> UploadFailure {
    match failure {
        TransportFailure::Unreachable(detail) => {
            log::warn!("[report] report server unreachable: {detail}");
            UploadFailure::key(UPLOAD_UNREACHABLE)
        }
        TransportFailure::Interrupted(detail) => {
            log::warn!("[report] upload interrupted, outcome unknown: {detail}");
            UploadFailure::key(UPLOAD_UNCONFIRMED)
        }
    }
}

pub fn fetch_config(
    transport: &dyn Transport,
    endpoint: &HubEndpoint,
) -> Result<UploadConfig, UploadFailure> {
    let response = transport
        .get(endpoint.config_url())
        .map_err(failure_of)
        // Nothing has been sent yet, so even a dropped connection here is plain unavailability.
        .map_err(|failure| {
            if failure.key == UPLOAD_UNCONFIRMED {
                UploadFailure::key(UPLOAD_UNREACHABLE)
            } else {
                failure
            }
        })?;
    if response.status == 429 {
        return Err(UploadFailure::rate_limited(response.retry_after_seconds));
    }
    let config: UploadConfig = (response.status == 200)
        .then(|| serde_json::from_slice(&response.body).ok())
        .flatten()
        .ok_or_else(|| {
            log::warn!("[report] config answered {} unusably", response.status);
            UploadFailure::key(UPLOAD_SERVER_ERROR)
        })?;
    if config.enabled {
        Ok(config)
    } else {
        Err(UploadFailure::key(UPLOAD_DISABLED))
    }
}

/// Crockford Base32 is `0-9` and `A-Z` without `I L O U`; the `OFM-` prefix is part of the code.
pub fn is_reference(candidate: &str) -> bool {
    const CROCKFORD: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    candidate
        .strip_prefix("OFM-")
        .is_some_and(|rest| rest.len() == 8 && rest.chars().all(|c| CROCKFORD.contains(c)))
}

/// Whether the server accepts this as `X-OFM-Version`:
/// `^\d{1,4}\.\d{1,4}\.\d{1,4}(?:[-+][0-9A-Za-z.+-]{1,17})?$`.
pub fn is_accepted_game_version(version: &str) -> bool {
    let (core, suffix) = match version.find(['-', '+']) {
        Some(at) => (&version[..at], Some(&version[at + 1..])),
        None => (version, None),
    };
    let numbers: Vec<&str> = core.split('.').collect();
    let core_ok = numbers.len() == 3
        && numbers
            .iter()
            .all(|n| (1..=4).contains(&n.len()) && n.chars().all(|c| c.is_ascii_digit()));
    let suffix_ok = suffix.is_none_or(|s| {
        (1..=17).contains(&s.len())
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-'))
    });
    core_ok && suffix_ok
}

#[derive(Debug, Clone)]
pub struct UploadParts {
    pub report_json: String,
    pub bundle: PathBuf,
    pub screenshots: Vec<PathBuf>,
    pub packages: Vec<PathBuf>,
}

struct FilePart {
    field: &'static str,
    path: PathBuf,
    limit: u64,
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned())
        // Quotes and line breaks would end the header early.
        .replace(['"', '\r', '\n'], "_")
}

fn has_accepted_extension(path: &Path, accepted: &[String]) -> bool {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|ext| accepted.iter().any(|a| a.eq_ignore_ascii_case(&ext)))
}

/// A request the server will refuse is not worth building and uploading first.
fn plan_files(parts: &UploadParts, config: &UploadConfig) -> Result<Vec<FilePart>, UploadFailure> {
    let mut files = vec![FilePart {
        field: "bundle",
        path: parts.bundle.clone(),
        limit: config.max_bundle_bytes,
    }];
    for path in &parts.screenshots {
        if !has_accepted_extension(path, &config.accepted_types.screenshots) {
            return Err(UploadFailure::key(UPLOAD_REJECTED));
        }
        files.push(FilePart {
            field: "screenshots[]",
            path: path.clone(),
            limit: config.max_screenshot_bytes,
        });
    }
    for path in &parts.packages {
        if !has_accepted_extension(path, &config.accepted_types.packages) {
            return Err(UploadFailure::key(UPLOAD_REJECTED));
        }
        files.push(FilePart {
            field: "packages[]",
            path: path.clone(),
            limit: config.max_package_bytes,
        });
    }
    if files.len() > config.max_files {
        return Err(UploadFailure::key(UPLOAD_TOO_MANY_FILES));
    }
    let mut total = 0u64;
    for file in &files {
        let bytes = std::fs::metadata(&file.path)
            .map_err(|error| io_failure_to_prepare("an attachment", &error))?
            .len();
        if bytes > file.limit {
            return Err(UploadFailure::key(UPLOAD_TOO_LARGE));
        }
        total += bytes;
    }
    if total > config.max_total_bytes {
        return Err(UploadFailure::key(UPLOAD_TOO_LARGE));
    }
    Ok(files)
}

fn random_boundary() -> String {
    let mut rng = rand::rng();
    format!(
        "----ofm{:016x}{:016x}",
        rng.random::<u64>(),
        rng.random::<u64>()
    )
}

/// The first boundary `mint` offers that occurs in none of the parts.
fn boundary_for(parts: &[(Vec<u8>, &[u8])], mint: &mut dyn FnMut() -> String) -> String {
    loop {
        let candidate = mint();
        let needle = candidate.as_bytes();
        let clashes = parts
            .iter()
            .any(|(_, body)| body.windows(needle.len()).any(|window| window == needle));
        if !clashes {
            return candidate;
        }
    }
}

fn multipart_body(
    report_json: &str,
    files: &[(&FilePart, Vec<u8>)],
    mint_boundary: &mut dyn FnMut() -> String,
) -> (String, Vec<u8>) {
    let mut sections: Vec<(Vec<u8>, &[u8])> = vec![(
        b"Content-Disposition: form-data; name=\"report\"\r\nContent-Type: application/json\r\n\r\n"
            .to_vec(),
        report_json.as_bytes(),
    )];
    for (file, bytes) in files {
        let header = format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n\r\n",
            file.field,
            file_name_of(&file.path)
        );
        sections.push((header.into_bytes(), bytes.as_slice()));
    }
    let boundary = boundary_for(&sections, mint_boundary);
    let mut body = Vec::new();
    for (header, content) in &sections {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(header);
        body.extend_from_slice(content);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

fn error_code(body: &Option<serde_json::Value>) -> Option<&str> {
    body.as_ref()
        .and_then(|json| json.get("code"))
        .and_then(serde_json::Value::as_str)
}

fn interpret(response: &HubResponse) -> Result<String, UploadFailure> {
    let body: Option<serde_json::Value> = serde_json::from_slice(&response.body).ok();

    if (200..300).contains(&response.status) {
        // 200 is a byte-identical retry answered with the first report's reference; it is as good
        // as 201. Any other 2xx, or a reference the server could not have issued, may still mean
        // the report was stored, so it is neither recorded nor called a failure to retry.
        let reference = body
            .as_ref()
            .and_then(|json| json.get("reference"))
            .and_then(serde_json::Value::as_str)
            .filter(|reference| is_reference(reference));
        return match (response.status, reference) {
            (200 | 201, Some(reference)) => Ok(reference.to_owned()),
            _ => Err(UploadFailure::key(UPLOAD_UNCONFIRMED)),
        };
    }

    // Branch on status and code, never on the message. A body that is not JSON is the web
    // server's own page, in front of the app: the contract calls that "unavailable".
    let key = match (response.status, error_code(&body)) {
        (_, None) => UPLOAD_SERVER_ERROR,
        (413, Some("too_large")) => UPLOAD_TOO_LARGE,
        (422, Some("too_many_files")) => UPLOAD_TOO_MANY_FILES,
        (422, Some("invalid" | "blocked")) => UPLOAD_REJECTED,
        (429, Some("rate_limited")) => UPLOAD_RATE_LIMITED,
        (503, Some("disabled")) => UPLOAD_DISABLED,
        (503, Some("capacity")) => UPLOAD_CAPACITY,
        (503, Some("unavailable")) => UPLOAD_IMAGES_UNAVAILABLE,
        // `version_required`, `not_found`, `method_not_allowed` are bugs on our side; `server_error`
        // and anything newer are the server's. None is something the player can act on.
        (_, Some(_)) => match response.status {
            413 => UPLOAD_TOO_LARGE,
            429 => UPLOAD_RATE_LIMITED,
            _ => UPLOAD_SERVER_ERROR,
        },
    };
    Err(UploadFailure {
        retry_after_seconds: (key == UPLOAD_RATE_LIMITED)
            .then_some(response.retry_after_seconds)
            .flatten(),
        ..UploadFailure::key(key)
    })
}

/// The save is never dropped to make a bundle fit: the player chose it, and sending less than they
/// agreed to is not this function's call. They are told instead.
pub fn submit(
    transport: &dyn Transport,
    endpoint: &HubEndpoint,
    game_version: &str,
    parts: &UploadParts,
) -> Result<String, UploadFailure> {
    if !is_accepted_game_version(game_version) {
        log::error!("[report] this build's version would be refused by the server");
        return Err(UploadFailure::key(UPLOAD_SERVER_ERROR));
    }
    if parts.report_json.len() > MAX_REPORT_JSON_BYTES {
        return Err(UploadFailure::key(UPLOAD_TOO_LARGE));
    }
    let config = fetch_config(transport, endpoint)?;
    let planned = plan_files(parts, &config)?;
    let mut files = Vec::with_capacity(planned.len());
    for file in &planned {
        let bytes = std::fs::read(&file.path)
            .map_err(|error| io_failure_to_prepare("an attachment", &error))?;
        files.push((file, bytes));
    }
    let (content_type, body) = multipart_body(&parts.report_json, &files, &mut random_boundary);

    let response = transport
        .post_multipart(endpoint.reports_url(), game_version, &content_type, body)
        .map_err(failure_of)?;
    let outcome = interpret(&response);
    match &outcome {
        Ok(reference) => log::info!("[report] the server accepted the report as {reference}"),
        Err(failure) => log::warn!(
            "[report] the server answered {}: {}",
            response.status,
            failure.key
        ),
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const BASE: &str = "https://hub.example.test/api/v1";
    const CONFIG_JSON: &str = r#"{"enabled":true,"max_total_bytes":26214400,"max_bundle_bytes":20971520,"max_screenshot_bytes":5242880,"max_package_bytes":20971520,"max_files":6,"accepted_types":{"bundle":["zip"],"screenshots":["png","jpg","jpeg","webp"],"packages":["ofm"]}}"#;

    fn endpoint() -> HubEndpoint {
        HubEndpoint::from_base_url(BASE).expect("a valid base")
    }

    fn response(status: u16, body: &str) -> HubResponse {
        HubResponse {
            status,
            body: body.as_bytes().to_vec(),
            retry_after_seconds: None,
        }
    }

    fn error_body(code: &str) -> String {
        format!(r#"{{"message":"English for a log.","code":"{code}"}}"#)
    }

    const ACCEPTED: &str = r#"{"reference":"OFM-7K2QX9DM"}"#;

    struct Posted {
        url: String,
        game_version: String,
        content_type: String,
        body: Vec<u8>,
    }

    /// Answers the config read and the post with canned results and remembers what was sent.
    struct FakeTransport {
        config: Result<HubResponse, TransportFailure>,
        post: Result<HubResponse, TransportFailure>,
        gets: RefCell<Vec<String>>,
        posts: RefCell<Vec<Posted>>,
    }

    impl FakeTransport {
        fn new(post: Result<HubResponse, TransportFailure>) -> Self {
            Self {
                config: Ok(response(200, CONFIG_JSON)),
                post,
                gets: RefCell::new(Vec::new()),
                posts: RefCell::new(Vec::new()),
            }
        }

        fn accepting() -> Self {
            Self::new(Ok(response(201, ACCEPTED)))
        }
    }

    impl Transport for FakeTransport {
        fn get(&self, url: &str) -> Result<HubResponse, TransportFailure> {
            self.gets.borrow_mut().push(url.to_owned());
            self.config.clone()
        }

        fn post_multipart(
            &self,
            url: &str,
            game_version: &str,
            content_type: &str,
            body: Vec<u8>,
        ) -> Result<HubResponse, TransportFailure> {
            self.posts.borrow_mut().push(Posted {
                url: url.to_owned(),
                game_version: game_version.to_owned(),
                content_type: content_type.to_owned(),
                body,
            });
            self.post.clone()
        }
    }

    fn file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("write");
        path
    }

    fn sparse(dir: &Path, name: &str, bytes: u64) -> PathBuf {
        let path = dir.join(name);
        std::fs::File::create(&path)
            .and_then(|f| f.set_len(bytes))
            .expect("sparse file");
        path
    }

    fn parts_with(bundle: PathBuf) -> UploadParts {
        UploadParts {
            report_json: r#"{"what_happened":"it broke"}"#.to_owned(),
            bundle,
            screenshots: Vec::new(),
            packages: Vec::new(),
        }
    }

    fn failure(key: &str) -> UploadFailure {
        UploadFailure::key(key)
    }

    /// Given an https base URL with a trailing slash,
    /// when it is accepted as the endpoint,
    /// then reports and the config read go to their paths with exactly one slash between.
    #[test]
    fn an_https_base_url_points_at_the_report_and_config_paths() {
        let endpoint = HubEndpoint::from_base_url("https://hub.example.test/api/v1/").expect("ok");

        assert_eq!(
            endpoint.reports_url(),
            "https://hub.example.test/api/v1/bug-reports"
        );
        assert_eq!(
            endpoint.config_url(),
            "https://hub.example.test/api/v1/bug-reports/config"
        );
    }

    /// Given a base URL that is not https,
    /// when it is offered as the endpoint,
    /// then it is refused, so a player's logs and save are never sent in clear text.
    #[test]
    fn a_hub_that_is_not_https_is_refused() {
        for base in [
            "http://hub.example.test",
            "ftp://hub.example.test",
            "hub.example.test",
            "https://",
            "https:// spaced.example.test",
            "",
        ] {
            assert_eq!(HubEndpoint::from_base_url(base), None, "{base:?}");
        }
    }

    /// Given a debug build with a valid, an empty, an invalid and no override,
    /// when the endpoint is resolved,
    /// then a valid override wins, empty means unset, invalid disables instead of falling back,
    /// and unset means staging.
    #[test]
    fn a_debug_build_honours_the_override_and_defaults_to_staging() {
        assert_eq!(
            HubEndpoint::resolve(Some("https://other.example.test/api/v1"), true),
            HubEndpoint::from_base_url("https://other.example.test/api/v1")
        );
        assert_eq!(
            HubEndpoint::resolve(Some("  "), true),
            HubEndpoint::from_base_url(STAGING_API_BASE)
        );
        assert_eq!(
            HubEndpoint::resolve(None, true),
            HubEndpoint::from_base_url(STAGING_API_BASE)
        );
        assert_eq!(
            HubEndpoint::resolve(Some("http://other.example.test"), true),
            None
        );
    }

    /// Given a release build with an override in its environment,
    /// when the endpoint is resolved,
    /// then the override is ignored and reports go to production.
    #[test]
    fn a_release_build_ignores_the_override_and_uses_production() {
        let production = HubEndpoint::from_base_url(PRODUCTION_API_BASE);

        assert_eq!(
            HubEndpoint::resolve(Some("https://evil.example.test/api/v1"), false),
            production
        );
        assert_eq!(HubEndpoint::resolve(None, false), production);
    }

    /// Given both compiled-in bases,
    /// when they are checked,
    /// then each is a valid https endpoint and they differ.
    #[test]
    fn both_compiled_in_bases_are_valid_and_distinct() {
        assert!(HubEndpoint::from_base_url(PRODUCTION_API_BASE).is_some());
        assert!(HubEndpoint::from_base_url(STAGING_API_BASE).is_some());
        assert_ne!(PRODUCTION_API_BASE, STAGING_API_BASE);
    }

    /// Given version strings of every shape,
    /// when each is checked against the server's `X-OFM-Version` rule,
    /// then the documented forms pass and malformed or over-long ones do not.
    #[test]
    fn only_versions_the_server_accepts_pass() {
        for good in [
            "0.2.0",
            "0.3.0-nightly.20260921",
            "1.0.0+abc123",
            "1234.1.1",
        ] {
            assert!(is_accepted_game_version(good), "{good:?}");
        }
        for bad in [
            "",
            "1.0",
            "1.0.0.0",
            "12345.0.0",
            "1.0.0-",
            "1.0.0-aaaaaaaaaaaaaaaaaa",
            "1.0.0 beta",
            "v1.0.0",
        ] {
            assert!(!is_accepted_game_version(bad), "{bad:?}");
        }
    }

    /// Given the version this build is stamped with,
    /// when it is checked against the server's rule,
    /// then it is accepted, so no player is sent a 400 for a version string.
    #[test]
    fn this_builds_own_version_is_accepted() {
        assert!(is_accepted_game_version(env!("CARGO_PKG_VERSION")));
    }

    /// Given strings of every shape,
    /// when each is checked as a reference,
    /// then only `OFM-` plus eight upper-case Crockford characters pass.
    #[test]
    fn only_ofm_and_eight_crockford_characters_are_a_reference() {
        assert!(is_reference("OFM-7K2QX9DM"));
        assert!(is_reference("OFM-0123ABCZ"));
        for bad in [
            "7K2QX9DM",
            "OFM-7K2QX9D",
            "OFM-7K2QX9DMX",
            "OFM-7k2qx9dm",
            "OFM-7K2QX9DI",
            "OFM-7K2QX9DL",
            "OFM-7K2QX9DO",
            "OFM-7K2QX9DU",
            "ofm-7K2QX9DM",
            "OFM-7K2Q X9D",
            "",
        ] {
            assert!(!is_reference(bad), "{bad:?}");
        }
    }

    /// Given a 201 and a 200 (a deduplicated retry), both carrying a reference,
    /// when the responses are interpreted,
    /// then both yield that reference.
    #[test]
    fn created_and_deduplicated_responses_both_yield_their_reference() {
        for status in [200, 201] {
            assert_eq!(
                interpret(&response(status, ACCEPTED)),
                Ok("OFM-7K2QX9DM".to_owned()),
                "{status}"
            );
        }
    }

    /// Given a success status whose body is not a usable reference,
    /// when the response is interpreted,
    /// then the outcome is "unconfirmed": the report may be stored, so it is neither a reference
    /// to record nor a failure to retry blindly.
    #[test]
    fn a_success_without_a_usable_reference_is_unconfirmed() {
        for (status, body) in [
            (201, r#"{"reference":"not-a-reference"}"#),
            (201, "<html>gateway</html>"),
            (201, "{}"),
            (202, ACCEPTED),
            (200, r#"{"code":"7K2M9Q4R"}"#),
        ] {
            assert_eq!(
                interpret(&response(status, body)),
                Err(failure(UPLOAD_UNCONFIRMED)),
                "{status} {body}"
            );
        }
    }

    /// Given each error the contract documents,
    /// when the response is interpreted,
    /// then the player is told by status and `code`, never by the English message.
    #[test]
    fn each_documented_error_maps_to_a_translation_key() {
        for (status, code, key) in [
            (400, "version_required", UPLOAD_SERVER_ERROR),
            (404, "not_found", UPLOAD_SERVER_ERROR),
            (405, "method_not_allowed", UPLOAD_SERVER_ERROR),
            (413, "too_large", UPLOAD_TOO_LARGE),
            (422, "invalid", UPLOAD_REJECTED),
            (422, "too_many_files", UPLOAD_TOO_MANY_FILES),
            (422, "blocked", UPLOAD_REJECTED),
            (429, "rate_limited", UPLOAD_RATE_LIMITED),
            (503, "disabled", UPLOAD_DISABLED),
            (503, "capacity", UPLOAD_CAPACITY),
            (503, "unavailable", UPLOAD_IMAGES_UNAVAILABLE),
            (500, "server_error", UPLOAD_SERVER_ERROR),
        ] {
            assert_eq!(
                interpret(&response(status, &error_body(code))),
                Err(failure(key)),
                "{status} {code}"
            );
        }
    }

    /// Given a status with a code this build does not know,
    /// when the response is interpreted,
    /// then the status decides where it can, and otherwise it is a server error.
    #[test]
    fn an_unknown_code_falls_back_to_the_status() {
        for (status, key) in [
            (413, UPLOAD_TOO_LARGE),
            (429, UPLOAD_RATE_LIMITED),
            (422, UPLOAD_SERVER_ERROR),
            (503, UPLOAD_SERVER_ERROR),
        ] {
            assert_eq!(
                interpret(&response(status, &error_body("something_new"))),
                Err(failure(key)),
                "{status}"
            );
        }
    }

    /// Given an error with no JSON body — the web server's own page in front of the app,
    /// when the response is interpreted,
    /// then it is "unavailable" whatever the status, which the contract says falls back.
    #[test]
    fn a_non_json_error_is_a_server_error() {
        for status in [413, 429, 502, 503, 418] {
            assert_eq!(
                interpret(&response(status, "<html>Request Entity Too Large</html>")),
                Err(failure(UPLOAD_SERVER_ERROR)),
                "{status}"
            );
        }
    }

    /// Given a 429 with `Retry-After` and a 503 with the same header,
    /// when the responses are interpreted,
    /// then only the rate limit carries the wait.
    #[test]
    fn only_a_rate_limit_carries_the_retry_after() {
        let limited = HubResponse {
            retry_after_seconds: Some(3600),
            ..response(429, &error_body("rate_limited"))
        };
        let capacity = HubResponse {
            retry_after_seconds: Some(3600),
            ..response(503, &error_body("capacity"))
        };

        assert_eq!(
            interpret(&limited),
            Err(UploadFailure::rate_limited(Some(3600)))
        );
        assert_eq!(interpret(&capacity), Err(failure(UPLOAD_CAPACITY)));
    }

    /// Given a server with intake switched on,
    /// when its config is read,
    /// then the limits come from the server and the read goes to the config URL.
    #[test]
    fn the_config_carries_the_servers_limits() {
        let transport = FakeTransport::accepting();

        let config = fetch_config(&transport, &endpoint()).expect("config");

        assert_eq!(config.max_files, 6);
        assert_eq!(config.max_bundle_bytes, 20_971_520);
        assert_eq!(config.accepted_types.packages, ["ofm"]);
        assert_eq!(
            *transport.gets.borrow(),
            ["https://hub.example.test/api/v1/bug-reports/config"]
        );
    }

    /// Given a server with the kill switch on,
    /// when a report is submitted,
    /// then the player is told sending is off, and nothing is posted.
    #[test]
    fn the_kill_switch_stops_the_upload_before_anything_is_sent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut transport = FakeTransport::accepting();
        transport.config = Ok(response(200, &CONFIG_JSON.replace("true", "false")));

        let outcome = submit(
            &transport,
            &endpoint(),
            "0.3.0",
            &parts_with(file(dir.path(), "r.zip", b"PK")),
        );

        assert_eq!(outcome, Err(failure(UPLOAD_DISABLED)));
        assert!(transport.posts.borrow().is_empty());
    }

    /// Given a config read that is rate limited, unusable, or unreachable,
    /// when it is fetched,
    /// then each is a failure the player is told about by key, with the wait carried.
    #[test]
    fn a_config_that_cannot_be_read_is_a_failure() {
        let mut transport = FakeTransport::accepting();
        transport.config = Ok(HubResponse {
            retry_after_seconds: Some(30),
            ..response(429, &error_body("rate_limited"))
        });
        assert_eq!(
            fetch_config(&transport, &endpoint()),
            Err(UploadFailure::rate_limited(Some(30)))
        );

        transport.config = Ok(response(200, "<html></html>"));
        assert_eq!(
            fetch_config(&transport, &endpoint()),
            Err(failure(UPLOAD_SERVER_ERROR))
        );

        // Nothing was sent yet, so a dropped read is plain unavailability, not "unconfirmed".
        transport.config = Err(TransportFailure::Interrupted("reset".to_owned()));
        assert_eq!(
            fetch_config(&transport, &endpoint()),
            Err(failure(UPLOAD_UNREACHABLE))
        );
        transport.config = Err(TransportFailure::Unreachable("dns".to_owned()));
        assert_eq!(
            fetch_config(&transport, &endpoint()),
            Err(failure(UPLOAD_UNREACHABLE))
        );
    }

    /// Given a bundle, a screenshot and a package,
    /// when they are submitted,
    /// then one multipart POST carries the report, bundle, `screenshots[]` and `packages[]`
    /// parts with each file's bytes, the version header, and a boundary absent from the content.
    #[test]
    fn a_report_is_one_multipart_post_with_every_part() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut parts = parts_with(file(dir.path(), "report.zip", b"PK-bundle"));
        parts.screenshots = vec![file(dir.path(), "shot.png", b"PNG-bytes")];
        parts.packages = vec![file(dir.path(), "league.ofm", b"OFM-bytes")];
        let transport = FakeTransport::accepting();

        let outcome = submit(&transport, &endpoint(), "0.3.0-nightly.20260921", &parts);

        assert_eq!(outcome, Ok("OFM-7K2QX9DM".to_owned()));
        let posts = transport.posts.borrow();
        assert_eq!(posts.len(), 1);
        let post = &posts[0];
        assert_eq!(post.url, "https://hub.example.test/api/v1/bug-reports");
        assert_eq!(post.game_version, "0.3.0-nightly.20260921");
        let boundary = post
            .content_type
            .strip_prefix("multipart/form-data; boundary=")
            .expect("multipart content type");
        let body = String::from_utf8_lossy(&post.body);
        for expected in [
            "name=\"report\"",
            "{\"what_happened\":\"it broke\"}",
            "name=\"bundle\"; filename=\"report.zip\"",
            "PK-bundle",
            "name=\"screenshots[]\"; filename=\"shot.png\"",
            "PNG-bytes",
            "name=\"packages[]\"; filename=\"league.ofm\"",
            "OFM-bytes",
        ] {
            assert!(body.contains(expected), "missing {expected:?}");
        }
        assert_eq!(body.matches(&format!("--{boundary}\r\n")).count(), 4);
        assert!(body.ends_with(&format!("--{boundary}--\r\n")));
    }

    /// Given a retry the server recognises as a duplicate (200 with the first reference),
    /// when it is submitted,
    /// then the player gets that reference like any other.
    #[test]
    fn a_deduplicated_retry_returns_the_first_reference() {
        let dir = tempfile::tempdir().expect("temp dir");
        let transport = FakeTransport::new(Ok(response(200, ACCEPTED)));

        let outcome = submit(
            &transport,
            &endpoint(),
            "0.3.0",
            &parts_with(file(dir.path(), "r.zip", b"PK")),
        );

        assert_eq!(outcome, Ok("OFM-7K2QX9DM".to_owned()));
    }

    /// Given the server's file limits,
    /// when a submission is one over each (count, bundle, screenshot, package, total),
    /// then it is refused locally with the matching key and nothing is posted; at the limit it is
    /// sent — each ceiling is inclusive.
    #[test]
    fn the_servers_limits_are_enforced_before_sending() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut config: serde_json::Value = serde_json::from_str(CONFIG_JSON).expect("json");
        config["max_bundle_bytes"] = 100.into();
        config["max_screenshot_bytes"] = 50.into();
        config["max_package_bytes"] = 70.into();
        config["max_total_bytes"] = 200.into();
        config["max_files"] = 3.into();
        let run = |parts: &UploadParts| {
            let mut transport = FakeTransport::accepting();
            transport.config = Ok(response(200, &config.to_string()));
            let outcome = submit(&transport, &endpoint(), "0.3.0", parts);
            let posted = transport.posts.borrow().len();
            (outcome, posted)
        };

        let at_limit = UploadParts {
            screenshots: vec![sparse(dir.path(), "s.png", 50)],
            packages: vec![sparse(dir.path(), "p.ofm", 50)],
            ..parts_with(sparse(dir.path(), "b.zip", 100))
        };
        assert_eq!(run(&at_limit), (Ok("OFM-7K2QX9DM".to_owned()), 1));

        let big_bundle = parts_with(sparse(dir.path(), "bb.zip", 101));
        assert_eq!(run(&big_bundle), (Err(failure(UPLOAD_TOO_LARGE)), 0));

        let big_shot = UploadParts {
            screenshots: vec![sparse(dir.path(), "bs.png", 51)],
            ..parts_with(sparse(dir.path(), "b1.zip", 1))
        };
        assert_eq!(run(&big_shot), (Err(failure(UPLOAD_TOO_LARGE)), 0));

        let big_package = UploadParts {
            packages: vec![sparse(dir.path(), "bp.ofm", 71)],
            ..parts_with(sparse(dir.path(), "b2.zip", 1))
        };
        assert_eq!(run(&big_package), (Err(failure(UPLOAD_TOO_LARGE)), 0));

        let over_total = UploadParts {
            screenshots: vec![sparse(dir.path(), "t1.png", 50)],
            packages: vec![sparse(dir.path(), "t1.ofm", 60)],
            ..parts_with(sparse(dir.path(), "t.zip", 100))
        };
        assert_eq!(run(&over_total), (Err(failure(UPLOAD_TOO_LARGE)), 0));

        let too_many = UploadParts {
            screenshots: vec![
                sparse(dir.path(), "m1.png", 1),
                sparse(dir.path(), "m2.png", 1),
                sparse(dir.path(), "m3.png", 1),
            ],
            ..parts_with(sparse(dir.path(), "m.zip", 1))
        };
        assert_eq!(run(&too_many), (Err(failure(UPLOAD_TOO_MANY_FILES)), 0));
    }

    /// Given a screenshot or package whose extension the server does not accept,
    /// when it is submitted,
    /// then it is refused locally as rejected and nothing is posted.
    #[test]
    fn an_attachment_of_an_unaccepted_type_is_refused_locally() {
        let dir = tempfile::tempdir().expect("temp dir");
        let transport = FakeTransport::accepting();
        let parts = UploadParts {
            screenshots: vec![file(dir.path(), "shot.gif", b"GIF")],
            ..parts_with(file(dir.path(), "r.zip", b"PK"))
        };

        assert_eq!(
            submit(&transport, &endpoint(), "0.3.0", &parts),
            Err(failure(UPLOAD_REJECTED))
        );
        assert!(transport.posts.borrow().is_empty());
    }

    /// Given a connection that drops after the post began,
    /// when a report is submitted,
    /// then the outcome is "unconfirmed", not a plain failure.
    #[test]
    fn a_dropped_connection_during_the_post_is_unconfirmed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let transport = FakeTransport::new(Err(TransportFailure::Interrupted("reset".to_owned())));

        assert_eq!(
            submit(
                &transport,
                &endpoint(),
                "0.3.0",
                &parts_with(file(dir.path(), "r.zip", b"PK"))
            ),
            Err(failure(UPLOAD_UNCONFIRMED))
        );
    }

    /// Given a server that cannot be reached for the post,
    /// when a report is submitted,
    /// then the player is told nothing was sent.
    #[test]
    fn an_unreachable_server_is_reported_as_nothing_sent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let transport = FakeTransport::new(Err(TransportFailure::Unreachable("dns".to_owned())));

        assert_eq!(
            submit(
                &transport,
                &endpoint(),
                "0.3.0",
                &parts_with(file(dir.path(), "r.zip", b"PK"))
            ),
            Err(failure(UPLOAD_UNREACHABLE))
        );
    }

    /// Given a bundle file that is not there to read,
    /// when it is submitted,
    /// then the player is told the report could not be prepared, and nothing is posted.
    #[test]
    fn a_bundle_that_cannot_be_read_is_not_sent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let transport = FakeTransport::accepting();

        assert_eq!(
            submit(
                &transport,
                &endpoint(),
                "0.3.0",
                &parts_with(dir.path().join("missing.zip"))
            ),
            Err(failure(UPLOAD_PREPARE_FAILED))
        );
        assert!(transport.posts.borrow().is_empty());
    }

    /// Given a version string the server would refuse,
    /// when a report is submitted,
    /// then it fails before any request is made, since a 400 would only waste the player's turn.
    #[test]
    fn an_unacceptable_game_version_sends_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let transport = FakeTransport::accepting();

        assert_eq!(
            submit(
                &transport,
                &endpoint(),
                "not-a-version",
                &parts_with(file(dir.path(), "r.zip", b"PK"))
            ),
            Err(failure(UPLOAD_SERVER_ERROR))
        );
        assert!(transport.gets.borrow().is_empty());
    }

    /// Splits a multipart body on its boundary into (disposition header, content) pairs.
    fn parse_multipart(content_type: &str, body: &[u8]) -> Vec<(String, Vec<u8>)> {
        let boundary = content_type
            .strip_prefix("multipart/form-data; boundary=")
            .expect("boundary");
        let delimiter = format!("--{boundary}");
        let text = body;
        let mut parts = Vec::new();
        let mut rest = text;
        let first = delimiter.as_bytes();
        let find = |haystack: &[u8], needle: &[u8]| {
            haystack.windows(needle.len()).position(|w| w == needle)
        };
        let start = find(rest, first).expect("opening boundary");
        rest = &rest[start + first.len()..];
        loop {
            if rest.starts_with(b"--") {
                break;
            }
            rest = rest.strip_prefix(b"\r\n").expect("CRLF after boundary");
            let split = find(rest, b"\r\n\r\n").expect("header end");
            let header = String::from_utf8(rest[..split].to_vec()).expect("utf-8 header");
            rest = &rest[split + 4..];
            let end = find(rest, format!("\r\n{delimiter}").as_bytes()).expect("closing boundary");
            parts.push((header, rest[..end].to_vec()));
            rest = &rest[end + 2 + delimiter.len()..];
        }
        assert_eq!(rest, b"--\r\n", "the body ends with the closing boundary");
        parts
    }

    /// Given a report with binary content that resembles multipart syntax,
    /// when the body is built and parsed back by its boundary,
    /// then every part is recovered exactly, in order, under its own name.
    #[test]
    fn the_multipart_body_parses_back_to_the_same_parts() {
        let dir = tempfile::tempdir().expect("temp dir");
        let tricky: Vec<u8> = b"\r\n--not-the-boundary\r\n\x00\xff\r\n\r\n".to_vec();
        let mut parts = parts_with(file(dir.path(), "r.zip", &tricky));
        parts.screenshots = vec![file(dir.path(), "a.png", b"A")];
        parts.packages = vec![file(dir.path(), "b.ofm", b"B")];
        let transport = FakeTransport::accepting();

        submit(&transport, &endpoint(), "0.3.0", &parts).expect("accepted");

        let posts = transport.posts.borrow();
        let parsed = parse_multipart(&posts[0].content_type, &posts[0].body);
        let names: Vec<&str> = parsed
            .iter()
            .map(|(header, _)| {
                let from = header.find("name=\"").expect("name") + 6;
                &header[from..from + header[from..].find('"').expect("end")]
            })
            .collect();
        assert_eq!(names, ["report", "bundle", "screenshots[]", "packages[]"]);
        assert_eq!(parsed[0].1, parts.report_json.as_bytes());
        assert_eq!(parsed[1].1, tricky);
        assert_eq!(parsed[2].1, b"A");
        assert_eq!(parsed[3].1, b"B");
    }

    /// Given every translation key this module can fail with,
    /// when a failure is made from each,
    /// then it carries its own kind, so the frontend never has to match on a key — and a key from
    /// elsewhere is `Other`.
    #[test]
    fn every_upload_key_has_its_own_kind() {
        use UploadFailureKind::*;
        for (key, kind) in [
            (UPLOAD_PREPARE_FAILED, PrepareFailed),
            (UPLOAD_CONSENT_REQUIRED, ConsentRequired),
            (UPLOAD_NOT_CONFIGURED, NotConfigured),
            (UPLOAD_DISABLED, Disabled),
            (UPLOAD_TOO_LARGE, TooLarge),
            (UPLOAD_TOO_MANY_FILES, TooManyFiles),
            (UPLOAD_RATE_LIMITED, RateLimited),
            (UPLOAD_CAPACITY, Capacity),
            (UPLOAD_IMAGES_UNAVAILABLE, ImagesUnavailable),
            (UPLOAD_REJECTED, Rejected),
            (UPLOAD_SERVER_ERROR, ServerError),
            (UPLOAD_UNREACHABLE, Unreachable),
            (UPLOAD_UNCONFIRMED, Unconfirmed),
            (UPLOAD_ATTACHMENTS_UNAVAILABLE, AttachmentsUnavailable),
            (UPLOAD_ALREADY_SENDING, AlreadySending),
        ] {
            assert_eq!(UploadFailure::key(key).kind, kind, "{key}");
        }
        assert_eq!(
            UploadFailure::key("be.error.report.saveMissing").kind,
            Other
        );
    }

    /// Given a failure of each kind,
    /// when it is serialised for the frontend,
    /// then the kind is its camelCase name, which the TypeScript union spells out.
    #[test]
    fn the_kind_serialises_in_camel_case() {
        let json = serde_json::to_value(UploadFailure::rate_limited(Some(5))).expect("json");

        assert_eq!(json["kind"], "rateLimited");
        assert_eq!(json["key"], UPLOAD_RATE_LIMITED);
        assert_eq!(json["retry_after_seconds"], 5);
    }

    /// Given a boundary source whose first offer occurs inside a part,
    /// when the body is built,
    /// then that boundary is skipped and the next one is used.
    #[test]
    fn a_boundary_that_occurs_in_a_part_is_not_used() {
        let dir = tempfile::tempdir().expect("temp dir");
        let bundle = file(dir.path(), "r.zip", b"zip with ----clash inside");
        let planned = FilePart {
            field: "bundle",
            path: bundle,
            limit: 100,
        };
        let mut offers = ["----clash", "----fine"].into_iter();
        let mut mint = || offers.next().expect("an offer").to_owned();

        let (content_type, body) = multipart_body(
            "{}",
            &[(&planned, b"zip with ----clash inside".to_vec())],
            &mut mint,
        );

        assert_eq!(content_type, "multipart/form-data; boundary=----fine");
        assert!(String::from_utf8_lossy(&body).starts_with("------fine\r\n"));
    }

    /// Given a `report` part over the server's 64 KiB limit,
    /// when it is submitted,
    /// then the player is told it is too large — not that the server failed — and nothing is sent.
    #[test]
    fn an_oversized_report_part_is_too_large_not_a_server_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let transport = FakeTransport::accepting();
        let mut parts = parts_with(file(dir.path(), "r.zip", b"PK"));
        parts.report_json = "x".repeat(MAX_REPORT_JSON_BYTES + 1);

        assert_eq!(
            submit(&transport, &endpoint(), "0.3.0", &parts),
            Err(failure(UPLOAD_TOO_LARGE))
        );
        assert!(transport.gets.borrow().is_empty());
    }
}
