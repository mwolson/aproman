#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    Reload,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepTransition {
    Sleeping,
    Waking,
}
