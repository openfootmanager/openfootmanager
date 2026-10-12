//! The one HTTP client the game owns. What is sent and what an answer means lives in `hub`; the
//! `https://`-only rule is `hub::HubEndpoint`'s, which is what lets the tests below drive this
//! client against a plain socket on localhost.

use std::sync::OnceLock;
use std::time::Duration;

use super::hub::{HubResponse, Transport, TransportFailure};

/// More than any answer the server gives; a larger body is not the server.
const MAX_RESPONSE_BYTES: u64 = 64 * 1024;

/// Long enough for 25 MiB on a slow uplink. A total of a few seconds would fail every
/// player with an attached save on a home connection, and report it as a dropped upload.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    pub fn shared() -> &'static Self {
        static SHARED: OnceLock<UreqTransport> = OnceLock::new();
        SHARED.get_or_init(|| Self::with_proxy(ureq::Proxy::try_from_env()))
    }

    fn with_proxy(proxy: Option<ureq::Proxy>) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(TOTAL_TIMEOUT))
            // A 4xx/5xx is an answer from the server with an error code inside, not a failure to
            // talk to it; `hub::interpret` is what reads it.
            .http_status_as_error(false)
            .proxy(proxy)
            .user_agent(format!("OpenFootManager/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Self { agent }
    }
}

impl UreqTransport {
    fn read(
        mut response: ureq::http::Response<ureq::Body>,
    ) -> Result<HubResponse, TransportFailure> {
        let status = response.status().as_u16();
        let retry_after_seconds = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok());
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_BYTES)
            .read_to_vec()
            // The status line arrived, so the request did too; only the answer is missing.
            .map_err(|error| TransportFailure::Interrupted(error.to_string()))?;
        Ok(HubResponse {
            status,
            body,
            retry_after_seconds,
        })
    }
}

impl Transport for UreqTransport {
    fn get(&self, url: &str) -> Result<HubResponse, TransportFailure> {
        let response = self
            .agent
            .get(url)
            .header("Accept", "application/json")
            .call()
            .map_err(classify)?;
        Self::read(response)
    }

    fn post_multipart(
        &self,
        url: &str,
        game_version: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<HubResponse, TransportFailure> {
        let response = self
            .agent
            .post(url)
            .header("Content-Type", content_type)
            .header("Accept", "application/json")
            .header("X-OFM-Version", game_version)
            .send(&body[..])
            .map_err(classify)?;
        Self::read(response)
    }
}

/// Only failures before a connection exists (resolving, connecting, TLS) are certain not to have
/// sent the report; everything else is "may have arrived".
fn classify(error: ureq::Error) -> TransportFailure {
    use std::io::ErrorKind;
    use ureq::Timeout;

    let detail = error.to_string();
    let never_connected = match &error {
        ureq::Error::HostNotFound
        | ureq::Error::ConnectionFailed
        | ureq::Error::ConnectProxyFailed(_)
        | ureq::Error::Tls(_)
        | ureq::Error::Rustls(_)
        | ureq::Error::BadUri(_)
        | ureq::Error::InvalidProxyUrl => true,
        ureq::Error::Timeout(Timeout::Resolve | Timeout::Connect) => true,
        ureq::Error::Io(io) => matches!(
            io.kind(),
            ErrorKind::ConnectionRefused
                | ErrorKind::AddrNotAvailable
                | ErrorKind::NetworkUnreachable
                | ErrorKind::HostUnreachable
                | ErrorKind::NotConnected
        ),
        _ => false,
    };
    if never_connected {
        TransportFailure::Unreachable(detail)
    } else {
        TransportFailure::Interrupted(detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    struct Captured {
        request_line: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    impl Captured {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
        }
    }

    fn serve_once(
        status_line: &'static str,
        extra_headers: &'static str,
        body: &'static str,
    ) -> (String, std::thread::JoinHandle<Captured>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!(
            "http://{}/api/v1/bug-reports",
            listener.local_addr().expect("addr")
        );
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut request_line = String::new();
            reader.read_line(&mut request_line).expect("request line");
            let mut headers = Vec::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).expect("header");
                let line = line.trim_end();
                if line.is_empty() {
                    break;
                }
                let (key, value) = line.split_once(':').expect("header shape");
                headers.push((key.trim().to_owned(), value.trim().to_owned()));
            }
            let length: usize = headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.parse().expect("length"))
                .unwrap_or(0);
            let mut received = vec![0; length];
            reader.read_exact(&mut received).expect("body");
            let mut stream = stream;
            write!(
                stream,
                "{status_line}\r\nContent-Type: application/json\r\n{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .expect("respond");
            Captured {
                request_line: request_line.trim_end().to_owned(),
                headers,
                body: received,
            }
        });
        (url, handle)
    }

    /// Given a server that answers 201 with a reference,
    /// when the real client posts a multipart body,
    /// then the request carries the version header, the multipart content type and the body byte
    /// for byte, and the answer is handed back unjudged.
    #[test]
    fn posts_the_multipart_body_with_the_version_header() {
        let (url, server) = serve_once(
            "HTTP/1.1 201 Created",
            "",
            r#"{"reference":"OFM-7K2QX9DM"}"#,
        );

        let answer = UreqTransport::with_proxy(None)
            .post_multipart(
                &url,
                "0.3.0",
                "multipart/form-data; boundary=abc",
                b"--abc--\r\n".to_vec(),
            )
            .expect("a response");

        let captured = server.join().expect("server");
        assert_eq!(captured.request_line, "POST /api/v1/bug-reports HTTP/1.1");
        assert_eq!(
            captured.header("content-type"),
            Some("multipart/form-data; boundary=abc")
        );
        assert_eq!(captured.header("x-ofm-version"), Some("0.3.0"));
        assert_eq!(captured.header("accept"), Some("application/json"));
        assert_eq!(captured.body, b"--abc--\r\n");
        assert_eq!(answer.status, 201);
        assert_eq!(answer.body, br#"{"reference":"OFM-7K2QX9DM"}"#);
    }

    /// Given a 429 that carries `Retry-After`,
    /// when the real client posts,
    /// then the status, body and the wait in seconds all come back as a response, not a failure.
    #[test]
    fn an_error_status_is_a_response_with_its_retry_after() {
        let (url, server) = serve_once(
            "HTTP/1.1 429 Too Many Requests",
            "Retry-After: 120\r\n",
            r#"{"message":"Slow down.","code":"rate_limited"}"#,
        );

        let answer = UreqTransport::with_proxy(None)
            .post_multipart(
                &url,
                "0.3.0",
                "multipart/form-data; boundary=a",
                b"x".to_vec(),
            )
            .expect("a response");

        server.join().expect("server");
        assert_eq!(answer.status, 429);
        assert_eq!(answer.retry_after_seconds, Some(120));
        assert!(String::from_utf8_lossy(&answer.body).contains("rate_limited"));
    }

    /// Given a `Retry-After` that is a date rather than seconds,
    /// when the real client reads it,
    /// then no wait is claimed.
    #[test]
    fn a_retry_after_that_is_not_seconds_is_ignored() {
        let (url, server) = serve_once(
            "HTTP/1.1 429 Too Many Requests",
            "Retry-After: Wed, 21 Oct 2026 07:28:00 GMT\r\n",
            "{}",
        );

        let answer = UreqTransport::with_proxy(None)
            .get(&url)
            .expect("a response");

        server.join().expect("server");
        assert_eq!(answer.retry_after_seconds, None);
    }

    /// Given a server that answers the config read,
    /// when the real client gets it,
    /// then it is a GET that asks for JSON and sends no version header.
    #[test]
    fn gets_the_config_asking_for_json() {
        let (url, server) = serve_once("HTTP/1.1 200 OK", "", r#"{"enabled":true}"#);

        let answer = UreqTransport::with_proxy(None)
            .get(&url)
            .expect("a response");

        let captured = server.join().expect("server");
        assert!(captured.request_line.starts_with("GET "));
        assert_eq!(captured.header("accept"), Some("application/json"));
        assert_eq!(captured.header("x-ofm-version"), None);
        assert_eq!(answer.status, 200);
    }

    /// Given nothing listening at the server's address,
    /// when the real client posts,
    /// then it reports that the server was unreachable — nothing was sent.
    #[test]
    fn a_refused_connection_is_unreachable() {
        // Bind and drop, so the port is very likely closed for the moment of the request.
        let port = TcpListener::bind("127.0.0.1:0")
            .expect("bind")
            .local_addr()
            .expect("addr")
            .port();

        let failure = UreqTransport::with_proxy(None)
            .post_multipart(
                &format!("http://127.0.0.1:{port}/api/v1/bug-reports"),
                "0.3.0",
                "multipart/form-data; boundary=a",
                b"x".to_vec(),
            )
            .expect_err("nothing is listening");

        assert!(
            matches!(failure, TransportFailure::Unreachable(_)),
            "{failure:?}"
        );
    }
}
