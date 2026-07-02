use crate::error::TagCliError;
use crate::taglib::supported_property_keys_set;
use crate::workflow::context::{Context, TagUpdates};
use crate::workflow::step::{Step, StepOutcome};

#[derive(Debug)]
pub struct UpdateTagsStep {
    pub updates: TagUpdates,
}

impl UpdateTagsStep {
    pub fn new(updates: TagUpdates) -> Self {
        Self { updates }
    }
}

impl Step for UpdateTagsStep {
    fn name(&self) -> &'static str {
        "UpdateTags"
    }

    fn execute(&self, ctx: &mut Context) -> Result<StepOutcome, TagCliError> {
        let mut updates = self.updates.clone();

        // Normalize keys to uppercase so manifests and CLI input behave consistently.
        updates.clears = updates
            .clears
            .iter()
            .map(|k| k.to_ascii_uppercase())
            .collect();
        updates.sets = updates
            .sets
            .into_iter()
            .map(|(k, v)| (k.to_ascii_uppercase(), v))
            .collect();

        if !updates.clear_all {
            for key in &updates.clears {
                if !supported_property_keys_set().contains(key) {
                    return Err(TagCliError::UnsupportedKey(key.clone()));
                }
            }
            for key in updates.sets.keys() {
                if !supported_property_keys_set().contains(key) {
                    return Err(TagCliError::UnsupportedKey(key.clone()));
                }
            }
        }

        if ctx.verbose {
            if updates.replace {
                tracing::info!("replacing all tags with: {:?}", updates.sets);
            } else if updates.clear_all {
                tracing::info!("clearing all tags");
            } else {
                if !updates.clears.is_empty() {
                    tracing::info!("clearing tags: {:?}", updates.clears);
                }
                for (key, values) in &updates.sets {
                    tracing::info!("setting tag {} = {:?}", key, values);
                }
            }
        }

        ctx.tag_updates = Some(updates);
        Ok(StepOutcome::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::capture_logs;
    use std::collections::BTreeMap;

    #[test]
    fn step_name_and_normalizes_keys() {
        let mut sets = BTreeMap::new();
        sets.insert("title".to_string(), vec!["Title".to_string()]);

        let updates = TagUpdates {
            sets,
            clears: vec!["artist".to_string()],
            clear_all: false,
            replace: false,
        };
        let step = UpdateTagsStep::new(updates);
        assert_eq!(step.name(), "UpdateTags");

        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        assert!(step.execute(&mut ctx).is_ok());

        let stored = ctx.tag_updates.unwrap();
        assert!(stored.sets.contains_key("TITLE"));
        assert_eq!(stored.clears, vec!["ARTIST"]);
    }

    #[test]
    fn unsupported_set_key_errors() {
        let mut sets = BTreeMap::new();
        sets.insert("NOTAKEY".to_string(), vec!["x".to_string()]);

        let step = UpdateTagsStep::new(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        let err = step.execute(&mut ctx).unwrap_err();
        assert_eq!(err.to_string(), "unsupported tag key: NOTAKEY");
    }

    #[test]
    fn unsupported_clear_key_errors() {
        let step = UpdateTagsStep::new(TagUpdates {
            sets: BTreeMap::new(),
            clears: vec!["NOTAKEY".to_string()],
            clear_all: false,
            replace: false,
        });
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        let err = step.execute(&mut ctx).unwrap_err();
        assert_eq!(err.to_string(), "unsupported tag key: NOTAKEY");
    }

    #[test]
    fn unsupported_key_does_not_log() {
        let step = UpdateTagsStep::new(TagUpdates {
            sets: BTreeMap::new(),
            clears: vec!["BADKEY".to_string()],
            clear_all: false,
            replace: false,
        });
        let mut ctx = Context::new("/tmp/test.mp3", false, true);
        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_err());
        assert!(!logs.contains("BADKEY"));
    }

    #[test]
    fn clear_all_skips_validation() {
        let mut sets = BTreeMap::new();
        sets.insert("NOTAKEY".to_string(), vec!["x".to_string()]);

        let step = UpdateTagsStep::new(TagUpdates {
            sets,
            clears: vec!["ALSOBAD".to_string()],
            clear_all: true,
            replace: false,
        });
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        assert!(step.execute(&mut ctx).is_ok());
    }

    #[test]
    fn verbose_logs_set_and_clear() {
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["Title".to_string()]);

        let step = UpdateTagsStep::new(TagUpdates {
            sets,
            clears: vec!["ARTIST".to_string()],
            clear_all: false,
            replace: false,
        });
        let mut ctx = Context::new("/tmp/test.mp3", false, true);
        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("setting tag TITLE"));
        assert!(logs.contains("clearing tags: [\"ARTIST\"]"));
    }

    #[test]
    fn verbose_logs_clear_all() {
        let step = UpdateTagsStep::new(TagUpdates {
            sets: BTreeMap::new(),
            clears: vec![],
            clear_all: true,
            replace: false,
        });
        let mut ctx = Context::new("/tmp/test.mp3", false, true);
        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("clearing all tags"));
    }

    #[test]
    fn verbose_logs_set_only() {
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["Title".to_string()]);

        let step = UpdateTagsStep::new(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: false,
        });
        let mut ctx = Context::new("/tmp/test.mp3", false, true);
        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("setting tag TITLE"));
        assert!(!logs.contains("clearing tags"));
    }

    #[test]
    fn replace_true_normalizes_keys() {
        let mut sets = BTreeMap::new();
        sets.insert("title".to_string(), vec!["Title".to_string()]);

        let step = UpdateTagsStep::new(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: true,
        });
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        assert!(step.execute(&mut ctx).is_ok());

        let stored = ctx.tag_updates.unwrap();
        assert!(stored.sets.contains_key("TITLE"));
        assert!(stored.replace);
    }

    #[test]
    fn replace_true_rejects_unsupported_key() {
        let mut sets = BTreeMap::new();
        sets.insert("NOTAKEY".to_string(), vec!["x".to_string()]);

        let step = UpdateTagsStep::new(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: true,
        });
        let mut ctx = Context::new("/tmp/test.mp3", false, false);
        let err = step.execute(&mut ctx).unwrap_err();
        assert_eq!(err.to_string(), "unsupported tag key: NOTAKEY");
    }

    #[test]
    fn verbose_logs_replace() {
        let mut sets = BTreeMap::new();
        sets.insert("TITLE".to_string(), vec!["Title".to_string()]);

        let step = UpdateTagsStep::new(TagUpdates {
            sets,
            clears: vec![],
            clear_all: false,
            replace: true,
        });
        let mut ctx = Context::new("/tmp/test.mp3", false, true);
        let (result, logs) = capture_logs(|| step.execute(&mut ctx));
        assert!(result.is_ok());
        assert!(logs.contains("replacing all tags with"));
    }
}
