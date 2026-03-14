use std::fs;
use std::path::Path;

use super::Project;

const MAGIC: &[u8; 4] = b"PSKT";
const FORMAT_VERSION: u16 = 2;

pub fn save(project: &Project, path: &Path) -> Result<(), String> {
    let mut buf: Vec<u8> = Vec::new();

    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());

    let encoded =
        rmp_serde::to_vec(project).map_err(|e| format!("Failed to encode project: {e}"))?;
    buf.extend_from_slice(&encoded);

    fs::write(path, &buf).map_err(|e| format!("Failed to write file: {e}"))
}

pub fn load(path: &Path) -> Result<Project, String> {
    let data = fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;
    load_bytes(&data)
}

pub fn load_bytes(data: &[u8]) -> Result<Project, String> {
    if data.len() < 8 {
        return Err("File too small".into());
    }

    if &data[0..4] != MAGIC {
        return Err("Not a valid .psikat file (bad magic)".into());
    }

    let version = u16::from_le_bytes([data[4], data[5]]);
    let _flags = u16::from_le_bytes([data[6], data[7]]);

    if version > FORMAT_VERSION {
        return Err(format!(
            "File version {version} is newer than supported version {FORMAT_VERSION}"
        ));
    }

    let project: Project =
        rmp_serde::from_slice(&data[8..]).map_err(|e| format!("Failed to decode project: {e}"))?;

    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;

    #[test]
    fn round_trip() {
        let project = Project::new();
        let dir = std::env::temp_dir();
        let path = dir.join("test_round_trip_v2.psikat");

        save(&project, &path).expect("save failed");
        let loaded = load(&path).expect("load failed");

        assert_eq!(loaded.patterns[0].bpm, project.patterns[0].bpm);
        assert_eq!(loaded.patterns[0].name, project.patterns[0].name);
        assert_eq!(loaded.arranger.len(), project.arranger.len());
        assert_eq!(loaded.flat_order(), project.flat_order());
        assert_eq!(loaded.patterns.len(), project.patterns.len());
        assert_eq!(loaded.tracks.len(), project.tracks.len());
        assert_eq!(loaded.patterns[0].rows, project.patterns[0].rows);
        assert_eq!(loaded.patterns[0].channels, project.patterns[0].channels);
        assert_eq!(loaded.tracks[0].name, project.tracks[0].name);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bad_magic() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_bad_magic.psikat");
        std::fs::write(&path, b"NOPE12345678").unwrap();
        assert!(load(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn file_too_small() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_too_small.psikat");
        std::fs::write(&path, b"PSK").unwrap();
        assert!(load(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }
}
