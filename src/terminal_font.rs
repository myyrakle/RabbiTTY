use ab_glyph::{FontArc, FontVec};
use fontdb::{Database, Family, Query};
use std::collections::BTreeSet;

pub fn discover_system_terminal_fonts() -> Vec<String> {
    let mut db = Database::new();
    db.load_system_fonts();

    let mut monospaced = BTreeSet::new();

    for face in db.faces() {
        if !face.monospaced {
            continue;
        }
        for (family, _) in &face.families {
            let family = family.trim();
            if !family.is_empty() {
                monospaced.insert(family.to_string());
            }
        }
    }

    monospaced.into_iter().collect()
}

pub fn load_system_font_by_family(family: &str) -> Option<FontArc> {
    let family = family.trim();
    if family.is_empty() {
        return None;
    }

    let mut db = Database::new();
    db.load_system_fonts();

    let families = [Family::Name(family)];
    let query = Query {
        families: &families,
        ..Query::default()
    };

    let id = db.query(&query)?;
    db.with_face_data(id, |data, index| {
        FontVec::try_from_vec_and_index(data.to_vec(), index)
            .ok()
            .map(FontArc::new)
    })?
}
