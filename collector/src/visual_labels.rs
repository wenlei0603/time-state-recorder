pub const IDENTITY_TAGS: &[&str] = &[
    "academic_researcher",
    "software_builder",
    "knowledge_manager",
    "teacher_mentor",
    "operator_admin",
    "investor_researcher",
    "personal_life",
    "unknown",
];

pub const ROUTINE_TAGS: &[&str] = &[
    "coding_build",
    "empirical_analysis",
    "paper_writing",
    "literature_reading",
    "teaching_coursework",
    "knowledge_capture",
    "notion_planning",
    "email_messaging",
    "meeting_discussion",
    "system_admin",
    "finance_research",
    "daily_life_admin",
    "learning_exploration",
    "break_or_entertainment",
    "idle_or_away",
    "unknown",
];

pub fn identity_tag_list_for_prompt() -> String {
    IDENTITY_TAGS.join(", ")
}

pub fn routine_tag_list_for_prompt() -> String {
    ROUTINE_TAGS.join(", ")
}

pub fn sanitize_identity_tags<I, S>(tags: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    sanitize_tags(tags, IDENTITY_TAGS)
}

pub fn sanitize_routine_tags<I, S>(tags: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    sanitize_tags(tags, ROUTINE_TAGS)
}

pub fn fallback_identity_tags(app: &str, title: &str) -> Vec<String> {
    let normalized = normalize_context(app, title);
    if has_any(
        &normalized,
        &["code", "cursor", "cargo", "rust", "typescript", "tsr"],
    ) {
        vec!["software_builder".to_string()]
    } else if has_any(
        &normalized,
        &["stata", "regression", "rstudio", "paper", "论文"],
    ) {
        vec!["academic_researcher".to_string()]
    } else if has_any(&normalized, &["notion", "diary", "principles os"]) {
        vec!["knowledge_manager".to_string()]
    } else if has_any(&normalized, &["wechat", "outlook", "mail"]) {
        vec!["operator_admin".to_string()]
    } else {
        vec!["unknown".to_string()]
    }
}

pub fn fallback_routine_tags(app: &str, title: &str) -> Vec<String> {
    let normalized = normalize_context(app, title);
    if has_any(
        &normalized,
        &["code", "cursor", "cargo", "rust", "typescript", "tsr"],
    ) {
        vec!["coding_build".to_string()]
    } else if has_any(
        &normalized,
        &["stata", "regression", "rstudio", "paper", "论文"],
    ) {
        vec!["empirical_analysis".to_string()]
    } else if has_any(&normalized, &["notion", "diary", "principles os"]) {
        vec!["knowledge_capture".to_string()]
    } else if has_any(
        &normalized,
        &[
            "browser", "pdf", "msedge", "edge", "chrome", "firefox", "safari",
        ],
    ) {
        vec!["literature_reading".to_string()]
    } else if has_any(&normalized, &["wechat", "outlook", "mail"]) {
        vec!["email_messaging".to_string()]
    } else {
        vec!["unknown".to_string()]
    }
}

fn sanitize_tags<I, S>(tags: I, allowed: &[&str]) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut cleaned = Vec::new();
    for tag in tags {
        let normalized = normalize(tag.as_ref());
        if allowed.iter().any(|allowed_tag| *allowed_tag == normalized)
            && !cleaned.contains(&normalized)
        {
            cleaned.push(normalized);
        }
    }
    if cleaned.is_empty() {
        vec!["unknown".to_string()]
    } else {
        cleaned
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_context(app: &str, title: &str) -> String {
    format!("{} {}", normalize(app), normalize(title))
}

fn has_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}
