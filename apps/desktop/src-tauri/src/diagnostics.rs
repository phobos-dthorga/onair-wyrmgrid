use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const LOG_FILE_NAME: &str = "wyrmgrid-diagnostics.jsonl";
const MAX_ENTRIES: usize = 200;
const MAX_FIELD_LENGTH: usize = 500;

static DIAGNOSTICS: OnceLock<DiagnosticLog> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEntry {
    pub occurred_at: String,
    pub level: String,
    pub code: String,
    pub operation: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticLogView {
    pub language: &'static str,
    pub storage: &'static str,
    pub entries: Vec<DiagnosticEntry>,
}

struct DiagnosticLog {
    path: Option<PathBuf>,
    entries: Mutex<VecDeque<DiagnosticEntry>>,
}

impl DiagnosticLog {
    fn open(directory: Option<&Path>) -> Self {
        let path = directory.map(|directory| directory.join(LOG_FILE_NAME));
        let entries = path.as_deref().and_then(load_entries).unwrap_or_default();
        Self {
            path,
            entries: Mutex::new(entries),
        }
    }

    fn record(&self, level: &str, code: &str, operation: &str, message: &str) {
        let entry = DiagnosticEntry {
            occurred_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            level: bounded_field(level),
            code: bounded_field(code),
            operation: bounded_field(operation),
            message: bounded_field(message),
        };
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        entries.push_back(entry.clone());
        while entries.len() > MAX_ENTRIES {
            entries.pop_front();
        }
        if let Some(path) = &self.path
            && (append_entry(path, &entry).is_err() || entries.len() == MAX_ENTRIES)
        {
            let _ = rewrite_entries(path, &entries);
        }
    }

    fn view(&self) -> DiagnosticLogView {
        let entries = self
            .entries
            .lock()
            .map(|entries| entries.iter().cloned().collect())
            .unwrap_or_default();
        DiagnosticLogView {
            language: "English",
            storage: if self.path.is_some() {
                "local_file"
            } else {
                "memory_only"
            },
            entries,
        }
    }

    fn clear(&self) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        entries.clear();
        if let Some(path) = &self.path {
            let _ = rewrite_entries(path, &entries);
        }
    }
}

pub fn initialize(directory: Option<&Path>) {
    let _ = DIAGNOSTICS.set(DiagnosticLog::open(directory));
}

pub fn record(level: &str, code: &str, operation: &str, message: &str) {
    DIAGNOSTICS
        .get_or_init(|| DiagnosticLog::open(None))
        .record(level, code, operation, message);
}

pub fn view() -> DiagnosticLogView {
    DIAGNOSTICS.get_or_init(|| DiagnosticLog::open(None)).view()
}

pub fn clear() -> DiagnosticLogView {
    let diagnostics = DIAGNOSTICS.get_or_init(|| DiagnosticLog::open(None));
    diagnostics.clear();
    diagnostics.view()
}

fn bounded_field(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || *character == ' ')
        .take(MAX_FIELD_LENGTH)
        .collect()
}

fn load_entries(path: &Path) -> Option<VecDeque<DiagnosticEntry>> {
    let file = fs::File::open(path).ok()?;
    let mut entries = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<DiagnosticEntry>(&line).ok())
        .collect::<VecDeque<_>>();
    while entries.len() > MAX_ENTRIES {
        entries.pop_front();
    }
    Some(entries)
}

fn append_entry(path: &Path, entry: &DiagnosticEntry) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, entry).map_err(std::io::Error::other)?;
    file.write_all(b"\n")
}

fn rewrite_entries(path: &Path, entries: &VecDeque<DiagnosticEntry>) -> Result<(), std::io::Error> {
    let mut file = fs::File::create(path)?;
    for entry in entries {
        serde_json::to_writer(&mut file, entry).map_err(std::io::Error::other)?;
        file.write_all(b"\n")?;
    }
    file.flush()
}

#[cfg(test)]
#[path = "tests/diagnostics.rs"]
mod tests;
