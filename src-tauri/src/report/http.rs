//! The one HTTP client the game owns, and the only code here that touches the network.
//!
//! Everything about *what* is sent and what an answer means lives in `relay`; this is the shell
//! that moves bytes. It refuses nothing on its own — the `https://`-only rule belongs to
//! `relay::RelayEndpoint`, which is what lets the test below drive the real client against a
//! plain socket on localhost.

use std::sync::OnceLock;
use std::time::Duration;

use super::relay::{RelayResponse, Transport, TransportFailure};

/// More than any answer the relay gives: a code, or an error envelope. A body larger than this is
/// not one of those, and reading it whole would be allocating for something that is not the relay.
const MAX_RESPONSE_BYTES: u64 = 64 * 1024;

/// Long enough for ~22 MiB of base64 on a slow uplink. A total of a few seconds would fail every
/// player with an attached save on a home connection, and report it as a dropped upload.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    /// The shared client, built once — connection pool, TLS roots and all.
    pub fn shared() -> &'static Self {
        static SHARED: OnceLock<UreqTransport> = OnceLock::new();
        SHARED.get_or_init(|| Self::with_proxy(ureq::Proxy::try_from_env()))
    }

    fn with_proxy(proxy: Option<ureq::Proxy>) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(TOTAL_TIMEOUT))
            // A 4xx/5xx is an answer from the relay with an error code inside, not a failure to
            // talk to it; `relay::interpret` is what reads it.
            .http_status_as_error(false)
            .proxy(proxy)
            .user_agent(format!("OpenFootManager/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Self { agent }
    }
}

impl Transport for UreqTransport {
    fn post_json(&self, url: &str, body: Vec<u8>) -> Result<RelayResponse, TransportFailure> {
        let mut response = self
            .agent
            .post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .send(&body[..])
            .map_err(classify)?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_BYTES)
            .read_to_vec()
            // The status line arrived, so the request did too; only the answer is missing.
            .map_err(|error| TransportFailure::Interrupted(error.to_string()))?;
        Ok(RelayResponse { status, body })
    }
}

/// Whether the request can have reached the relay before this went wrong.
///
/// Only failures that happen before a connection exists — resolving the host, opening the socket,
/// the TLS handshake — are certain not to have sent the report. Everything else is treated as
/// "may have arrived", because telling a player to retry something that did arrive makes a
/// duplicate in the maintainer's inbox.
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

    /// What one request looked like on the wire.
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

    /// Serve exactly one request with `status` and `body`, handing back what was received.
    fn serve_once(
        status_line: &'static str,
        body: &'static str,
    ) -> (String, std::thread::JoinHandle<Captured>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!(
            "http://{}/api/v1/reports",
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
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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

    /// Given a relay that answers 201 with a code,
    /// when the real client posts an envelope,
    /// then it is a JSON POST that asks for JSON back, the body arrives byte for byte, and the
    /// answer is handed back unjudged.
    #[test]
    fn posts_the_body_as_json_and_returns_the_answer() {
        let (url, server) = serve_once("HTTP/1.1 201 Created", r#"{"code":"7K2M9Q4R"}"#);

        let answer = UreqTransport::with_proxy(None)
            .post_json(&url, br#"{"schema_version":1}"#.to_vec())
            .expect("a response");

        let captured = server.join().expect("server");
        assert_eq!(captured.request_line, "POST /api/v1/reports HTTP/1.1");
        assert_eq!(captured.header("content-type"), Some("application/json"));
        assert_eq!(captured.header("accept"), Some("application/json"));
        assert_eq!(captured.body, br#"{"schema_version":1}"#);
        assert_eq!(answer.status, 201);
        assert_eq!(answer.body, br#"{"code":"7K2M9Q4R"}"#);
    }

    /// Given a relay that answers with an error status,
    /// when the real client posts,
    /// then the status and its JSON body come back as a response, not as a transport failure —
    /// the error code inside is what the player is told.
    #[test]
    fn an_error_status_is_a_response_not_a_failure() {
        let (url, server) = serve_once(
            "HTTP/1.1 429 Too Many Requests",
            r#"{"error":{"code":"rate_limited","message":"Slow down."}}"#,
        );

        let answer = UreqTransport::with_proxy(None)
            .post_json(&url, b"{}".to_vec())
            .expect("a response");

        server.join().expect("server");
        assert_eq!(answer.status, 429);
        assert!(String::from_utf8_lossy(&answer.body).contains("rate_limited"));
    }

    /// Given nothing listening at the relay's address,
    /// when the real client posts,
    /// then it reports that the relay was unreachable — nothing was sent.
    #[test]
    fn a_refused_connection_is_unreachable() {
        // Bind and drop, so the port is very likely closed for the moment of the request.
        let port = TcpListener::bind("127.0.0.1:0")
            .expect("bind")
            .local_addr()
            .expect("addr")
            .port();

        let failure = UreqTransport::with_proxy(None)
            .post_json(
                &format!("http://127.0.0.1:{port}/api/v1/reports"),
                b"{}".to_vec(),
            )
            .expect_err("nothing is listening");

        assert!(
            matches!(failure, TransportFailure::Unreachable(_)),
            "{failure:?}"
        );
    }
}
