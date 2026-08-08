mod db;
mod duplicate;
mod scan;

pub use db::{init_library, LibraryState, ScanResult, Track};
pub use duplicate::{DuplicateChoice, DuplicateResolver, TrackCandidate};
pub use scan::{import_file, start_scan_folder};
