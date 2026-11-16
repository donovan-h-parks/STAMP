//! A Model Context Protocol server on this computer only, through which
//! Claude Code (or another MCP client) uses the assistant's tools.
//!
//! It speaks MCP's "Streamable HTTP" transport in its simplest form: each
//! JSON-RPC message is POSTed to `/mcp` and answered with plain JSON (the
//! transport allows that instead of an event stream). It listens on
//! 127.0.0.1 only, at a port chosen by the system, and refuses requests
//! without the access token made for this run, requests addressed to
//! another host name (DNS rebinding) and requests from web pages (an
//! `Origin` other than this computer).
//!
//! Tool calls are handed to the window as [`Request`]s, which it answers
//! between frames; the server thread waits for the answer.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::tools::{self, Content, Outcome};
use super::Request;

/// Protocol versions understood; the first is offered to clients that ask
/// for one not listed.
const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-11-25", "2025-03-26"];
/// Longest wait for the window to answer a tool call: long analyses with
/// many permutations take minutes.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const MAX_HEADER_BYTES: usize = 32 * 1024;
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

/// The running server. Stopped when dropped.
pub struct Server {
    port: u16,
    token: String,
    receiver: Receiver<Request>,
    stop: Arc<AtomicBool>,
    /// Number of clients that have connected (sent `initialize`).
    clients: Arc<AtomicUsize>,
}

impl Server {
    /// Start listening. `wake` is called whenever a tool call arrives, to
    /// get the window to answer it (`egui::Context::request_repaint`).
    pub fn start(wake: impl Fn() + Send + Sync + 'static) -> std::io::Result<Server> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        let token = new_token();
        let (sender, receiver) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let clients = Arc::new(AtomicUsize::new(0));
        let shared = Arc::new(Shared {
            port,
            token: token.clone(),
            sender: std::sync::Mutex::new(sender),
            wake: Box::new(wake),
            clients: clients.clone(),
        });
        let stopped = stop.clone();
        std::thread::Builder::new()
            .name("stamp-mcp".to_string())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stopped.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    let shared = shared.clone();
                    let _ = std::thread::Builder::new()
                        .name("stamp-mcp-connection".to_string())
                        .spawn(move || serve_connection(stream, &shared));
                }
            })?;
        Ok(Server {
            port,
            token,
            receiver,
            stop,
            clients,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// The address clients connect to.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/mcp", self.port)
    }

    /// The access token clients send as `Authorization: Bearer <token>`.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// The next tool call waiting for an answer, if any.
    pub fn next_request(&self) -> Option<Request> {
        self.receiver.try_recv().ok()
    }

    /// Number of times a client has connected.
    pub fn clients(&self) -> usize {
        self.clients.load(Ordering::SeqCst)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the listening thread so that it sees it should stop.
        let _ = TcpStream::connect_timeout(
            &(std::net::Ipv4Addr::LOCALHOST, self.port).into(),
            Duration::from_millis(200),
        );
    }
}

struct Shared {
    port: u16,
    token: String,
    sender: std::sync::Mutex<Sender<Request>>,
    wake: Box<dyn Fn() + Send + Sync>,
    clients: Arc<AtomicUsize>,
}

/// An access token that cannot be guessed: 128 bits from the hash keys the
/// standard library draws from the operating system's random source.
fn new_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut token = String::new();
    for i in 0..2u64 {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u64(i);
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
        );
        token.push_str(&format!("{:016x}", hasher.finish()));
    }
    token
}

/// An HTTP request, as much of it as is needed.
struct HttpRequest {
    method: String,
    path: String,
    /// Header names in lower case.
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

fn read_request(stream: &TcpStream) -> Result<HttpRequest, &'static str> {
    let mut reader = BufReader::new(stream);
    let mut head = Vec::new();
    loop {
        let mut line = Vec::new();
        let n = reader
            .by_ref()
            .take((MAX_HEADER_BYTES + 1 - head.len().min(MAX_HEADER_BYTES)) as u64)
            .read_until(b'\n', &mut line)
            .map_err(|_| "could not read the request")?;
        if n == 0 {
            return Err("the connection closed");
        }
        head.extend_from_slice(&line);
        if head.len() > MAX_HEADER_BYTES {
            return Err("the request headers are too long");
        }
        if line == b"\r\n" || line == b"\n" {
            break;
        }
    }
    let head = String::from_utf8_lossy(&head);
    let mut lines = head.lines();
    let mut first = lines.next().unwrap_or("").split_whitespace();
    let method = first.next().unwrap_or("").to_string();
    let path = first.next().unwrap_or("").to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let length: usize = headers
        .iter()
        .find(|(n, _)| n == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    if length > MAX_BODY_BYTES {
        return Err("the request is too large");
    }
    let mut body = vec![0; length];
    reader
        .read_exact(&mut body)
        .map_err(|_| "could not read the request body")?;
    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn respond(mut stream: &TcpStream, status: &str, content_type: Option<&str>, body: &[u8]) {
    let mut head = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n",
        body.len()
    );
    if let Some(content_type) = content_type {
        head.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    if status.starts_with("405") {
        head.push_str("Allow: POST\r\n");
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn serve_connection(stream: TcpStream, shared: &Shared) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let request = match read_request(&stream) {
        Ok(request) => request,
        Err(reason) => {
            respond(
                &stream,
                "400 Bad Request",
                Some("text/plain"),
                reason.as_bytes(),
            );
            return;
        }
    };
    if let Err((status, reason)) = check(&request, shared) {
        respond(&stream, status, Some("text/plain"), reason.as_bytes());
        return;
    }
    if request.method != "POST" {
        // No event stream is offered, and there are no sessions to end.
        respond(&stream, "405 Method Not Allowed", None, b"");
        return;
    }
    let message: Value = match serde_json::from_slice(&request.body) {
        Ok(message) => message,
        Err(_) => {
            let error = rpc_error(Value::Null, -32700, "The request is not valid JSON.");
            respond(&stream, "400 Bad Request", Some("application/json"), &error);
            return;
        }
    };
    let call_tool = |tool: &str, args: Value| -> Outcome {
        let (reply, answer) = channel();
        let sent = shared.sender.lock().map(|sender| {
            sender.send(Request {
                tool: tool.to_string(),
                args,
                reply,
            })
        });
        if !matches!(sent, Ok(Ok(()))) {
            return Outcome::error("STAMP's window has closed.");
        }
        (shared.wake)();
        answer.recv_timeout(ANSWER_TIMEOUT).unwrap_or_else(|_| {
            Outcome::error(
                "STAMP did not answer in time: it may still be busy, or its window was closed.",
            )
        })
    };
    if message.get("method").and_then(Value::as_str) == Some("initialize") {
        shared.clients.fetch_add(1, Ordering::SeqCst);
    }
    match handle(&message, &call_tool) {
        Some(answer) => respond(
            &stream,
            "200 OK",
            Some("application/json"),
            answer.to_string().as_bytes(),
        ),
        None => respond(&stream, "202 Accepted", None, b""),
    }
}

/// Refuse requests that are not for this server, from this computer, with
/// its token.
fn check(request: &HttpRequest, shared: &Shared) -> Result<(), (&'static str, &'static str)> {
    if request.path.split('?').next() != Some("/mcp") {
        return Err(("404 Not Found", "STAMP's assistant server answers at /mcp"));
    }
    let host_ok = request.header("host").is_some_and(|host| {
        host == format!("127.0.0.1:{}", shared.port) || host == format!("localhost:{}", shared.port)
    });
    if !host_ok {
        return Err(("403 Forbidden", "wrong host"));
    }
    if let Some(origin) = request.header("origin") {
        let local = ["http://127.0.0.1", "http://localhost"]
            .iter()
            .any(|prefix| origin == *prefix || origin.starts_with(&format!("{prefix}:")));
        if !local {
            return Err(("403 Forbidden", "requests from web pages are refused"));
        }
    }
    let token_ok = request
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| token.trim() == shared.token);
    if !token_ok {
        return Err(("401 Unauthorized", "missing or wrong access token"));
    }
    Ok(())
}

fn rpc_error(id: Value, code: i64, message: &str) -> Vec<u8> {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
        .to_string()
        .into_bytes()
}

/// Answer one JSON-RPC message; `None` for a notification or a response,
/// which get no answer. `call_tool` carries out a tool call.
pub fn handle(message: &Value, call_tool: &dyn Fn(&str, Value) -> Outcome) -> Option<Value> {
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str);
    let (Some(id), Some(method)) = (id, method) else {
        // A notification (no id) or a response to us (no method).
        return None;
    };
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => {
            let asked = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("");
            let version = PROTOCOL_VERSIONS
                .iter()
                .find(|v| **v == asked)
                .unwrap_or(&PROTOCOL_VERSIONS[0]);
            Ok(json!({
                "protocolVersion": version,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {
                    "name": "stamp",
                    "title": "STAMP",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": tools::INSTRUCTIONS
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({
            "tools": tools::list()
                .into_iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": t.input_schema,
                        "annotations": {
                            "readOnlyHint": !t.changes,
                            "destructiveHint": false,
                            "openWorldHint": false
                        }
                    })
                })
                .collect::<Vec<_>>()
        })),
        "tools/call" => match params.get("name").and_then(Value::as_str) {
            Some(name) if tools::list().iter().any(|t| t.name == name) => {
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                Ok(outcome_json(&call_tool(name, args)))
            }
            Some(name) => Err((-32602, format!("Unknown tool: {name}"))),
            None => Err((-32602, "tools/call needs a tool name".to_string())),
        },
        other => Err((-32601, format!("Method not found: {other}"))),
    };
    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err((code, message)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
        }
    })
}

/// A tool's answer as MCP's `CallToolResult`.
fn outcome_json(outcome: &Outcome) -> Value {
    let content: Vec<Value> = outcome
        .content
        .iter()
        .map(|c| match c {
            Content::Text(text) => json!({"type": "text", "text": text}),
            Content::Png(bytes) => json!({
                "type": "image",
                "data": super::base64(bytes),
                "mimeType": "image/png"
            }),
        })
        .collect();
    json!({"content": content, "isError": outcome.is_error})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(port: u16, token: Option<&str>, host: &str, body: &str) -> (String, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let auth = token.map_or(String::new(), |t| format!("Authorization: Bearer {t}\r\n"));
        let request = format!(
            "POST /mcp HTTP/1.1\r\nHost: {host}\r\n{auth}Content-Type: application/json\r\n\
             Accept: application/json, text/event-stream\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        (head.lines().next().unwrap().to_string(), body.to_string())
    }

    #[test]
    fn answers_mcp_requests_with_the_token_only() {
        let server = Server::start(|| {}).unwrap();
        let port = server.port();
        let token = server.token().to_string();
        let host = format!("127.0.0.1:{port}");

        // The window, answering tool calls.
        let answering = std::thread::spawn(move || {
            for _ in 0..200 {
                if let Some(request) = server.next_request() {
                    let outcome = Outcome::text(format!("called {}", request.tool));
                    request.reply.send(outcome).unwrap();
                    return server.clients();
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            panic!("no tool call arrived");
        });

        let initialize = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#;
        let (status, body) = post(port, Some(&token), &host, initialize);
        assert_eq!(status, "HTTP/1.1 200 OK");
        let body: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(body["result"]["serverInfo"]["name"], "stamp");

        let (status, body) = post(
            port,
            Some(&token),
            &host,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        );
        assert_eq!(status, "HTTP/1.1 202 Accepted");
        assert!(body.is_empty());

        let (_, body) = post(
            port,
            Some(&token),
            &host,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        );
        let body: Value = serde_json::from_str(&body).unwrap();
        let tools = body["result"]["tools"].as_array().unwrap();
        assert!(tools.iter().any(|t| t["name"] == "get_current_plot"));

        let (_, body) = post(
            port,
            Some(&token),
            &host,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_session","arguments":{}}}"#,
        );
        let body: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["result"]["content"][0]["text"], "called get_session");
        assert_eq!(body["result"]["isError"], false);
        assert_eq!(answering.join().unwrap(), 1);
    }

    #[test]
    fn refuses_requests_without_the_token_or_from_elsewhere() {
        let server = Server::start(|| {}).unwrap();
        let port = server.port();
        let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let host = format!("127.0.0.1:{port}");
        let (status, _) = post(port, None, &host, ping);
        assert_eq!(status, "HTTP/1.1 401 Unauthorized");
        let (status, _) = post(port, Some("wrong"), &host, ping);
        assert_eq!(status, "HTTP/1.1 401 Unauthorized");
        let (status, _) = post(port, Some(server.token()), "evil.example:80", ping);
        assert_eq!(status, "HTTP/1.1 403 Forbidden");
        let (status, body) = post(
            port,
            Some(server.token()),
            &format!("localhost:{port}"),
            ping,
        );
        assert_eq!(status, "HTTP/1.1 200 OK", "{body}");
    }

    #[test]
    fn unknown_methods_and_tools_are_errors() {
        let call = |_: &str, _: Value| Outcome::text("");
        let answer = handle(&json!({"jsonrpc": "2.0", "id": 7, "method": "nope"}), &call).unwrap();
        assert_eq!(answer["error"]["code"], -32601);
        let answer = handle(
            &json!({"jsonrpc": "2.0", "id": 8, "method": "tools/call", "params": {"name": "nope"}}),
            &call,
        )
        .unwrap();
        assert_eq!(answer["error"]["code"], -32602);
        assert!(handle(
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            &call
        )
        .is_none());
    }

    #[test]
    fn tokens_differ() {
        let a = new_token();
        assert_eq!(a.len(), 32);
        assert_ne!(a, new_token());
    }
}
