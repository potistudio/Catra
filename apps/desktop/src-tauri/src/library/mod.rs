mod convert;
mod db;
mod duplicate;
mod facet;
mod health;
mod import_rekordbox;
mod ingest;
mod link;
mod paths;
mod playlist;
mod rule;
mod scan;
mod tag;
mod trash;

pub use convert::{start_convert_tracks, ConvertOptions};
pub use db::{init_library, LibraryState, Track};
pub use duplicate::{DuplicateChoice, DuplicateResolver};
pub use facet::{browse_facet as facet_values, facet_track_ids, FacetValue};
pub use health::{check_health, check_health_with, HealthDepth, HealthReport};
pub use import_rekordbox::start_import_from_rekordbox;
pub use link::{
    add_to_rekordbox as add_track_to_rekordbox, remove_content_id as remove_rekordbox_content_id,
    remove_from_rekordbox as remove_track_from_rekordbox,
};
pub use playlist::{
    add_tracks as add_tracks_to_playlist, create as create_playlist, delete as delete_playlist,
    delete_impact as playlist_deletion_impact, entries as entries_of_playlist,
    list_tree as list_playlist_tree, move_node as move_playlist,
    remove_entries as remove_playlist_entries, remove_tracks as remove_playlist_tracks,
    rename as rename_playlist, reorder_entries as reorder_playlist_entries,
    set_rule as set_playlist_rule, DeleteImpact, PlaylistEntry, PlaylistKind, PlaylistNode,
};
pub use scan::{import_file, start_import_paths, start_scan_folder};
pub use tag::{
    assign as assign_tags, axis_conflicts, create_axis, create_tag, delete_axis, delete_tag,
    list_axes, of_tracks as tags_of_tracks, rename_tag, track_ids as tagged_track_ids,
    unassign as unassign_tags, update_axis, Tag, TagAxis, TagSelection, TrackTags,
};
pub use trash::{delete_permanently, empty_trash, restore_tracks, trash_tracks};
