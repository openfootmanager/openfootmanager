use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use reqwest::blocking::Client as Http;
use serde::de::DeserializeOwned;
use serde_json::{json, Map, Value};

const PROTOCOL_VERSION: &str = "2025-03-26";

/// A tool call the server answered with an error.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    /// The `be.error.*` key, or `None` for a message private to the MCP layer.
    pub key: Option<String>,
    pub params: Map<String, Value>,
    pub message: String,
}

#[derive(Debug)]
pub enum CallError {
    /// The server refused the call.
    Refused(Refusal),
    /// The call was sent and no answer came in time. The call may have run, so the client does
    /// not send it again: read the state back and decide.
    TimedOut { tool: String },
    /// The request could not be delivered or the reply could not be read.
    Transport(String),
    /// The reply was not the shape the typed client expects.
    Shape { tool: String, detail: String },
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(refusal) => write!(
                f,
                "refused ({}): {}",
                refusal.key.as_deref().unwrap_or("no key"),
                refusal.message
            ),
            Self::TimedOut { tool } => write!(f, "`{tool}` timed out after it was sent"),
            Self::Transport(detail) => write!(f, "transport: {detail}"),
            Self::Shape { tool, detail } => {
                write!(f, "`{tool}` replied in an unexpected shape: {detail}")
            }
        }
    }
}

impl std::error::Error for CallError {}

/// What a call left on the wire, for the failure bundle.
#[derive(Debug, Clone)]
pub struct Exchange {
    pub request: Value,
    pub response: Option<Value>,
}

pub struct Client {
    http: Http,
    url: String,
    session: Option<String>,
    next_id: AtomicU64,
    transcript: Mutex<Vec<Exchange>>,
}

impl Client {
    /// Opens a session: `initialize`, then the `initialized` notification.
    pub fn connect(url: &str, timeout: Duration) -> Result<Self, CallError> {
        let http = Http::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| CallError::Transport(e.to_string()))?;
        let mut client = Self {
            http,
            url: url.to_string(),
            session: None,
            next_id: AtomicU64::new(1),
            transcript: Mutex::new(Vec::new()),
        };
        let (_, session) = client.rpc(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "mcp-e2e", "version": "0.1.0"}
            }),
        )?;
        client.session = session;
        client.notify("notifications/initialized")?;
        Ok(client)
    }

    /// The same session with a different timeout for each call.
    pub fn with_timeout(mut self, timeout: Duration) -> Result<Self, CallError> {
        self.http = Http::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| CallError::Transport(e.to_string()))?;
        Ok(self)
    }

    /// `tools/list`, as the server reports it.
    pub fn list_tools(&self) -> Result<Vec<Value>, CallError> {
        let (result, _) = self.rpc("tools/list", json!({}))?;
        result["tools"]
            .as_array()
            .cloned()
            .ok_or_else(|| CallError::Shape {
                tool: "tools/list".to_string(),
                detail: result.to_string(),
            })
    }

    /// Calls a tool and returns its structured success, or the refusal.
    pub fn call<T: DeserializeOwned>(&self, tool: &str, arguments: Value) -> Result<T, CallError> {
        let structured = self.call_value(tool, arguments)?;
        serde_json::from_value(structured.clone()).map_err(|e| CallError::Shape {
            tool: tool.to_string(),
            detail: format!("{e}: {structured}"),
        })
    }

    /// Calls a tool and returns its `structuredContent` untyped.
    pub fn call_value(&self, tool: &str, arguments: Value) -> Result<Value, CallError> {
        let (result, _) = self
            .rpc("tools/call", json!({"name": tool, "arguments": arguments}))
            .map_err(|error| match error {
                CallError::TimedOut { .. } => CallError::TimedOut {
                    tool: tool.to_string(),
                },
                other => other,
            })?;
        let structured = result.get("structuredContent").cloned();
        if result["isError"].as_bool() == Some(true) {
            return Err(CallError::Refused(refusal_of(&result, structured)));
        }
        structured.ok_or_else(|| CallError::Shape {
            tool: tool.to_string(),
            detail: format!("no structuredContent: {result}"),
        })
    }

    /// Every request sent and what came back, oldest first.
    pub fn transcript(&self) -> Vec<Exchange> {
        self.transcript
            .lock()
            .map(|t| t.clone())
            .unwrap_or_default()
    }

    fn rpc(&self, method: &str, params: Value) -> Result<(Value, Option<String>), CallError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let (body, session) = self.post(&request).inspect_err(|_| {
            self.record(request.clone(), None);
        })?;
        let Some(reply) = last_event_data(&body) else {
            self.record(request, None);
            return Err(CallError::Transport(format!(
                "no JSON-RPC message in the reply to `{method}`: {body}"
            )));
        };
        self.record(request, Some(reply.clone()));
        if let Some(error) = reply.get("error") {
            return Err(CallError::Transport(format!("`{method}` failed: {error}")));
        }
        Ok((reply["result"].clone(), session))
    }

    fn notify(&self, method: &str) -> Result<(), CallError> {
        let request = json!({"jsonrpc": "2.0", "method": method});
        self.post(&request)?;
        self.record(request, None);
        Ok(())
    }

    fn post(&self, request: &Value) -> Result<(String, Option<String>), CallError> {
        let mut builder = self
            .http
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(request);
        if let Some(session) = &self.session {
            builder = builder.header("Mcp-Session-Id", session);
        }
        let response = builder.send().map_err(transport_error)?;
        let session = response
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let status = response.status();
        let body = response.text().map_err(transport_error)?;
        if !status.is_success() {
            return Err(CallError::Transport(format!("HTTP {status}: {body}")));
        }
        Ok((body, session))
    }

    fn record(&self, request: Value, response: Option<Value>) {
        if let Ok(mut transcript) = self.transcript.lock() {
            transcript.push(Exchange { request, response });
        }
    }
}

fn transport_error(error: reqwest::Error) -> CallError {
    if error.is_timeout() {
        CallError::TimedOut {
            tool: String::new(),
        }
    } else {
        CallError::Transport(error.to_string())
    }
}

fn refusal_of(result: &Value, structured: Option<Value>) -> Refusal {
    let error = structured.as_ref().map(|s| &s["error"]);
    let text = result["content"][0]["text"].as_str().unwrap_or_default();
    Refusal {
        key: error.and_then(|e| e["key"].as_str()).map(str::to_string),
        params: error
            .and_then(|e| e["params"].as_object().cloned())
            .unwrap_or_default(),
        message: error
            .and_then(|e| e["message"].as_str())
            .unwrap_or(text)
            .to_string(),
    }
}

/// The JSON of the last `data:` line of a server-sent-events body, or the body itself when the
/// server answered with plain JSON.
fn last_event_data(body: &str) -> Option<Value> {
    let from_events = body
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .filter_map(|data| serde_json::from_str::<Value>(data.trim()).ok())
        .next_back();
    from_events.or_else(|| serde_json::from_str(body.trim()).ok())
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use std::thread;

    use super::*;

    /// Serves canned replies to the first requests, counting every request that arrives.
    fn serve(replies: Vec<Option<String>>) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let seen = Arc::new(AtomicUsize::new(0));
        let counter = seen.clone();
        thread::spawn(move || {
            let mut replies = replies.into_iter();
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                counter.fetch_add(1, Ordering::SeqCst);
                let mut buffer = [0_u8; 8192];
                let _ = stream.read(&mut buffer);
                match replies.next() {
                    Some(Some(body)) => {
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nMcp-Session-Id: s1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Some(None) => thread::sleep(Duration::from_secs(3)),
                    None => return,
                }
            }
        });
        (url, seen)
    }

    fn event(result: Value) -> Option<String> {
        Some(format!(
            "data: \ndata: {}\n\n",
            json!({"jsonrpc": "2.0", "id": 1, "result": result})
        ))
    }

    /// Given a server that answers `initialize`, the notification and a refused tool call
    /// When the client calls the tool
    /// Then the refusal carries the key, the parameters and the message from the structure.
    #[test]
    fn a_refusal_is_read_from_the_structured_error() {
        let refusal = json!({
            "isError": true,
            "content": [{"type": "text", "text": "No active game session."}],
            "structuredContent": {"error": {
                "key": "be.error.noActiveGameSession", "params": {}, "message": "No active game session."
            }}
        });
        let (url, _) = serve(vec![event(json!({})), Some(String::new()), event(refusal)]);
        let client = Client::connect(&url, Duration::from_secs(2)).unwrap();

        let outcome: Result<Value, _> = client.call("info_game_summary", json!({}));

        match outcome {
            Err(CallError::Refused(refusal)) => {
                assert_eq!(refusal.key.as_deref(), Some("be.error.noActiveGameSession"));
                assert_eq!(refusal.message, "No active game session.");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    /// Given a tool call the server never answers
    /// When the client gives up
    /// Then it reports the timeout and has sent that call exactly once.
    #[test]
    fn a_timed_out_call_is_not_sent_again() {
        let (url, seen) = serve(vec![event(json!({})), Some(String::new()), None]);
        let client = Client::connect(&url, Duration::from_millis(400)).unwrap();
        let before = seen.load(Ordering::SeqCst);

        let outcome: Result<Value, _> =
            client.call("transfer_make_bid", json!({"player_id": "p1", "fee": 10}));

        match outcome {
            Err(CallError::TimedOut { tool }) => assert_eq!(tool, "transfer_make_bid"),
            other => panic!("expected a timeout, got {other:?}"),
        }
        thread::sleep(Duration::from_millis(900));
        assert_eq!(
            seen.load(Ordering::SeqCst) - before,
            1,
            "the call must be sent once"
        );
        let last = client
            .transcript()
            .pop()
            .expect("the timed-out call is in the transcript");
        assert_eq!(last.request["params"]["name"], "transfer_make_bid");
        assert!(last.response.is_none(), "no reply arrived");
    }

    /// Given a success reply
    /// When the client reads it as a typed result
    /// Then the structure is deserialized without touching the text.
    #[test]
    fn a_success_is_read_as_the_typed_struct() {
        let success = json!({
            "isError": false,
            "content": [{"type": "text", "text": "## Staff Hired\n\nStaff member s1 hired."}],
            "structuredContent": {"staff_id": "s1"}
        });
        let (url, _) = serve(vec![event(json!({})), Some(String::new()), event(success)]);
        let client = Client::connect(&url, Duration::from_secs(2)).unwrap();

        let hired: mcp_results::club::StaffHired = client
            .call("staff_hire", json!({"staff_id": "s1"}))
            .unwrap();

        assert_eq!(hired.staff_id, "s1");
    }
}
