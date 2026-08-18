mod convert;
mod db;
mod duplicate;
mod health;
mod import_rekordbox;
mod ingest;
mod link;
mod paths;
mod scan;
mod trash;

pub use convert::{start_convert_tracks, ConvertOptions};
pub use db::{init_library, LibraryState, Track};
pub use duplicate::{DuplicateChoice, DuplicateResolver};
pub use health::{check_health, HealthReport};
pub use import_rekordbox::start_import_from_rekordbox;
pub use link::{
    add_to_rekordbox as add_track_to_rekordbox, remove_content_id as remove_rekordbox_content_id,
    remove_from_rekordbox as remove_track_from_rekordbox,
};
pub use scan::{import_file, start_import_paths, start_scan_folder};
pub use trash::{delete_permanently, empty_trash, restore_tracks, trash_tracks};
