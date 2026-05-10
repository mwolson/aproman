use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::thread;

use crossbeam_channel::{unbounded, Receiver};

use crate::events::SleepTransition;

pub struct SleepMonitor {
    child: Child,
    rx: Receiver<SleepTransition>,
}

impl SleepMonitor {
    pub fn spawn() -> anyhow::Result<Self> {
        let filter = concat!(
            "interface=org.freedesktop.login1.Manager,",
            "sender=org.freedesktop.login1,",
            "member=PrepareForSleep"
        );
        let mut child = Command::new("dbus-monitor")
            .args(["--system", "--monitor", filter])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdout = child.stdout.take().expect("stdout piped");
        let (tx, rx) = unbounded();
        thread::Builder::new()
            .name("aproman-sleep-monitor".into())
            .spawn(move || {
                let mut expect_state = false;
                for raw_line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if let Some(transition) = parse_line(&raw_line, &mut expect_state) {
                        let _ = tx.send(transition);
                    }
                }
            })?;
        Ok(Self { child, rx })
    }

    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    pub fn rx(&self) -> Receiver<SleepTransition> {
        self.rx.clone()
    }
}

pub fn parse_line(line: &str, expect_state: &mut bool) -> Option<SleepTransition> {
    if line.contains("member=PrepareForSleep") {
        *expect_state = true;
        return None;
    }
    if !*expect_state || !line.contains("boolean ") {
        return None;
    }
    *expect_state = false;
    if line.contains("boolean true") {
        Some(SleepTransition::Sleeping)
    } else if line.contains("boolean false") {
        Some(SleepTransition::Waking)
    } else {
        None
    }
}
