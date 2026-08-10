use chrono::{DateTime, Local};
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::{Reader, Writer};
use std::io::Cursor;
use std::path::{Path, PathBuf};

const SPECIAL_PLAYLIST_IDS: &[&str] = &["100000", "200000"];

#[derive(Debug, Clone)]
pub struct PlaylistXmlNode {
    pub id: String,
    pub parent_id: String,
    pub attribute: i32,
    pub timestamp_ms: i64,
    pub lib_type: i32,
    pub check_type: i32,
}

/// Handler for `masterPlaylists6.xml` playlist NODE entries.
pub struct MasterPlaylistXml {
    path: PathBuf,
    /// Raw XML bytes kept for round-trip save.
    raw: String,
    nodes: Vec<PlaylistXmlNode>,
    modified: bool,
}

impl MasterPlaylistXml {
    pub fn open(db_dir: &Path) -> Result<Option<Self>, String> {
        let path = db_dir.join("masterPlaylists6.xml");
        if !path.is_file() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let nodes = parse_nodes(&raw)?;
        Ok(Some(Self {
            path,
            raw,
            nodes,
            modified: false,
        }))
    }

    #[allow(dead_code)]
    pub fn modified(&self) -> bool {
        self.modified
    }

    #[allow(dead_code)]
    pub fn get(&self, playlist_id: &str) -> Option<&PlaylistXmlNode> {
        let hex_id = to_hex_id(playlist_id)?;
        self.nodes.iter().find(|node| node.id.eq_ignore_ascii_case(&hex_id))
    }

    pub fn add(
        &mut self,
        playlist_id: &str,
        parent_id: &str,
        attribute: i32,
        updated_at: DateTime<Local>,
    ) -> Result<(), String> {
        let hex_id = to_hex_id(playlist_id)
            .ok_or_else(|| format!("invalid playlist ID for XML: {playlist_id}"))?;
        if self
            .nodes
            .iter()
            .any(|node| node.id.eq_ignore_ascii_case(&hex_id))
        {
            return Err(format!("playlist {playlist_id} already exists in masterPlaylists6.xml"));
        }
        let parent_hex = parent_to_hex(parent_id)?;
        self.nodes.push(PlaylistXmlNode {
            id: hex_id,
            parent_id: parent_hex,
            attribute,
            timestamp_ms: updated_at.timestamp_millis(),
            lib_type: 0,
            check_type: 0,
        });
        self.modified = true;
        Ok(())
    }

    pub fn update_timestamp(
        &mut self,
        playlist_id: &str,
        updated_at: DateTime<Local>,
    ) -> Result<(), String> {
        let hex_id = to_hex_id(playlist_id)
            .ok_or_else(|| format!("invalid playlist ID for XML: {playlist_id}"))?;
        let Some(node) = self
            .nodes
            .iter_mut()
            .find(|node| node.id.eq_ignore_ascii_case(&hex_id))
        else {
            if !SPECIAL_PLAYLIST_IDS.contains(&playlist_id) {
                eprintln!(
                    "playlist {playlist_id} not found in masterPlaylists6.xml; skipping XML update"
                );
            }
            return Ok(());
        };
        node.timestamp_ms = updated_at.timestamp_millis();
        self.modified = true;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_parent(
        &mut self,
        playlist_id: &str,
        parent_id: &str,
        updated_at: DateTime<Local>,
    ) -> Result<(), String> {
        let hex_id = to_hex_id(playlist_id)
            .ok_or_else(|| format!("invalid playlist ID for XML: {playlist_id}"))?;
        let parent_hex = parent_to_hex(parent_id)?;
        let Some(node) = self
            .nodes
            .iter_mut()
            .find(|node| node.id.eq_ignore_ascii_case(&hex_id))
        else {
            if !SPECIAL_PLAYLIST_IDS.contains(&playlist_id) {
                eprintln!(
                    "playlist {playlist_id} not found in masterPlaylists6.xml; skipping XML update"
                );
            }
            return Ok(());
        };
        node.parent_id = parent_hex;
        node.timestamp_ms = updated_at.timestamp_millis();
        self.modified = true;
        Ok(())
    }

    pub fn remove(&mut self, playlist_id: &str) -> Result<(), String> {
        let hex_id = to_hex_id(playlist_id)
            .ok_or_else(|| format!("invalid playlist ID for XML: {playlist_id}"))?;
        let before = self.nodes.len();
        self.nodes
            .retain(|node| !node.id.eq_ignore_ascii_case(&hex_id));
        if self.nodes.len() != before {
            self.modified = true;
        }
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), String> {
        if !self.modified {
            return Ok(());
        }
        let output = rebuild_xml(&self.raw, &self.nodes)?;
        std::fs::write(&self.path, output)
            .map_err(|error| format!("failed to write {}: {error}", self.path.display()))?;
        self.modified = false;
        Ok(())
    }
}

fn to_hex_id(playlist_id: &str) -> Option<String> {
    let value: u64 = playlist_id.parse().ok()?;
    Some(format!("{value:X}"))
}

fn parent_to_hex(parent_id: &str) -> Result<String, String> {
    if parent_id.is_empty() || parent_id.eq_ignore_ascii_case("root") || parent_id == "0" {
        return Ok("0".to_string());
    }
    to_hex_id(parent_id).ok_or_else(|| format!("invalid parent playlist ID for XML: {parent_id}"))
}

fn parse_nodes(raw: &str) -> Result<Vec<PlaylistXmlNode>, String> {
    let mut reader = Reader::from_str(raw);
    reader.config_mut().trim_text(true);
    let mut nodes = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(element)) | Ok(Event::Start(element)) => {
                if element.name().as_ref() == b"NODE" {
                    if let Some(node) = parse_node_attrs(&element)? {
                        nodes.push(node);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(format!("failed to parse masterPlaylists6.xml: {error}")),
            _ => {}
        }
        buf.clear();
    }

    Ok(nodes)
}

fn parse_node_attrs(element: &BytesStart<'_>) -> Result<Option<PlaylistXmlNode>, String> {
    let mut id = None;
    let mut parent_id = None;
    let mut attribute = None;
    let mut timestamp_ms = None;
    let mut lib_type = 0;
    let mut check_type = 0;

    for attr in element.attributes() {
        let attr = attr.map_err(|error| error.to_string())?;
        let key = attr.key.as_ref();
        let value = attr.unescape_value().map_err(|error| error.to_string())?;
        match key {
            b"Id" => id = Some(value.into_owned()),
            b"ParentId" => parent_id = Some(value.into_owned()),
            b"Attribute" => {
                attribute = Some(
                    value
                        .parse::<i32>()
                        .map_err(|error| format!("invalid Attribute: {error}"))?,
                )
            }
            b"Timestamp" => {
                timestamp_ms = Some(
                    value
                        .parse::<i64>()
                        .map_err(|error| format!("invalid Timestamp: {error}"))?,
                )
            }
            b"Lib_Type" => {
                lib_type = value
                    .parse::<i32>()
                    .map_err(|error| format!("invalid Lib_Type: {error}"))?
            }
            b"CheckType" => {
                check_type = value
                    .parse::<i32>()
                    .map_err(|error| format!("invalid CheckType: {error}"))?
            }
            _ => {}
        }
    }

    Ok(Some(PlaylistXmlNode {
        id: id.ok_or_else(|| "NODE missing Id".to_string())?,
        parent_id: parent_id.unwrap_or_else(|| "0".to_string()),
        attribute: attribute.unwrap_or(0),
        timestamp_ms: timestamp_ms.unwrap_or(0),
        lib_type,
        check_type,
    }))
}

/// Rewrite PLAYLISTS section NODE children while preserving the rest of the document shell.
fn rebuild_xml(raw: &str, nodes: &[PlaylistXmlNode]) -> Result<String, String> {
    let mut reader = Reader::from_str(raw);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut in_playlists = false;
    let mut skipping_old_nodes = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(element)) => {
                if element.name().as_ref() == b"PLAYLISTS" {
                    writer
                        .write_event(Event::Start(element.clone()))
                        .map_err(|error| error.to_string())?;
                    in_playlists = true;
                    skipping_old_nodes = true;
                    for node in nodes {
                        write_node(&mut writer, node)?;
                    }
                } else if !(in_playlists && skipping_old_nodes && element.name().as_ref() == b"NODE")
                {
                    writer
                        .write_event(Event::Start(element.clone()))
                        .map_err(|error| error.to_string())?;
                }
            }
            Ok(Event::Empty(element)) => {
                if in_playlists && skipping_old_nodes && element.name().as_ref() == b"NODE" {
                    // Drop original NODE entries; rewritten above.
                } else {
                    writer
                        .write_event(Event::Empty(element.clone()))
                        .map_err(|error| error.to_string())?;
                }
            }
            Ok(Event::End(element)) => {
                if element.name().as_ref() == b"PLAYLISTS" {
                    in_playlists = false;
                    skipping_old_nodes = false;
                }
                if !(skipping_old_nodes && element.name().as_ref() == b"NODE") {
                    writer
                        .write_event(Event::End(element.clone()))
                        .map_err(|error| error.to_string())?;
                }
            }
            Ok(Event::Eof) => break,
            Ok(event) => {
                if !(in_playlists && skipping_old_nodes) {
                    writer
                        .write_event(event)
                        .map_err(|error| error.to_string())?;
                }
            }
            Err(error) => return Err(format!("failed to rebuild masterPlaylists6.xml: {error}")),
        }
        buf.clear();
    }

    let bytes = writer.into_inner().into_inner();
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

fn write_node(writer: &mut Writer<Cursor<Vec<u8>>>, node: &PlaylistXmlNode) -> Result<(), String> {
    let attribute = node.attribute.to_string();
    let timestamp = node.timestamp_ms.to_string();
    let lib_type = node.lib_type.to_string();
    let check_type = node.check_type.to_string();
    let mut element = BytesStart::new("NODE");
    element.push_attribute(("Id", node.id.as_str()));
    element.push_attribute(("ParentId", node.parent_id.as_str()));
    element.push_attribute(("Attribute", attribute.as_str()));
    element.push_attribute(("Timestamp", timestamp.as_str()));
    element.push_attribute(("Lib_Type", lib_type.as_str()));
    element.push_attribute(("CheckType", check_type.as_str()));
    writer
        .write_event(Event::Empty(element))
        .map_err(|error| error.to_string())?;
    let _ = BytesEnd::new("NODE");
    Ok(())
}
