//! The bridge between Claude Desktop and STAMP: the `stamp-mcp` program.
//!
//! Claude Desktop starts local MCP servers itself and talks to them over
//! standard input and output, one JSON-RPC message per line. It usually
//! starts them when it starts, before STAMP is open, so the bridge answers
//! everything but tool calls itself (it knows STAMP's tools) and passes
//! each tool call on to the STAMP window that is sharing its session, over
//! the same local HTTP server Claude Code uses. Which STAMP that is, and
//! its access token, are read afresh for each call from the connection
//! file STAMP writes while it is sharing ([`connection_file`]).
//!
//! When no STAMP is sharing, a tool call is answered with an error that
//! tells Claude to ask the user to open STAMP and share.

use std::io::{BufRead, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{json, Value};

use super::server;
use super::tools::Outcome;

/// Environment variable naming the connection file, for tests.
pub const CONNECTION_ENV: &str = "STAMP_ASSISTANT_CONNECTION";

/// What a tool call is answered with when no STAMP is sharing.
pub const NOT_SHARING: &str = "STAMP is not sharing a session with Claude right now. Ask the \
user to open STAMP, load their data and choose Assistant > Share with Claude Desktop, then try \
again.";

/// Longest wait for STAMP to answer, as the server's own limit.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Where STAMP says how to reach it while it is sharing: its port and
/// access token, readable only by the user.
pub fn connection_file() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(CONNECTION_ENV) {
        return Some(PathBuf::from(path));
    }
    super::launch::working_folder().map(|folder| folder.join("connection.json"))
}

/// How to reach the STAMP that is sharing.
#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
    pub port: u16,
    pub token: String,
}

impl Connection {
    pub fn to_json(&self) -> String {
        json!({"port": self.port, "token": self.token}).to_string()
    }

    pub fn from_json(text: &str) -> Option<Connection> {
        let value: Value = serde_json::from_str(text).ok()?;
        Some(Connection {
            port: u16::try_from(value.get("port")?.as_u64()?).ok()?,
            token: value.get("token")?.as_str()?.to_string(),
        })
    }

    /// The connection in the connection file, if STAMP is sharing.
    pub fn read() -> Option<Connection> {
        Connection::from_json(&std::fs::read_to_string(connection_file()?).ok()?)
    }
}

/// POST one JSON-RPC message to STAMP and return its answer, or `None` if
/// STAMP cannot be reached.
fn post(connection: &Connection, message: &Value) -> Option<Value> {
    let address = (std::net::Ipv4Addr::LOCALHOST, connection.port).into();
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2)).ok()?;
    stream.set_read_timeout(Some(ANSWER_TIMEOUT)).ok()?;
    let body = message.to_string();
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n\
         Content-Type: application/json\r\nAccept: application/json, text/event-stream\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        connection.port,
        connection.token,
        body.len()
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).ok()?;
    let response = String::from_utf8_lossy(&response);
    let (head, body) = response.split_once("\r\n\r\n")?;
    if !head.starts_with("HTTP/1.1 200") {
        return None;
    }
    serde_json::from_str(body).ok()
}

/// The answer to one message from Claude Desktop, or `None` for a
/// notification. `forward` passes a tool call to STAMP.
pub fn answer(message: &Value, forward: &dyn Fn(&Value) -> Option<Value>) -> Option<Value> {
    if message.get("method").and_then(Value::as_str) == Some("tools/call")
        && message.get("id").is_some()
    {
        if let Some(answer) = forward(message) {
            return Some(answer);
        }
        let not_sharing = |_: &str, _: Value| Outcome::error(NOT_SHARING);
        return server::handle(message, &not_sharing);
    }
    // Everything else is answered here; tool calls never reach this
    // `call_tool`.
    let unused = |_: &str, _: Value| Outcome::error(NOT_SHARING);
    server::handle(message, &unused)
}

/// Run the bridge: read messages from standard input until it closes.
pub fn run() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let forward = |message: &Value| post(&Connection::read()?, message);
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(message) => answer(&message, &forward),
            Err(_) => Some(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {"code": -32700, "message": "The message is not valid JSON."}
            })),
        };
        if let Some(reply) = reply {
            // One message per line: serde_json writes no line breaks.
            if writeln!(stdout, "{reply}")
                .and_then(|_| stdout.flush())
                .is_err()
            {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_round_trips() {
        let connection = Connection {
            port: 4321,
            token: "abc".to_string(),
        };
        assert_eq!(
            Connection::from_json(&connection.to_json()),
            Some(connection)
        );
        assert_eq!(
            Connection::from_json("{\"port\": 99999, \"token\": \"x\"}"),
            None
        );
        assert_eq!(Connection::from_json("not json"), None);
    }

    #[test]
    fn answers_without_stamp_and_explains_tool_calls() {
        let nobody = |_: &Value| None;
        let initialize = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                                "params": {"protocolVersion": "2025-06-18"}});
        let reply = answer(&initialize, &nobody).unwrap();
        assert_eq!(reply["result"]["serverInfo"]["name"], "stamp");
        let list = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"});
        let reply = answer(&list, &nobody).unwrap();
        assert!(reply["result"]["tools"].as_array().unwrap().len() > 5);
        let call = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                          "params": {"name": "get_session", "arguments": {}}});
        let reply = answer(&call, &nobody).unwrap();
        assert_eq!(reply["result"]["isError"], true);
        assert_eq!(reply["result"]["content"][0]["text"], NOT_SHARING);
        let notification = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        assert!(answer(&notification, &nobody).is_none());
    }

    #[test]
    fn tool_calls_go_to_stamp() {
        let stamp = |message: &Value| {
            Some(json!({"jsonrpc": "2.0", "id": message["id"], "result": {"forwarded": true}}))
        };
        let call = json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call",
                          "params": {"name": "get_session"}});
        let reply = answer(&call, &stamp).unwrap();
        assert_eq!(reply["id"], 9);
        assert_eq!(reply["result"]["forwarded"], true);
    }
}
