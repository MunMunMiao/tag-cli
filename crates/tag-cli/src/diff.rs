use tag_core::workflow::context::{Context, CoverAction};

/// Compute a human-readable diff of the changes recorded in `ctx`.
///
/// Returns `None` if the original metadata has not been loaded yet (i.e.
/// `ReadMetadataStep` has not run). The diff includes tag set/clear changes
/// when `ctx.tag_updates` is present, and cover art changes when
/// `ctx.cover_action` indicates a set/clear.
pub fn compute_diff(ctx: &Context) -> Option<String> {
    let original = ctx.original_metadata.as_ref()?;
    let mut lines = vec![format!("Would update: {}", ctx.input_path.display())];

    // Tag removals / clears
    if let Some(updates) = ctx.tag_updates.as_ref() {
        if updates.replace {
            // Replace mode: keep only the explicitly provided tags; everything
            // else is shown as being cleared.
            for (key, old) in &original.properties {
                if !updates.sets.contains_key(key) {
                    lines.push(format!("  - {}: {:?} -> (cleared)", key, Some(old)));
                }
            }

            // Tag sets
            for (key, new_values) in &updates.sets {
                let old = original.properties.get(key);
                lines.push(format!("  - {}: {:?} -> {:?}", key, old, new_values));
            }
        } else {
            for key in &updates.clears {
                let old = original.properties.get(key);
                lines.push(format!("  - {}: {:?} -> (cleared)", key, old));
            }

            if updates.clear_all {
                for (key, old) in &original.properties {
                    lines.push(format!("  - {}: {:?} -> (cleared)", key, old));
                }
            }

            // Tag sets
            for (key, new_values) in &updates.sets {
                let old = original.properties.get(key);
                lines.push(format!("  - {}: {:?} -> {:?}", key, old, new_values));
            }
        }
    }

    // Cover
    match ctx.cover_action {
        CoverAction::Clear if !original.pictures.is_empty() => {
            lines.push("  - cover: (present) -> (removed)".to_string());
        }
        CoverAction::Set(_) if ctx.processed_cover.is_some() => {
            lines.push("  - cover: (old) -> (new processed cover)".to_string());
        }
        _ => {}
    }

    if lines.len() == 1 {
        lines.push("  (no changes)".to_string());
    }

    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use tag_core::taglib::{Metadata, Picture, Tags};
    use tag_core::workflow::context::TagUpdates;

    fn sample_picture() -> Picture {
        Picture {
            mime_type: Some("image/jpeg".to_string()),
            description: None,
            picture_type: None,
            data: vec![1, 2, 3],
        }
    }

    fn ctx_with(
        properties: BTreeMap<String, Vec<String>>,
        pictures: Vec<Picture>,
        updates: Option<TagUpdates>,
    ) -> Context {
        let mut ctx = Context::new("/tmp/test.flac", true, false);
        ctx.original_metadata = Some(Metadata {
            tags: Tags::default(),
            properties,
            pictures,
            audio: None,
        });
        ctx.tag_updates = updates;
        ctx
    }

    #[test]
    fn test_compute_diff_sets_shows_old_and_new() {
        let diff = compute_diff(&ctx_with(
            BTreeMap::from([("TITLE".to_string(), vec!["Old Title".to_string()])]),
            vec![],
            Some(TagUpdates {
                sets: BTreeMap::from([("TITLE".to_string(), vec!["New Title".to_string()])]),
                clears: vec![],
                clear_all: false,
                replace: false,
            }),
        ))
        .unwrap();

        assert!(diff.contains("Would update: /tmp/test.flac"));
        assert!(diff.contains("TITLE: Some([\"Old Title\"]) -> [\"New Title\"]"));
    }

    #[test]
    fn test_compute_diff_clears_shows_cleared() {
        let diff = compute_diff(&ctx_with(
            BTreeMap::from([("TITLE".to_string(), vec!["Old Title".to_string()])]),
            vec![],
            Some(TagUpdates {
                sets: BTreeMap::new(),
                clears: vec!["TITLE".to_string()],
                clear_all: false,
                replace: false,
            }),
        ))
        .unwrap();

        assert!(diff.contains("TITLE: Some([\"Old Title\"]) -> (cleared)"));
    }

    #[test]
    fn test_compute_diff_clear_all_clears_all_properties() {
        let diff = compute_diff(&ctx_with(
            BTreeMap::from([
                ("TITLE".to_string(), vec!["Title".to_string()]),
                ("ARTIST".to_string(), vec!["Artist".to_string()]),
            ]),
            vec![],
            Some(TagUpdates {
                sets: BTreeMap::new(),
                clears: vec![],
                clear_all: true,
                replace: false,
            }),
        ))
        .unwrap();

        assert!(diff.contains("TITLE: [\"Title\"] -> (cleared)"));
        assert!(diff.contains("ARTIST: [\"Artist\"] -> (cleared)"));
    }

    #[test]
    fn test_compute_diff_cover_set_shows_new_processed_cover() {
        let mut ctx = ctx_with(BTreeMap::new(), vec![sample_picture()], None);
        ctx.cover_action = CoverAction::Set(PathBuf::from("cover.jpg"));
        ctx.processed_cover = Some(sample_picture());

        let diff = compute_diff(&ctx).unwrap();
        assert!(diff.contains("cover: (old) -> (new processed cover)"));
    }

    #[test]
    fn test_compute_diff_cover_clear_shows_removed() {
        let mut ctx = ctx_with(BTreeMap::new(), vec![sample_picture()], None);
        ctx.cover_action = CoverAction::Clear;

        let diff = compute_diff(&ctx).unwrap();
        assert!(diff.contains("cover: (present) -> (removed)"));
    }

    #[test]
    fn test_compute_diff_no_changes_shows_no_changes() {
        let diff = compute_diff(&ctx_with(
            BTreeMap::new(),
            vec![],
            Some(TagUpdates::default()),
        ))
        .unwrap();

        assert!(diff.contains("Would update: /tmp/test.flac"));
        assert!(diff.contains("(no changes)"));
    }

    #[test]
    fn test_compute_diff_missing_original_returns_none() {
        let mut ctx = Context::new("/tmp/test.flac", true, false);
        ctx.tag_updates = Some(TagUpdates::default());
        assert!(compute_diff(&ctx).is_none());
    }

    #[test]
    fn test_compute_diff_cover_action_without_change_no_cover_line() {
        // Set without a processed cover, clear with no pictures, and keep should all
        // produce no cover diff line.
        let mut set_ctx = ctx_with(BTreeMap::new(), vec![sample_picture()], None);
        set_ctx.cover_action = CoverAction::Set(PathBuf::from("cover.jpg"));
        let diff = compute_diff(&set_ctx).unwrap();
        assert!(!diff.contains("cover"));

        let mut clear_ctx = ctx_with(BTreeMap::new(), vec![], None);
        clear_ctx.cover_action = CoverAction::Clear;
        let diff = compute_diff(&clear_ctx).unwrap();
        assert!(!diff.contains("cover"));

        let keep_ctx = ctx_with(BTreeMap::new(), vec![sample_picture()], None);
        let diff = compute_diff(&keep_ctx).unwrap();
        assert!(!diff.contains("cover"));
    }

    #[test]
    fn test_compute_diff_cover_only_without_updates() {
        // cover clear on a file with a cover should produce a diff even when
        // tag_updates is None.
        let mut ctx = ctx_with(BTreeMap::new(), vec![sample_picture()], None);
        ctx.cover_action = CoverAction::Clear;

        let diff = compute_diff(&ctx).unwrap();
        assert!(diff.contains("Would update: /tmp/test.flac"));
        assert!(diff.contains("cover: (present) -> (removed)"));
        assert!(!diff.contains("(no changes)"));
    }

    #[test]
    fn test_compute_diff_replace_shows_set_and_cleared_tags() {
        let diff = compute_diff(&ctx_with(
            BTreeMap::from([
                ("TITLE".to_string(), vec!["Old Title".to_string()]),
                ("ARTIST".to_string(), vec!["Old Artist".to_string()]),
            ]),
            vec![],
            Some(TagUpdates {
                sets: BTreeMap::from([("TITLE".to_string(), vec!["New Title".to_string()])]),
                clears: vec![],
                clear_all: false,
                replace: true,
            }),
        ))
        .unwrap();

        assert!(diff.contains("Would update: /tmp/test.flac"));
        assert!(diff.contains("TITLE: Some([\"Old Title\"]) -> [\"New Title\"]"));
        assert!(diff.contains("ARTIST: Some([\"Old Artist\"]) -> (cleared)"));
        assert!(!diff.contains("(no changes)"));
    }
}
