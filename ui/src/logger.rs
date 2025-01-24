use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc, TimeZone};

#[derive(Clone)]
pub struct LogEntry {
    pub id: String,
    pub summary: String,
    pub details: String,
    pub created_at: u64,
}

pub struct Logger {
    logs: Vec<LogEntry>,
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            logs: Vec::new(),
        }
    }

    pub fn add(&mut self, summary: String, details: String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let id = format!("{}-{}", timestamp, uuid::Uuid::new_v4().to_string());
        
        self.logs.push(LogEntry {
            id,
            summary,
            details,
            created_at: timestamp,
        });
    }

    pub fn format_logs(&self) -> String {
        self.logs.iter()
            .map(|entry| {
                let dt = Utc.timestamp_opt(entry.created_at as i64, 0).unwrap();
                format!("{} -- {}", dt.format("%Y-%m-%d %H:%M:%S"), entry.summary)
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn get_logs(&self) -> &Vec<LogEntry> {
        &self.logs
    }
}
