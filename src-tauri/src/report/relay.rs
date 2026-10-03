//! The game's side of the bug-report relay.
//!
//! `docs/api/bug-reports.md` is the contract and the server side wrote it; this module is the one
//! place that knows its shape. Everything here is pure except `submit`, which reads one file and
//! hands bytes to a [`Transport`] — so the envelope, the size ceiling and every status the relay
//! can answer with are tested without a network, and the real HTTP client is a thin shell around
//! them (`report::http`).

use std::path::Path;

use base64::Engine as _;

use super::REPORT_BUNDLE_FAILED;

/// The relay refuses a decoded ZIP above this, inclusive (`BUG_REPORT_MAX_BUNDLE_BYTES`).
///
/// Checked against the file's size on disk, before a byte of it is read: a bundle the relay will
/// refuse is not worth allocating, encoding and sending first. Base64 makes 16 MiB about 22.4 MiB,
/// which sits under the 24 MiB request ceiling, so the request ceiling needs no check of its own.
pub const MAX_BUNDLE_BYTES: u64 = 16 * 1024 * 1024;

/// The endpoint, relative to the relay's base URL.
const REPORTS_PATH: &str = "/api/v1/reports";

// What the player is told, by translation key. The relay's `error.message` is English written for
// a developer; the contract says to translate by `error.code`, and a player never sees the other.
pub const UPLOAD_CONSENT_REQUIRED: &str = "be.error.report.upload.consentRequired";
pub const UPLOAD_NOT_CONFIGURED: &str = "be.error.report.upload.notConfigured";
pub const UPLOAD_TOO_LARGE: &str = "be.error.report.upload.tooLarge";
pub const UPLOAD_RATE_LIMITED: &str = "be.error.report.upload.rateLimited";
pub const UPLOAD_BUSY: &str = "be.error.report.upload.busy";
pub const UPLOAD_REJECTED: &str = "be.error.report.upload.rejected";
pub const UPLOAD_SERVER_ERROR: &str = "be.error.report.upload.serverError";
/// No connection could be opened, so nothing was sent — offline, DNS, a refused connection.
pub const UPLOAD_UNREACHABLE: &str = "be.error.report.upload.unreachable";
/// The request may have arrived; the game cannot tell.
///
/// A dropped connection or a success the game cannot read is not a failure it can report as one:
/// the relay may already have stored the report, and v1 has no idempotency key, so "try again"
/// is a request for a second copy. The wording on the other side of this key says exactly that.
pub const UPLOAD_UNCONFIRMED: &str = "be.error.report.upload.unconfirmed";

/// Where reports go, already checked.
///
/// Only an `https://` base is accepted. The bundle is the player's logs and possibly their whole
/// career; sending it in clear text because someone typed `http://` into an override is not a
/// failure mode worth having, so a bad URL disables the upload rather than downgrading it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayEndpoint {
    reports_url: String,
}

impl RelayEndpoint {
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
        })
    }

    /// The runtime override when one is set, the build's default otherwise.
    ///
    /// An override that is set but invalid does **not** fall back to the default. Whoever set it
    /// meant to send reports somewhere else, and quietly sending them to the default instead is
    /// the one outcome they did not ask for.
    pub fn resolve(override_url: Option<&str>, compiled_default: Option<&str>) -> Option<Self> {
        // An exported-but-empty variable is how a shell says "unset", so it is treated as one.
        match override_url.map(str::trim).filter(|url| !url.is_empty()) {
            Some(url) => Self::from_base_url(url),
            None => compiled_default.and_then(Self::from_base_url),
        }
    }

    pub fn reports_url(&self) -> &str {
        &self.reports_url
    }
}

/// The JSON body of `POST /api/v1/reports`.
///
/// `consent.upload` is always `true` because nothing reaches this function without it — the
/// command refuses first. It is a statement the client makes, and this is the only place it can.
pub fn envelope(zip: &[u8], include_save: bool) -> String {
    serde_json::json!({
        "schema_version": 1,
        "consent": { "upload": true, "include_save": include_save },
        "bundle": {
            "encoding": "base64",
            "data": base64::engine::general_purpose::STANDARD.encode(zip),
        },
    })
    .to_string()
}

/// What came back, before anyone has decided what it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Eight Crockford Base32 characters, upper case — the only shape of code the relay issues.
pub fn is_reference_code(candidate: &str) -> bool {
    const CROCKFORD: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    candidate.len() == 8 && candidate.chars().all(|c| CROCKFORD.contains(c))
}

/// The reference code on success, or the translation key for what to tell the player.
pub fn interpret(response: &RelayResponse) -> Result<String, &'static str> {
    let body: Option<serde_json::Value> = serde_json::from_slice(&response.body).ok();

    if (200..300).contains(&response.status) {
        // Only 201 with a code the relay could have issued is a success. Anything else in the 2xx
        // range may still mean the report was stored, so it is neither recorded nor called a
        // failure the player should retry.
        let code = body
            .as_ref()
            .and_then(|json| json.get("code"))
            .and_then(serde_json::Value::as_str)
            .filter(|code| is_reference_code(code));
        return match (response.status, code) {
            (201, Some(code)) => Ok(code.to_owned()),
            _ => Err(UPLOAD_UNCONFIRMED),
        };
    }

    let error_code = body
        .as_ref()
        .and_then(|json| json.pointer("/error/code"))
        .and_then(serde_json::Value::as_str);
    Err(match error_code {
        Some("payload_too_large") => UPLOAD_TOO_LARGE,
        Some("rate_limited") => UPLOAD_RATE_LIMITED,
        Some("service_unavailable") => UPLOAD_BUSY,
        Some("internal_error") => UPLOAD_SERVER_ERROR,
        Some(
            "invalid_json"
            | "unsupported_media_type"
            | "validation_failed"
            | "forbidden"
            | "not_found"
            | "method_not_allowed",
        ) => UPLOAD_REJECTED,
        // No envelope, or a code this build does not know: a proxy or webserver page, which the
        // relay cannot normalise, or a newer relay. The status is all there is to go on.
        _ => match response.status {
            413 => UPLOAD_TOO_LARGE,
            429 => UPLOAD_RATE_LIMITED,
            503 => UPLOAD_BUSY,
            400..=499 => UPLOAD_REJECTED,
            _ => UPLOAD_SERVER_ERROR,
        },
    })
}

/// Why no response arrived. The detail is for the log, never for the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportFailure {
    /// Nothing left this machine: the connection was never established.
    Unreachable(String),
    /// The connection was open when it failed, so the relay may have the report.
    Interrupted(String),
}

/// Sends one request. The production one is `report::http::UreqTransport`.
pub trait Transport {
    /// POST `body` as JSON and return whatever status came back, error statuses included.
    fn post_json(&self, url: &str, body: Vec<u8>) -> Result<RelayResponse, TransportFailure>;
}

/// Upload the bundle at `zip_path` and return the code the relay issued for it.
pub fn submit(
    transport: &dyn Transport,
    endpoint: &RelayEndpoint,
    zip_path: &Path,
    include_save: bool,
) -> Result<String, &'static str> {
    let bytes = std::fs::metadata(zip_path)
        .map_err(|error| {
            log::error!("[report] could not read the bundle to upload: {error}");
            REPORT_BUNDLE_FAILED
        })?
        .len();
    // Before reading it. The save is never dropped to make it fit: the player chose it, and
    // sending less than they agreed to is not this function's call. They are told instead.
    if bytes > MAX_BUNDLE_BYTES {
        log::info!("[report] bundle of {bytes} bytes is over the relay's limit");
        return Err(UPLOAD_TOO_LARGE);
    }
    let zip = std::fs::read(zip_path).map_err(|error| {
        log::error!("[report] could not read the bundle to upload: {error}");
        REPORT_BUNDLE_FAILED
    })?;

    let response = transport
        .post_json(
            endpoint.reports_url(),
            envelope(&zip, include_save).into_bytes(),
        )
        .map_err(|failure| match failure {
            TransportFailure::Unreachable(detail) => {
                log::warn!("[report] relay unreachable: {detail}");
                UPLOAD_UNREACHABLE
            }
            TransportFailure::Interrupted(detail) => {
                log::warn!("[report] upload interrupted, outcome unknown: {detail}");
                UPLOAD_UNCONFIRMED
            }
        })?;
    let outcome = interpret(&response);
    match &outcome {
        Ok(code) => log::info!("[report] relay accepted the report as {code}"),
        Err(key) => log::warn!("[report] relay answered {}: {key}", response.status),
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const BASE: &str = "https://reports.example.test";

    fn endpoint() -> RelayEndpoint {
        RelayEndpoint::from_base_url(BASE).expect("a valid base")
    }

    fn response(status: u16, body: &str) -> RelayResponse {
        RelayResponse {
            status,
            body: body.as_bytes().to_vec(),
        }
    }

    fn error_body(code: &str) -> String {
        format!(r#"{{"error":{{"code":"{code}","message":"English for a developer."}}}}"#)
    }

    /// Answers every request with one canned result and remembers what it was sent.
    struct FakeTransport {
        answer: Result<RelayResponse, TransportFailure>,
        sent: RefCell<Vec<(String, Vec<u8>)>>,
    }

    impl FakeTransport {
        fn answering(answer: Result<RelayResponse, TransportFailure>) -> Self {
            Self {
                answer,
                sent: RefCell::new(Vec::new()),
            }
        }
    }

    impl Transport for FakeTransport {
        fn post_json(&self, url: &str, body: Vec<u8>) -> Result<RelayResponse, TransportFailure> {
            self.sent.borrow_mut().push((url.to_owned(), body));
            self.answer.clone()
        }
    }

    fn zip_of(dir: &Path, bytes: u64) -> std::path::PathBuf {
        let path = dir.join("report.zip");
        let file = std::fs::File::create(&path).expect("create");
        file.set_len(bytes).expect("size");
        path
    }

    /// Given an https base URL with a trailing slash,
    /// when it is accepted as the relay,
    /// then reports go to its `/api/v1/reports` with exactly one slash between.
    #[test]
    fn an_https_base_url_points_at_the_reports_endpoint() {
        let endpoint = RelayEndpoint::from_base_url("https://reports.example.test/").expect("ok");

        assert_eq!(
            endpoint.reports_url(),
            "https://reports.example.test/api/v1/reports"
        );
    }

    /// Given a base URL that is not https,
    /// when it is offered as the relay,
    /// then it is refused, so a player's logs and save are never sent in clear text.
    #[test]
    fn a_relay_that_is_not_https_is_refused() {
        for base in [
            "http://reports.example.test",
            "ftp://reports.example.test",
            "reports.example.test",
            "https://",
            "https:// spaced.example.test",
            "",
        ] {
            assert_eq!(RelayEndpoint::from_base_url(base), None, "{base:?}");
        }
    }

    /// Given both a runtime override and a compiled-in default,
    /// when the relay is resolved,
    /// then the override wins.
    #[test]
    fn a_runtime_override_wins_over_the_compiled_default() {
        let endpoint = RelayEndpoint::resolve(
            Some("https://staging.example.test"),
            Some("https://reports.example.test"),
        )
        .expect("resolved");

        assert_eq!(
            endpoint.reports_url(),
            "https://staging.example.test/api/v1/reports"
        );
    }

    /// Given no override,
    /// when the relay is resolved,
    /// then the compiled-in default is used — and with no default either, there is no relay.
    #[test]
    fn without_an_override_the_compiled_default_is_used() {
        assert_eq!(RelayEndpoint::resolve(None, Some(BASE)), Some(endpoint()));
        // An exported-but-empty variable is the shell's way of saying "unset".
        assert_eq!(
            RelayEndpoint::resolve(Some("  "), Some(BASE)),
            Some(endpoint())
        );
        assert_eq!(RelayEndpoint::resolve(None, None), None);
    }

    /// Given an override that is set but not https,
    /// when the relay is resolved,
    /// then there is no relay at all, rather than a silent fall back to the default.
    #[test]
    fn an_invalid_override_disables_the_upload_instead_of_falling_back() {
        assert_eq!(
            RelayEndpoint::resolve(Some("http://staging.example.test"), Some(BASE)),
            None
        );
    }

    /// Given a ZIP and the player's save choice,
    /// when the request body is built,
    /// then it has exactly the contract's keys, explicit upload consent, and padded base64.
    #[test]
    fn the_envelope_carries_consent_and_the_padded_base64_bundle() {
        // Two bytes, so standard Base64 needs padding — the contract asks for padded.
        let json: serde_json::Value =
            serde_json::from_str(&envelope(&[0xff, 0x00], true)).expect("json");

        assert_eq!(
            json,
            serde_json::json!({
                "schema_version": 1,
                "consent": { "upload": true, "include_save": true },
                "bundle": { "encoding": "base64", "data": "/wA=" }
            })
        );
    }

    /// Given strings of every shape,
    /// when each is checked as a reference code,
    /// then only eight upper-case Crockford Base32 characters pass.
    #[test]
    fn only_eight_crockford_characters_are_a_reference_code() {
        assert!(is_reference_code("7K2M9Q4R"));
        assert!(is_reference_code("0123ABCZ"));
        for bad in [
            "7K2M9Q4",
            "7K2M9Q4RX",
            "7k2m9q4r",
            "7K2M9Q4I",
            "7K2M9Q4L",
            "7K2M9Q4O",
            "7K2M9Q4U",
            "7K2M 9Q4",
            "",
        ] {
            assert!(!is_reference_code(bad), "{bad:?}");
        }
    }

    /// Given a 201 carrying a well-formed code,
    /// when the response is interpreted,
    /// then that code is the result.
    #[test]
    fn a_created_response_yields_its_reference_code() {
        assert_eq!(
            interpret(&response(201, r#"{"code":"7K2M9Q4R"}"#)),
            Ok("7K2M9Q4R".to_owned())
        );
    }

    /// Given a success status whose body is not a well-formed code,
    /// when the response is interpreted,
    /// then the outcome is "unconfirmed": the report may be stored, so it is neither a code to
    /// record nor a failure to retry.
    #[test]
    fn a_success_without_a_usable_code_is_unconfirmed() {
        for (status, body) in [
            (201, r#"{"code":"not-a-code"}"#),
            (201, "<html>gateway</html>"),
            (201, "{}"),
            (200, r#"{"code":"7K2M9Q4R"}"#),
        ] {
            assert_eq!(
                interpret(&response(status, body)),
                Err(UPLOAD_UNCONFIRMED),
                "{status} {body}"
            );
        }
    }

    /// Given each error the contract documents,
    /// when the response is interpreted,
    /// then the player is told by the stable `error.code`, never by the English message.
    #[test]
    fn each_documented_error_code_maps_to_a_translation_key() {
        for (status, code, key) in [
            (400, "invalid_json", UPLOAD_REJECTED),
            (413, "payload_too_large", UPLOAD_TOO_LARGE),
            (415, "unsupported_media_type", UPLOAD_REJECTED),
            (422, "validation_failed", UPLOAD_REJECTED),
            (429, "rate_limited", UPLOAD_RATE_LIMITED),
            (503, "service_unavailable", UPLOAD_BUSY),
            (500, "internal_error", UPLOAD_SERVER_ERROR),
            (403, "forbidden", UPLOAD_REJECTED),
            (404, "not_found", UPLOAD_REJECTED),
            (405, "method_not_allowed", UPLOAD_REJECTED),
        ] {
            assert_eq!(
                interpret(&response(status, &error_body(code))),
                Err(key),
                "{status} {code}"
            );
        }
    }

    /// Given an error with no JSON envelope — a proxy or webserver page in front of the app,
    /// which the contract says the relay cannot normalise —
    /// when the response is interpreted,
    /// then the status alone decides.
    #[test]
    fn an_error_without_the_json_envelope_is_judged_by_its_status() {
        for (status, key) in [
            (413, UPLOAD_TOO_LARGE),
            (429, UPLOAD_RATE_LIMITED),
            (503, UPLOAD_BUSY),
            (502, UPLOAD_SERVER_ERROR),
            (418, UPLOAD_REJECTED),
            (302, UPLOAD_SERVER_ERROR),
        ] {
            assert_eq!(
                interpret(&response(status, "<html>Request Entity Too Large</html>")),
                Err(key),
                "{status}"
            );
        }
    }

    /// Given a bundle within the relay's limit and a relay that answers 201,
    /// when it is submitted,
    /// then the envelope of exactly that file goes to the reports endpoint and the code comes back.
    #[test]
    fn a_bundle_within_the_limit_is_sent_and_its_code_returned() {
        let dir = tempfile::tempdir().expect("temp dir");
        let zip = dir.path().join("report.zip");
        std::fs::write(&zip, b"PK-not-really").expect("write");
        let transport = FakeTransport::answering(Ok(response(201, r#"{"code":"7K2M9Q4R"}"#)));

        let code = submit(&transport, &endpoint(), &zip, false);

        assert_eq!(code, Ok("7K2M9Q4R".to_owned()));
        let sent = transport.sent.borrow();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "https://reports.example.test/api/v1/reports");
        assert_eq!(
            String::from_utf8(sent[0].1.clone()).expect("utf-8"),
            envelope(b"PK-not-really", false)
        );
    }

    /// Given a bundle exactly at the relay's 16 MiB ceiling,
    /// when it is submitted,
    /// then it is sent — the ceiling is inclusive.
    #[test]
    fn a_bundle_exactly_at_the_ceiling_is_sent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let zip = zip_of(dir.path(), MAX_BUNDLE_BYTES);
        let transport = FakeTransport::answering(Ok(response(201, r#"{"code":"7K2M9Q4R"}"#)));

        assert_eq!(
            submit(&transport, &endpoint(), &zip, true),
            Ok("7K2M9Q4R".to_owned())
        );
    }

    /// Given a bundle one byte over the relay's ceiling,
    /// when it is submitted,
    /// then it is refused locally as too large and nothing is sent.
    #[test]
    fn a_bundle_over_the_ceiling_is_refused_before_anything_is_sent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let zip = zip_of(dir.path(), MAX_BUNDLE_BYTES + 1);
        let transport = FakeTransport::answering(Ok(response(201, r#"{"code":"7K2M9Q4R"}"#)));

        assert_eq!(
            submit(&transport, &endpoint(), &zip, true),
            Err(UPLOAD_TOO_LARGE)
        );
        assert!(transport.sent.borrow().is_empty(), "nothing may be sent");
    }

    /// Given a connection that drops before a response arrives,
    /// when a bundle is submitted,
    /// then the outcome is "unconfirmed", not a plain failure.
    #[test]
    fn a_dropped_connection_is_unconfirmed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let zip = zip_of(dir.path(), 10);
        let transport = FakeTransport::answering(Err(TransportFailure::Interrupted(
            "connection reset".to_owned(),
        )));

        assert_eq!(
            submit(&transport, &endpoint(), &zip, false),
            Err(UPLOAD_UNCONFIRMED)
        );
    }

    /// Given a relay that cannot be reached at all,
    /// when a bundle is submitted,
    /// then the player is told nothing was sent, which is safe to retry.
    #[test]
    fn an_unreachable_relay_is_reported_as_nothing_sent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let zip = zip_of(dir.path(), 10);
        let transport = FakeTransport::answering(Err(TransportFailure::Unreachable(
            "dns: no such host".to_owned(),
        )));

        assert_eq!(
            submit(&transport, &endpoint(), &zip, false),
            Err(UPLOAD_UNREACHABLE)
        );
    }
}
