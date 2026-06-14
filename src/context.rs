use crate::app::App;
use crate::pane::PaneKind;
use std::fmt::Write;
use std::path::Path;

pub struct AtelierContext {
    pub cursor_line: Option<String>,
    pub cursor_file: Option<String>,
    pub pipeline: Option<String>,
    pub variables: Vec<(String, String, String)>,
    pub repl_history: Vec<String>,
    pub hash: u64,
}

impl AtelierContext {
    pub fn gather(app: &App) -> Self {
        let cursor_line = gather_cursor_line(app);
        let cursor_file = gather_cursor_file(app);
        let pipeline = gather_pipeline();
        let variables = gather_variables(&app.vars_csv_path);
        let repl_history = gather_repl_history(app);

        let hash = compute_hash(
            &cursor_line,
            &cursor_file,
            &pipeline,
            &variables,
            &repl_history,
        );

        Self {
            cursor_line,
            cursor_file,
            pipeline,
            variables,
            repl_history,
            hash,
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        writeln!(md, "# Atelier Context Snapshot").unwrap();
        writeln!(md).unwrap();

        if let Some(ref file) = self.cursor_file {
            writeln!(md, "## Current File").unwrap();
            writeln!(md, "`{}`", file).unwrap();
            writeln!(md).unwrap();
        }

        if let Some(ref line) = self.cursor_line {
            writeln!(md, "## Cursor Line").unwrap();
            writeln!(md, "```").unwrap();
            writeln!(md, "{}", line).unwrap();
            writeln!(md, "```").unwrap();
            writeln!(md).unwrap();
        }

        if let Some(ref pipeline) = self.pipeline {
            writeln!(md, "## Pipeline State").unwrap();
            writeln!(md, "```dot").unwrap();
            writeln!(md, "{}", pipeline).unwrap();
            writeln!(md, "```").unwrap();
            writeln!(md).unwrap();
        }

        if !self.variables.is_empty() {
            writeln!(md, "## Environment Variables").unwrap();
            writeln!(md, "| Name | Type | Value |").unwrap();
            writeln!(md, "|------|------|-------|").unwrap();
            for (name, ty, val) in &self.variables {
                writeln!(md, "| {} | {} | {} |", name, ty, val).unwrap();
            }
            writeln!(md).unwrap();
        }

        if !self.repl_history.is_empty() {
            writeln!(md, "## Recent REPL History").unwrap();
            writeln!(md, "```").unwrap();
            for line in self.repl_history.iter().rev().take(10) {
                writeln!(md, "{}", line).unwrap();
            }
            writeln!(md, "```").unwrap();
        }

        md
    }
}

fn gather_cursor_line(app: &App) -> Option<String> {
    app.panes
        .iter()
        .find(|p| p.kind() == PaneKind::Editor)
        .and_then(|p| p.get_cursor_line())
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

fn gather_cursor_file(app: &App) -> Option<String> {
    app.panes
        .iter()
        .find(|p| p.kind() == PaneKind::Editor)
        .and_then(|p| p.get_status_line())
        .and_then(|s| extract_filename(&s))
}

fn extract_filename(status_line: &str) -> Option<String> {
    let trimmed = status_line.trim();
    if trimmed.is_empty() {
        return None;
    }
    for sep in &['/', '\\'] {
        if let Some(pos) = trimmed.rfind(*sep) {
            let candidate = trimmed[pos + 1..].trim();
            if !candidate.is_empty()
                && !candidate.contains('\x1b')
                && !candidate.contains('<')
                && !candidate.contains('"')
            {
                return Some(candidate.to_string());
            }
        }
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    words
        .first()
        .filter(|w| !w.is_empty() && !w.contains('\x1b'))
        .map(|w| w.to_string())
}

fn gather_pipeline() -> Option<String> {
    std::fs::read_to_string("/tmp/atelier-diagram.mmd")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn unescape_csv_field(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].replace("\"\"", "\"")
    } else {
        s.replace("\"\"", "\"")
    }
}

fn gather_variables(path: &Path) -> Vec<(String, String, String)> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines = content.lines();
    let _header = lines.next();
    lines
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, ',').collect();
            if parts.len() == 3 {
                Some((
                    unescape_csv_field(parts[0]),
                    unescape_csv_field(parts[1]),
                    unescape_csv_field(parts[2]),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn gather_repl_history(app: &App) -> Vec<String> {
    app.panes
        .iter()
        .find(|p| p.kind() == PaneKind::Repl)
        .map(|p| p.get_visible_lines())
        .unwrap_or_default()
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .collect()
}

fn compute_hash(
    cursor_line: &Option<String>,
    cursor_file: &Option<String>,
    pipeline: &Option<String>,
    variables: &Vec<(String, String, String)>,
    repl_history: &Vec<String>,
) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    cursor_line.hash(&mut h);
    cursor_file.hash(&mut h);
    pipeline.hash(&mut h);
    variables.hash(&mut h);
    repl_history.hash(&mut h);
    h.finish()
}
