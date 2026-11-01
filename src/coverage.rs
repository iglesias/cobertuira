use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct CoverageExport {
    pub data: Vec<DataEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataEntry {
    pub files: Vec<FileCoverage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileCoverage {
    pub filename: String,
    pub segments: Vec<Segment>,
    pub branches: Vec<Branch>,
    pub summary: Summary,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Segment(pub u32, pub u32, pub u64, pub bool, pub bool, pub bool);

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Branch(
    pub u32,
    pub u32,
    pub u32,
    pub u32,
    pub u64,
    pub u64,
    pub u32,
    pub u32,
    pub u32,
);

#[derive(Debug, Deserialize, Serialize)]
pub struct Summary {
    pub lines: Percentage,
    pub branches: Percentage,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Percentage {
    pub count: u32,
    pub covered: u32,
    pub percent: f64,
}

pub struct ProcessedFile {
    pub path: PathBuf,
    pub rel_path: String,
    pub line_pct: f64,
    pub branch_pct: f64,
    pub line_coverage: HashMap<u32, u64>,
    pub branch_info: HashMap<u32, Vec<u64>>,
    pub covered_by: HashMap<u32, HashSet<String>>,
}

impl ProcessedFile {
    pub fn get_line_count(&self, line: u32) -> Option<u64> {
        self.line_coverage.get(&line).copied()
    }
}

pub fn get_line_coverage(segments: &[Segment]) -> HashMap<u32, u64> {
    let mut line_counts = HashMap::new();
    let mut sorted_segments = segments.to_vec();
    sorted_segments.sort_by_key(|s| (s.0, s.1));

    let mut current_count = None;
    let mut max_line = 0;
    if !segments.is_empty() {
        max_line = segments.iter().map(|s| s.0).max().unwrap_or(0);
    }

    for line in 1..=(max_line + 1) {
        let line_segments: Vec<_> = sorted_segments.iter().filter(|s| s.0 == line).collect();
        if !line_segments.is_empty() {
            for s in line_segments {
                if s.3 {
                    // has_count
                    current_count = Some(s.2);
                }
            }
        }
        if let Some(count) = current_count {
            line_counts.insert(line, count);
        }
    }
    line_counts
}
