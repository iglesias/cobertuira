use crate::coverage::{self, CoverageExport, ProcessedFile};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ActivePanel {
    Filter,
    Files,
    Source,
}

pub struct App {
    pub files: Vec<ProcessedFile>,
    pub filtered_files: Vec<usize>,
    pub selected_index: usize,
    pub scroll_offset: u16,
    pub filter_query: String,
    pub is_refreshing: bool,
    pub root_dir: PathBuf,
    pub active_panel: ActivePanel,
}

impl App {
    pub fn new() -> Result<Self> {
        // Let's find root by looking for CMakeLists.txt
        let mut root = std::env::current_dir()?;
        while !root.join("CMakeLists.txt").exists() {
            if let Some(parent) = root.parent() {
                root = parent.to_path_buf();
            } else {
                break;
            }
        }

        let mut app = Self {
            files: Vec::new(),
            filtered_files: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            filter_query: String::new(),
            is_refreshing: false,
            root_dir: root,
            active_panel: ActivePanel::Files,
        };
        app.load_data()?;
        Ok(app)
    }

    pub fn switch_focus(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Filter => ActivePanel::Files,
            ActivePanel::Files => ActivePanel::Source,
            ActivePanel::Source => ActivePanel::Filter,
        };
    }

    pub fn load_data(&mut self) -> Result<()> {
        let json_path = self.root_dir.join("build/coverage.json");
        if !json_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(json_path)?;
        let export: CoverageExport = serde_json::from_str(&content)?;

        let mut processed_files = Vec::new();

        // Load per-test data first to have it available
        let per_test_coverage = self.load_per_test_data()?;

        for entry in export.data {
            for file_cov in entry.files {
                let path = PathBuf::from(&file_cov.filename);
                let real_path = fs::canonicalize(&path).unwrap_or(path.clone());
                let real_root = fs::canonicalize(&self.root_dir).unwrap_or(self.root_dir.clone());

                if real_path.starts_with(&real_root)
                    && !file_cov.filename.contains("build/")
                    && !file_cov.filename.contains("googletest")
                {
                    let rel_path = real_path
                        .strip_prefix(&real_root)
                        .unwrap_or(&real_path)
                        .to_string_lossy()
                        .into_owned();

                    let line_coverage = coverage::get_line_coverage(&file_cov.segments);
                    let mut branch_info = HashMap::new();
                    for b in file_cov.branches {
                        branch_info.entry(b.0).or_insert_with(Vec::new).push(b.4);
                    }

                    let mut covered_by = HashMap::new();
                    for (line, count) in &line_coverage {
                        if *count > 0 {
                            if let Some(tests) =
                                per_test_coverage.get(&(file_cov.filename.clone(), *line))
                            {
                                covered_by.insert(*line, tests.clone());
                            }
                        }
                    }

                    processed_files.push(ProcessedFile {
                        path: real_path,
                        rel_path,
                        line_pct: file_cov.summary.lines.percent,
                        branch_pct: file_cov.summary.branches.percent,
                        line_coverage,
                        branch_info,
                        covered_by,
                    });
                }
            }
        }

        processed_files.sort_by(|a, b| a.line_pct.partial_cmp(&b.line_pct).unwrap());
        self.files = processed_files;
        self.apply_filter();
        Ok(())
    }

    fn load_per_test_data(&self) -> Result<HashMap<(String, u32), HashSet<String>>> {
        let mut mapping: HashMap<(String, u32), HashSet<String>> = HashMap::new();
        let per_test_dir = self.root_dir.join("build/per_test");
        if !per_test_dir.exists() {
            return Ok(mapping);
        }

        for entry in WalkDir::new(per_test_dir) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                let filename = path.file_name().unwrap().to_string_lossy();
                if filename.starts_with("cov_") {
                    let test_name = filename[4..filename.len() - 5].to_string();
                    let content = fs::read_to_string(path)?;
                    let export: CoverageExport = serde_json::from_str(&content)?;
                    for data_entry in export.data {
                        for file_cov in data_entry.files {
                            let line_cov = coverage::get_line_coverage(&file_cov.segments);
                            for (line, count) in line_cov {
                                if count > 0 {
                                    mapping
                                        .entry((file_cov.filename.clone(), line))
                                        .or_default()
                                        .insert(test_name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(mapping)
    }

    pub fn apply_filter(&mut self) {
        let query = self.filter_query.to_lowercase();
        self.filtered_files = self
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| f.rel_path.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();

        if self.selected_index >= self.filtered_files.len() {
            self.selected_index = self.filtered_files.len().saturating_sub(1);
        }
    }

    pub fn next_file(&mut self) {
        if !self.filtered_files.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_files.len();
            self.scroll_offset = 0;
        }
    }

    pub fn prev_file(&mut self) {
        if !self.filtered_files.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_files.len() - 1;
            } else {
                self.selected_index -= 1;
            }
            self.scroll_offset = 0;
        }
    }
}
