use std::collections::BTreeMap;

use crate::error::TagCliError;
use crate::taglib::{CoverWriteAction, write_full_properties_to_path, write_properties_to_path};
use crate::workflow::context::{Context, CoverAction};
use crate::workflow::step::Step;

#[derive(Debug)]
pub enum SaveMode {
    Incremental,
    FullReplace,
}

#[derive(Debug)]
pub struct SaveFileStep {
    pub mode: SaveMode,
}

impl SaveFileStep {
    pub fn new(mode: SaveMode) -> Self {
        Self { mode }
    }
}

impl Step for SaveFileStep {
    fn name(&self) -> &'static str {
        "SaveFile"
    }

    #[inline(never)]
    fn execute(&self, ctx: &mut Context) -> Result<(), TagCliError> {
        let output_path = ctx.output_path.clone().unwrap_or(ctx.input_path.clone());

        if ctx.dry_run {
            ctx.report
                .messages
                .push(format!("[dry-run] would save to {}", output_path.display()));
            if ctx.verbose {
                tracing::info!("[dry-run] would save to {}", output_path.display());
            }
            return Ok(());
        }

        if ctx.verbose {
            tracing::info!("saving to {}", output_path.display());
        }

        // If writing to a new file, seed it with the original audio data so
        // TagLib can open it.
        if output_path != ctx.input_path
            && !output_path.exists()
            && let Err(e) = std::fs::copy(&ctx.input_path, &output_path)
        {
            return Err(TagCliError::Io(e));
        }

        let cover_action = match &ctx.cover_action {
            CoverAction::Keep => CoverWriteAction::Keep,
            CoverAction::Clear => CoverWriteAction::Clear,
            CoverAction::Set(_) => match ctx.processed_cover.take() {
                Some(pic) => CoverWriteAction::Set(pic),
                None => CoverWriteAction::Keep,
            },
        };

        match self.mode {
            SaveMode::Incremental => {
                let properties = ctx
                    .tag_updates
                    .as_ref()
                    .map(build_incremental_properties)
                    .unwrap_or_default();
                if let Err(e) = write_properties_to_path(&output_path, &properties, cover_action) {
                    return Err(TagCliError::TagLib(e));
                }
            }
            SaveMode::FullReplace => {
                let properties = ctx
                    .tag_updates
                    .as_ref()
                    .map(|u| u.sets.clone())
                    .unwrap_or_default();
                if let Err(e) =
                    write_full_properties_to_path(&output_path, &properties, cover_action)
                {
                    return Err(TagCliError::TagLib(e));
                }
            }
        }

        ctx.report
            .messages
            .push(format!("saved to {}", output_path.display()));

        Ok(())
    }
}

fn build_incremental_properties(
    updates: &crate::workflow::context::TagUpdates,
) -> BTreeMap<String, Vec<String>> {
    let mut props = BTreeMap::new();
    if updates.clear_all {
        // Clear all supported keys by passing empty values.
        for key in crate::taglib::supported_property_keys() {
            props.insert(key.clone(), vec![]);
        }
    } else {
        for key in &updates.clears {
            props.insert(key.clone(), vec![]);
        }
        for (key, values) in &updates.sets {
            props.insert(key.clone(), values.clone());
        }
    }
    props
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taglib::{
        CoverWriteAction, Picture, read_metadata_from_path, write_properties_to_path,
    };
    use crate::workflow::context::TagUpdates;
    use std::collections::BTreeMap;
    use taglib_rs::test_utils::{generate_flac, generate_mp3, generate_ogg};
    use tempfile::TempDir;

    #[test]
    fn step_name() {
        let inc = SaveFileStep::new(SaveMode::Incremental);
        assert_eq!(inc.name(), "SaveFile");
        let full = SaveFileStep::new(SaveMode::FullReplace);
        let _ = format!("{:?} {:?}", inc, full);
    }

    #[test]
    fn dry_run_reports_without_saving() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, true, false);
        ctx.output_path = Some(input.clone());
        assert!(step.execute(&mut ctx).is_ok());
        assert!(ctx.report.messages[0].contains("[dry-run]"));
    }

    #[test]
    fn verbose_dry_run_logs_would_save() {
        use crate::test_helpers::capture_logs;

        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, true, true);
        ctx.output_path = Some(input.clone());
        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("[dry-run] would save to"));
    }

    #[test]
    fn incremental_save_in_place() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["Saved Title".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        assert!(step.execute(&mut ctx).is_ok());
        assert!(ctx.report.messages[0].contains("saved to"));
    }

    #[test]
    fn verbose_save_logs_path() {
        use crate::test_helpers::capture_logs;

        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, true);
        ctx.output_path = Some(input.clone());
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["Saved Title".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("saving to"));
        assert!(logs.contains(&input.to_string_lossy().to_string()));
    }

    #[test]
    fn full_replace_in_place() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::FullReplace);
        let mut ctx = Context::new(&input, false, false);
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["Full Title".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        assert!(step.execute(&mut ctx).is_ok());
    }

    #[test]
    fn copy_input_to_new_output() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        let output = tmp.path().join("output.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        ctx.output_path = Some(output.clone());
        assert!(step.execute(&mut ctx).is_ok());
        assert!(output.exists());
    }

    #[test]
    fn copy_input_to_output_fails_when_input_is_directory() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input_dir");
        std::fs::create_dir(&input).unwrap();
        let output = tmp.path().join("output.mp3");

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        ctx.output_path = Some(output.clone());
        assert!(step.execute(&mut ctx).is_err());
    }

    #[test]
    #[allow(clippy::permissions_set_readonly_false)]
    fn incremental_save_fails_on_read_only_file() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let mut perms = std::fs::metadata(&input).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&input, perms).unwrap();

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        ctx.output_path = Some(input.clone());
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["T".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        let result = step.execute(&mut ctx);

        let mut perms = std::fs::metadata(&input).unwrap().permissions();
        perms.set_readonly(false);
        let _ = std::fs::set_permissions(&input, perms);

        assert!(result.is_err());
    }

    #[test]
    #[allow(clippy::permissions_set_readonly_false)]
    fn full_replace_save_fails_on_read_only_file() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let mut perms = std::fs::metadata(&input).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&input, perms).unwrap();

        let step = SaveFileStep::new(SaveMode::FullReplace);
        let mut ctx = Context::new(&input, false, false);
        ctx.output_path = Some(input.clone());
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["T".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        let result = step.execute(&mut ctx);

        let mut perms = std::fs::metadata(&input).unwrap().permissions();
        perms.set_readonly(false);
        let _ = std::fs::set_permissions(&input, perms);

        assert!(result.is_err());
    }

    #[test]
    fn set_cover_without_processed_cover_falls_back_to_keep() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        ctx.cover_action = CoverAction::Set(tmp.path().join("cover.jpg"));
        assert!(step.execute(&mut ctx).is_ok());
    }

    #[test]
    fn clear_cover_in_place() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        ctx.cover_action = CoverAction::Clear;
        assert!(step.execute(&mut ctx).is_ok());
    }

    #[test]
    fn incremental_save_clear_all_and_cover_removes_cover() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let cover = Picture {
            mime_type: Some("image/jpeg".to_string()),
            description: Some("cover".to_string()),
            picture_type: Some("Front Cover".to_string()),
            data: vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46],
        };
        write_properties_to_path(&input, &BTreeMap::new(), CoverWriteAction::Set(cover)).unwrap();

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        ctx.tag_updates = Some(TagUpdates {
            sets: BTreeMap::new(),
            clears: vec![],
            clear_all: true,
            replace: false,
        });
        ctx.cover_action = CoverAction::Clear;
        assert!(step.execute(&mut ctx).is_ok());

        let metadata = read_metadata_from_path(&input).unwrap();
        assert!(metadata.pictures.is_empty());
    }

    #[test]
    fn set_cover_with_processed_cover_uses_set_action() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        ctx.cover_action = CoverAction::Set(tmp.path().join("cover.jpg"));
        ctx.processed_cover = Some(crate::taglib::Picture {
            mime_type: Some("image/jpeg".to_string()),
            description: Some("cover".to_string()),
            picture_type: Some("Front Cover".to_string()),
            data: vec![0xff, 0xd8, 0xff, 0xe0],
        });
        assert!(step.execute(&mut ctx).is_ok());
    }

    #[test]
    fn incremental_save_clears_all_tags() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        ctx.tag_updates = Some(TagUpdates {
            sets: BTreeMap::new(),
            clears: vec![],
            clear_all: true,
            replace: false,
        });
        assert!(step.execute(&mut ctx).is_ok());
    }

    #[test]
    fn incremental_save_clears_specific_keys() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        ctx.tag_updates = Some(TagUpdates {
            sets: BTreeMap::new(),
            clears: vec!["TITLE".to_string()],
            clear_all: false,
            replace: false,
        });
        assert!(step.execute(&mut ctx).is_ok());
    }

    #[test]
    fn incremental_save_with_multiple_values() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        let mut sets = BTreeMap::new();
        sets.insert("ARTIST".to_string(), vec!["A".to_string(), "B".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        assert!(step.execute(&mut ctx).is_ok());
    }

    #[test]
    fn incremental_save_to_invalid_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("empty.bin");
        std::fs::write(&input, b"not audio").unwrap();

        let step = SaveFileStep::new(SaveMode::Incremental);
        let mut ctx = Context::new(&input, false, false);
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["T".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        let err = step.execute(&mut ctx).unwrap_err();
        assert!(matches!(
            err,
            TagCliError::TagLib(crate::taglib::TagError::InvalidFile)
        ));
    }

    #[test]
    fn full_replace_removes_other_tags() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let mut initial = BTreeMap::new();
        initial.insert("TITLE".to_string(), vec!["Old Title".to_string()]);
        initial.insert("ARTIST".to_string(), vec!["Old Artist".to_string()]);
        write_properties_to_path(&input, &initial, CoverWriteAction::Keep).unwrap();

        let step = SaveFileStep::new(SaveMode::FullReplace);
        let mut ctx = Context::new(&input, false, false);
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["New Title".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        assert!(step.execute(&mut ctx).is_ok());

        let metadata = read_metadata_from_path(&input).unwrap();
        assert_eq!(
            metadata.properties.get("TITLE"),
            Some(&vec!["New Title".to_string()])
        );
        assert!(!metadata.properties.contains_key("ARTIST"));
    }

    #[test]
    fn full_replace_removes_unsupported_tags() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp3");
        generate_mp3(&input);

        let mut initial = BTreeMap::new();
        initial.insert("TITLE".to_string(), vec!["Old Title".to_string()]);
        initial.insert("CUSTOMTAG".to_string(), vec!["Custom Value".to_string()]);
        write_properties_to_path(&input, &initial, CoverWriteAction::Keep).unwrap();

        let step = SaveFileStep::new(SaveMode::FullReplace);
        let mut ctx = Context::new(&input, false, false);
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["New Title".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        assert!(step.execute(&mut ctx).is_ok());

        let metadata = read_metadata_from_path(&input).unwrap();
        assert_eq!(
            metadata.properties.get("TITLE"),
            Some(&vec!["New Title".to_string()])
        );
        assert!(!metadata.properties.contains_key("CUSTOMTAG"));
    }

    #[test]
    fn full_replace_removes_unsupported_tags_flac() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.flac");
        generate_flac(&input);

        let mut initial = BTreeMap::new();
        initial.insert("TITLE".to_string(), vec!["Old Title".to_string()]);
        initial.insert("CUSTOMTAG".to_string(), vec!["Custom Value".to_string()]);
        write_properties_to_path(&input, &initial, CoverWriteAction::Keep).unwrap();

        let step = SaveFileStep::new(SaveMode::FullReplace);
        let mut ctx = Context::new(&input, false, false);
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["New Title".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        assert!(step.execute(&mut ctx).is_ok());

        let metadata = read_metadata_from_path(&input).unwrap();
        assert_eq!(
            metadata.properties.get("TITLE"),
            Some(&vec!["New Title".to_string()])
        );
        assert!(!metadata.properties.contains_key("CUSTOMTAG"));
    }

    #[test]
    fn full_replace_removes_unsupported_tags_ogg() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.ogg");
        generate_ogg(&input);

        let mut initial = BTreeMap::new();
        initial.insert("TITLE".to_string(), vec!["Old Title".to_string()]);
        initial.insert("CUSTOMTAG".to_string(), vec!["Custom Value".to_string()]);
        write_properties_to_path(&input, &initial, CoverWriteAction::Keep).unwrap();

        let step = SaveFileStep::new(SaveMode::FullReplace);
        let mut ctx = Context::new(&input, false, false);
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["New Title".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        assert!(step.execute(&mut ctx).is_ok());

        let metadata = read_metadata_from_path(&input).unwrap();
        assert_eq!(
            metadata.properties.get("TITLE"),
            Some(&vec!["New Title".to_string()])
        );
        assert!(!metadata.properties.contains_key("CUSTOMTAG"));
    }

    #[test]
    fn full_replace_to_invalid_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("empty.bin");
        std::fs::write(&input, b"not audio").unwrap();

        let step = SaveFileStep::new(SaveMode::FullReplace);
        let mut ctx = Context::new(&input, false, false);
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["T".to_string()]);
        ctx.tag_updates = Some(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        let err = step.execute(&mut ctx).unwrap_err();
        assert!(matches!(
            err,
            TagCliError::TagLib(crate::taglib::TagError::InvalidFile)
        ));
    }
}
