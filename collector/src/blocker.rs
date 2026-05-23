use std::path::Path;

use anyhow::Result;

use crate::models::{BlockerConfig, BlockerRule, WindowSnapshot};

#[derive(Debug, Clone)]
pub struct BlockerEngine {
    config: BlockerConfig,
}

impl BlockerEngine {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: BlockerConfig = serde_json::from_str(&contents)?;
        Ok(Self { config })
    }

    pub fn empty() -> Self {
        Self {
            config: BlockerConfig {
                version: 1,
                rules: Vec::new(),
            },
        }
    }

    pub fn is_blocked(&self, capture_type: &str, snapshot: &WindowSnapshot) -> bool {
        self.config
            .rules
            .iter()
            .any(|rule| self.rule_matches(rule, capture_type, snapshot))
    }

    pub fn matching_rules<'a>(
        &'a self,
        capture_type: &str,
        snapshot: &WindowSnapshot,
    ) -> Vec<&'a BlockerRule> {
        self.config
            .rules
            .iter()
            .filter(|rule| self.rule_matches(rule, capture_type, snapshot))
            .collect()
    }

    pub fn rules(&self) -> &[BlockerRule] {
        &self.config.rules
    }

    fn rule_matches(
        &self,
        rule: &BlockerRule,
        capture_type: &str,
        snapshot: &WindowSnapshot,
    ) -> bool {
        if rule.capture_type != capture_type {
            return false;
        }

        let actual_value = match rule.field.as_str() {
            "process_name" => &snapshot.process_name,
            "window_title" => snapshot.window_title.as_deref().unwrap_or(""),
            "exe_path_hash" => snapshot.exe_path_hash.as_deref().unwrap_or(""),
            _ => return false,
        };

        match rule.operator.as_str() {
            "equals" => actual_value.eq_ignore_ascii_case(&rule.value),
            "contains" => actual_value
                .to_lowercase()
                .contains(&rule.value.to_lowercase()),
            "starts_with" => actual_value
                .to_lowercase()
                .starts_with(&rule.value.to_lowercase()),
            _ => false,
        }
    }
}
