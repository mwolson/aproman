use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::thread;

use crossbeam_channel::{unbounded, Receiver};
use serde_json::{self, Value};
use tracing::info;

use crate::deps;

pub struct PwMonitor {
    child: Child,
    rx: Receiver<String>,
}

impl PwMonitor {
    pub fn spawn() -> Option<Self> {
        deps::which("pw-dump")?;
        let mut child = Command::new("pw-dump")
            .args(["--monitor", "--no-colors"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdout = child.stdout.take()?;
        let (tx, rx) = unbounded();
        thread::Builder::new()
            .name("aproman-pw-monitor".into())
            .spawn(move || {
                let mut buffer = String::new();
                let mut initial_done = false;
                let mut seen_error_ids = std::collections::HashSet::new();
                let mut reader = BufReader::new(stdout);
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            buffer.push_str(&line);
                            let (rest, next_initial_done, errors) =
                                extract_errors(&buffer, initial_done, &mut seen_error_ids);
                            buffer = rest;
                            initial_done = next_initial_done;
                            for error in errors {
                                let _ = tx.send(error);
                            }
                        }
                    }
                }
            })
            .ok()?;
        info!("Monitoring PipeWire for node errors...");
        Some(Self { child, rx })
    }

    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    pub fn rx(&self) -> Receiver<String> {
        self.rx.clone()
    }
}

pub fn extract_errors(
    buffer: &str,
    mut initial_done: bool,
    seen_error_ids: &mut std::collections::HashSet<i64>,
) -> (String, bool, Vec<String>) {
    let mut rest = buffer.trim_start().to_string();
    let mut errors = Vec::new();
    loop {
        if rest.is_empty() {
            return (String::new(), initial_done, errors);
        }
        let mut stream = serde_json::Deserializer::from_str(&rest).into_iter::<Value>();
        let Some(result) = stream.next() else {
            return (rest, initial_done, errors);
        };
        let Ok(value) = result else {
            return (rest, initial_done, errors);
        };
        let offset = stream.byte_offset();
        rest = rest[offset..].trim_start().to_string();
        let Value::Array(items) = value else {
            continue;
        };
        for item in items {
            let Value::Object(item) = item else {
                continue;
            };
            if item.get("type").and_then(Value::as_str) != Some("PipeWire:Interface:Node") {
                continue;
            }
            let Some(id) = item.get("id").and_then(Value::as_i64) else {
                continue;
            };
            let info = item.get("info").and_then(Value::as_object);
            let state = info
                .and_then(|info| info.get("state"))
                .and_then(Value::as_str);
            if state == Some("error") {
                if initial_done && !seen_error_ids.contains(&id) {
                    let name = info
                        .and_then(|info| info.get("props"))
                        .and_then(Value::as_object)
                        .and_then(|props| props.get("node.name"))
                        .and_then(Value::as_str)
                        .map_or_else(|| format!("id={id}"), ToString::to_string);
                    errors.push(format!("node {name} entered error state"));
                }
                seen_error_ids.insert(id);
            } else {
                seen_error_ids.remove(&id);
            }
        }
        if !initial_done {
            initial_done = true;
        }
    }
}
