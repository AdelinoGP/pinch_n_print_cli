//! Mock host helpers for module tests.

use std::collections::HashMap;

/// Log severity levels captured by [`MockHost`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Informational messages.
    Info,
    /// Warning messages.
    Warn,
    /// Error messages.
    Error,
}

/// In-memory host double for call-count and log assertions.
#[derive(Debug, Default)]
pub struct MockHost {
    call_counts: HashMap<String, usize>,
    logging_enabled: bool,
    logs: Vec<LogEntry>,
}

#[derive(Debug, Clone)]
struct LogEntry {
    level: LogLevel,
    message: String,
}

impl MockHost {
    /// Create a new mock host instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let _host = MockHost::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a named host call occurred.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let mut host = MockHost::new();
    /// host.record_call("clip_polygons");
    /// ```
    pub fn record_call(&mut self, name: &str) {
        let counter = self.call_counts.entry(name.to_string()).or_insert(0);
        *counter += 1;
    }

    /// Enable in-memory log capture.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let mut host = MockHost::new();
    /// host.enable_logging();
    /// ```
    pub fn enable_logging(&mut self) {
        self.logging_enabled = true;
    }

    /// Record a warning log message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let mut host = MockHost::new();
    /// host.enable_logging();
    /// host.log_warn("example");
    /// ```
    pub fn log_warn(&mut self, message: &str) {
        if self.logging_enabled {
            self.logs.push(LogEntry {
                level: LogLevel::Warn,
                message: message.to_string(),
            });
        }
    }

    /// Return how many times a named host call was recorded.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let mut host = MockHost::new();
    /// host.record_call("clip_polygons");
    /// assert_eq!(host.call_count("clip_polygons"), 1);
    /// ```
    #[must_use]
    pub fn call_count(&self, name: &str) -> usize {
        self.call_counts.get(name).copied().unwrap_or(0)
    }

    /// Assert a call count for a named host call.
    ///
    /// # Panics
    /// Panics when the observed call count differs from `expected`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::MockHost;
    ///
    /// let mut host = MockHost::new();
    /// host.record_call("clip_polygons");
    /// host.assert_call_count("clip_polygons", 1);
    /// ```
    pub fn assert_call_count(&self, name: &str, expected: usize) {
        let actual = self.call_count(name);
        assert_eq!(actual, expected, "unexpected call count for {name}");
    }

    /// Check whether logs include a message containing `needle` at `level`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_test::mock_host::LogLevel;
    /// use slicer_test::MockHost;
    ///
    /// let mut host = MockHost::new();
    /// host.enable_logging();
    /// host.log_warn("density near limit");
    /// assert!(host.log_contains(LogLevel::Warn, "density"));
    /// ```
    #[must_use]
    pub fn log_contains(&self, level: LogLevel, needle: &str) -> bool {
        self.logs
            .iter()
            .any(|entry| entry.level == level && entry.message.contains(needle))
    }
}
