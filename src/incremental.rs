use filetime::FileTime;
use std::path::Path;
use walkdir::WalkDir;

pub struct FreshnessChecker {
    newest_header: Option<FileTime>,
}

impl FreshnessChecker {
    pub fn new(include_paths: &[impl AsRef<Path>]) -> Self {
        let mut newest: Option<FileTime> = None;

        for dir in include_paths {
            let dir = dir.as_ref();
            if !dir.exists() {
                continue;
            }
            for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }
                let ext = entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                if matches!(ext, "h" | "hpp" | "hxx" | "hh") {
                    if let Ok(meta) = entry.metadata() {
                        let mtime = FileTime::from_last_modification_time(&meta);
                        newest = Some(match newest {
                            Some(prev) if mtime > prev => mtime,
                            Some(prev) => prev,
                            None => mtime,
                        });
                    }
                }
            }
        }

        Self {
            newest_header: newest,
        }
    }

    /// Returns true if the object file is up-to-date relative to its source
    pub fn is_fresh(&self, source: &Path, object: &Path) -> bool {
        let Some(obj_mtime) = file_mtime(object) else {
            return false;
        };
        let Some(src_mtime) = file_mtime(source) else {
            return false;
        };

        if src_mtime > obj_mtime {
            return false;
        }

        if let Some(hdr_mtime) = self.newest_header {
            if hdr_mtime > obj_mtime {
                return false;
            }
        }

        true
    }
}

fn file_mtime(path: &Path) -> Option<FileTime> {
    std::fs::metadata(path)
        .ok()
        .map(|m| FileTime::from_last_modification_time(&m))
}
