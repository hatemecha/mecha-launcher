use std::collections::HashMap;

use regex::Regex;
use serde::Deserialize;

use crate::launcher::{LauncherError, LauncherResult};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Rule {
    #[serde(default)]
    pub action: RuleAction,
    #[serde(default)]
    pub os: Option<RuleOs>,
    #[serde(default)]
    pub features: HashMap<String, bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    #[default]
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuleOs {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeEnvironment {
    pub os_name: String,
    pub os_arch: String,
    pub arch_bits: String,
    pub os_version: Option<String>,
    pub features: HashMap<String, bool>,
}

impl RuntimeEnvironment {
    pub fn current() -> Self {
        let os_name = match std::env::consts::OS {
            "macos" => "osx",
            value => value,
        }
        .to_string();

        let os_arch = std::env::consts::ARCH.to_string();
        let arch_bits = if cfg!(target_pointer_width = "64") {
            "64"
        } else {
            "32"
        }
        .to_string();

        Self {
            os_name,
            os_arch,
            arch_bits,
            os_version: None,
            features: HashMap::new(),
        }
    }
}

pub fn rules_allow(rules: &[Rule], environment: &RuntimeEnvironment) -> LauncherResult<bool> {
    if rules.is_empty() {
        return Ok(true);
    }

    let mut allowed = false;

    for rule in rules {
        if rule_matches(rule, environment)? {
            allowed = matches!(rule.action, RuleAction::Allow);
        }
    }

    Ok(allowed)
}

pub fn replace_placeholders<F>(raw_value: &str, mut resolver: F) -> LauncherResult<String>
where
    F: FnMut(&str) -> LauncherResult<String>,
{
    let placeholder_pattern =
        Regex::new(r"\$\{([^}]+)\}").expect("placeholder regex should be valid");

    let mut resolved = String::with_capacity(raw_value.len());
    let mut cursor = 0usize;

    for capture in placeholder_pattern.captures_iter(raw_value) {
        let full_match = capture
            .get(0)
            .expect("placeholder capture should contain the full match");
        let placeholder = capture
            .get(1)
            .expect("placeholder capture should contain the placeholder name");

        resolved.push_str(&raw_value[cursor..full_match.start()]);
        resolved.push_str(&resolver(placeholder.as_str())?);
        cursor = full_match.end();
    }

    resolved.push_str(&raw_value[cursor..]);

    Ok(resolved)
}

fn rule_matches(rule: &Rule, environment: &RuntimeEnvironment) -> LauncherResult<bool> {
    if let Some(os_rule) = &rule.os {
        if let Some(name) = &os_rule.name {
            if name != &environment.os_name {
                return Ok(false);
            }
        }

        if let Some(arch) = &os_rule.arch {
            if arch != &environment.os_arch {
                return Ok(false);
            }
        }

        if let Some(version_pattern) = &os_rule.version {
            let Some(runtime_version) = environment.os_version.as_ref() else {
                return Ok(false);
            };
            let regex = Regex::new(version_pattern).map_err(|error| {
                LauncherError::new(format!("Invalid OS version rule regex: {error}"))
            })?;

            if !regex.is_match(runtime_version) {
                return Ok(false);
            }
        }
    }

    for (feature_name, expected_value) in &rule.features {
        let current_value = environment
            .features
            .get(feature_name)
            .copied()
            .unwrap_or(false);

        if current_value != *expected_value {
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{replace_placeholders, rules_allow, Rule, RuleAction, RuleOs, RuntimeEnvironment};

    #[test]
    fn allow_rule_matches_current_os() {
        let environment = RuntimeEnvironment {
            os_name: "windows".to_string(),
            os_arch: "x86_64".to_string(),
            arch_bits: "64".to_string(),
            os_version: None,
            features: HashMap::new(),
        };

        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: Some(RuleOs {
                name: Some("windows".to_string()),
                arch: None,
                version: None,
            }),
            features: HashMap::new(),
        }];

        assert!(rules_allow(&rules, &environment).expect("rules should evaluate"));
    }

    #[test]
    fn disallow_rule_overrides_allow() {
        let environment = RuntimeEnvironment {
            os_name: "windows".to_string(),
            os_arch: "x86_64".to_string(),
            arch_bits: "64".to_string(),
            os_version: None,
            features: HashMap::new(),
        };

        let rules = vec![
            Rule {
                action: RuleAction::Allow,
                os: Some(RuleOs {
                    name: Some("windows".to_string()),
                    arch: None,
                    version: None,
                }),
                features: HashMap::new(),
            },
            Rule {
                action: RuleAction::Disallow,
                os: Some(RuleOs {
                    name: Some("windows".to_string()),
                    arch: None,
                    version: None,
                }),
                features: HashMap::new(),
            },
        ];

        assert!(!rules_allow(&rules, &environment).expect("rules should evaluate"));
    }

    #[test]
    fn replace_placeholders_resolves_every_value() {
        let value = replace_placeholders(
            "--gameDir ${game_directory} --version ${version_name}",
            |placeholder| {
                Ok(match placeholder {
                    "game_directory" => "C:/Users/example/.minecraft".to_string(),
                    "version_name" => "1.20.1".to_string(),
                    _ => unreachable!("test placeholder"),
                })
            },
        )
        .expect("placeholders should resolve");

        assert_eq!(
            value,
            "--gameDir C:/Users/example/.minecraft --version 1.20.1"
        );
    }
}
