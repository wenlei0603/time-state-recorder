# Structured Visual Insight Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make MiniMax-backed screenshot/window analysis and 5-hour reports structured, robust to fenced or unfenced JSON, tagged around the user's identities and daily routines, and rendered as human-readable project narratives instead of raw流水账.

**Architecture:** Add a shared backend JSON extraction layer and a stable visual-label taxonomy, then move report generation from one `summaryText` paragraph to a typed report contract. The frontend will consume typed presentation fields first and keep raw model text behind details/raw mode, so ` ```json ` wrappers, escaped newlines, and long single-paragraph report text do not leak into the product surface.

**Tech Stack:** Rust collector with `serde`, `serde_json`, `rusqlite`, `chrono`; React/TypeScript frontend with Vitest and Testing Library; existing SQLite migration style via `Store::ensure_column`.

---

## Goal Contract

### User-Facing Contract

- 5-minute window notes remain concise, but each sampled image gets labels for `identityTags`, `routineTags`, and `projectHints`.
- 5-hour reports are project-restoring narratives, not a concatenation of 5-minute notes.
- Reports should answer: what projects were active, what role/life identity the activity belonged to, how the work moved over time, where switching or drift happened, and what is uncertain.
- The UI never displays raw JSON fences, JSON object syntax, escaped `\n`, or model wrapper text as the primary report.
- Raw JSON remains available only in raw/details surfaces for diagnosis.

### Backend Output Contract

MiniMax visual window output must parse into:

```json
{
  "summaryText": "5分钟窗口的人类可读一句话。",
  "continuity": "continued_focus | resumed_project | context_switch | unclear",
  "primaryActivity": "project_work | research | writing | coding | communication | meeting | admin | learning | planning | loafing | personal | idle | unknown",
  "projectHints": ["Time State Recorder"],
  "identityTags": ["software_builder"],
  "routineTags": ["coding_build"],
  "taskIntent": "正在实现结构化视觉分析契约",
  "trajectory": [
    {
      "minuteMark": 1,
      "observation": "第1分钟可见Rust分析代码。",
      "activityCategory": "coding",
      "projectHints": ["Time State Recorder"],
      "identityTags": ["software_builder"],
      "routineTags": ["coding_build"]
    }
  ],
  "switchingLevel": "low | medium | high",
  "switchingEvidence": "同一项目内切换文件。",
  "loafingLevel": "none | possible | clear",
  "loafingEvidence": "未见娱乐或无关内容。",
  "visibleApps": ["Code.exe"],
  "visibleTextHints": ["visual_analysis.rs"],
  "riskFlags": [],
  "confidence": 0.86
}
```

MiniMax 5-hour report output must parse into:

```json
{
  "mainThread": "过去5小时主要围绕Time State Recorder的视觉分析和前端呈现迭代。",
  "summaryText": "兼容旧字段的一段人类可读摘要。",
  "timelinePhases": [
    {
      "label": "结构化分析契约",
      "start": "09:00",
      "end": "10:15",
      "narrative": "从后端prompt和JSON解析开始，随后检查前端呈现。",
      "projects": ["Time State Recorder"],
      "identityTags": ["software_builder"],
      "routineTags": ["coding_build"],
      "evidenceCount": 6
    }
  ],
  "projectNarratives": [
    {
      "project": "Time State Recorder",
      "narrative": "核心工作是把5分钟窗口摘要升级为可供5小时报告聚合的结构化数据。",
      "identityTags": ["software_builder"],
      "routineTags": ["coding_build"],
      "evidenceCount": 12
    }
  ],
  "attentionPattern": "整体低切换，主要在同一工程链路内移动。",
  "uncertainty": "只有屏幕证据，不能确认离屏活动。",
  "projectHints": ["Time State Recorder"],
  "identityTags": ["software_builder"],
  "routineTags": ["coding_build"]
}
```

### Label Taxonomy Contract

Use stable string tags rather than Rust enums so the taxonomy can be adjusted without database churn.

Identity tags:

```text
academic_researcher
software_builder
knowledge_manager
teacher_mentor
operator_admin
investor_researcher
personal_life
unknown
```

Routine tags:

```text
coding_build
empirical_analysis
paper_writing
literature_reading
teaching_coursework
knowledge_capture
notion_planning
email_messaging
meeting_discussion
system_admin
finance_research
daily_life_admin
learning_exploration
break_or_entertainment
idle_or_away
unknown
```

### MiniMax Request-Format Contract

- Do not unconditionally send `response_format` for `MiniMax-M3`; MiniMax's official text-generation docs state `response_format` is only supported by `MiniMax-Text-01`.
- For `MiniMax-M3`, improve `system` and user prompt schema: "single JSON object", "no markdown fences", "no commentary", "use only listed labels".
- Add a small helper that includes `response_format` only when the configured model is `MiniMax-Text-01`.
- Keep parser tolerant of ` ```json `, bare JSON, leading/trailing text, and escaped-newline JSON strings.

### Non-Goals

- Do not change screenshot capture cadence or retention policy.
- Do not build a Notion export in this iteration.
- Do not infer private identity beyond the visible screen evidence and the configured taxonomy.
- Do not redesign the whole dashboard layout; constrain frontend work to Review Notes and Daily Brief report presentation.

### Acceptance Criteria

- Rust tests show fenced JSON, unfenced JSON, and prefixed/suffixed JSON all parse for visual window, 5-hour report, and daily brief responses.
- Rust tests show MiniMax-M3 requests do not include unsupported `response_format`, while MiniMax-Text-01 requests do.
- Rust tests show window summaries round-trip `identityTags` and `routineTags`.
- Rust tests show 5-hour reports group consecutive 5-minute windows into project phases instead of listing every window.
- TypeScript tests show report presentation uses `mainThread`, `timelinePhases`, and `projectNarratives` before falling back to `summaryText`.
- App tests show raw JSON fences and escaped newlines do not appear in Review Notes or Daily Brief primary UI.
- Verification commands pass: `cargo test -p tsr-collector`, `npm test`, `npm run build`.

---

## File Structure

- Create: `collector/src/llm_json.rs`
  - Shared JSON extraction and optional `response_format` helper for MiniMax text/chat responses.
- Create: `collector/src/visual_labels.rs`
  - Stable identity/routine tag lists, sanitizers, and local metadata fallback helpers.
- Modify: `collector/src/lib.rs`
  - Export the new modules.
- Modify: `collector/src/models.rs`
  - Add label fields to visual summaries, trajectory points, and insight reports.
  - Add typed 5-hour report structs.
- Modify: `collector/src/visual_analysis.rs`
  - Use shared JSON parser.
  - Upgrade system prompts and user prompt schema.
  - Parse and store image/window labels.
- Modify: `collector/src/insights.rs`
  - Use shared JSON parser.
  - Generate structured 5-hour project reports from window summaries.
  - Improve MiniMax report prompt and model parsing.
- Modify: `collector/src/storage.rs`
  - Add SQLite columns with `ensure_column`.
  - Persist and hydrate label/report-structure JSON.
- Modify: `collector/src/api.rs`
  - No new routes required; existing JSON responses expand through model serialization.
- Modify: `src/types.ts`
  - Mirror added API fields.
- Modify: `src/lib/insights.ts`
  - Validate expanded fields with default-compatible fallbacks.
- Modify: `src/lib/insightPresentation.ts`
  - Build presentation from structured report fields first.
- Modify: `src/InsightFeedback.tsx`
  - Render project phases, project narratives, identity/routine chips.
- Modify: `src/DailyBriefPanel.tsx`
  - Reuse report presentation instead of printing `report.summaryText`.
- Modify: `src/styles.css`
  - Add stable layouts for project report sections and long text wrapping.
- Test: `collector/tests/llm_json_tests.rs`
- Test: `collector/tests/visual_analysis_tests.rs`
- Test: `collector/tests/insight_tests.rs`
- Test: `collector/tests/storage_tests.rs`
- Test: `src/lib/insightPresentation.test.ts`
- Test: `src/lib/insights.test.ts`
- Test: `src/App.test.tsx`

---

### Task 1: Shared JSON Extraction And MiniMax Request Helper

**Files:**
- Create: `collector/src/llm_json.rs`
- Modify: `collector/src/lib.rs`
- Modify: `collector/src/visual_analysis.rs`
- Modify: `collector/src/insights.rs`
- Test: `collector/tests/llm_json_tests.rs`

- [ ] **Step 1: Write the failing parser tests**

Create `collector/tests/llm_json_tests.rs`:

```rust
use tsr_collector::llm_json::{
    extract_json_object_text, minimax_json_response_format, parse_json_object,
};

#[test]
fn extracts_fenced_json_object() {
    let text = "```json\n{\"summaryText\":\"完成分析\"}\n```";

    assert_eq!(
        extract_json_object_text(text).unwrap(),
        "{\"summaryText\":\"完成分析\"}"
    );
    assert_eq!(
        parse_json_object(text).unwrap()["summaryText"],
        "完成分析"
    );
}

#[test]
fn extracts_unfenced_json_object() {
    let text = "{\"summaryText\":\"完成分析\",\"confidence\":0.8}";

    assert_eq!(
        parse_json_object(text).unwrap()["confidence"].as_f64().unwrap(),
        0.8
    );
}

#[test]
fn extracts_json_object_from_wrapper_text() {
    let text = "Here is the JSON:\n{\"summaryText\":\"完成分析\"}\nNo more.";

    assert_eq!(
        parse_json_object(text).unwrap()["summaryText"],
        "完成分析"
    );
}

#[test]
fn decodes_escaped_newline_json_string() {
    let text = "\"{\\n  \\\"summaryText\\\": \\\"完成分析\\\"\\n}\"";

    assert_eq!(
        parse_json_object(text).unwrap()["summaryText"],
        "完成分析"
    );
}

#[test]
fn response_format_is_only_sent_for_minimax_text_01() {
    assert!(minimax_json_response_format("MiniMax-M3").is_none());
    assert!(minimax_json_response_format("MiniMax-M2.7").is_none());
    assert_eq!(
        minimax_json_response_format("MiniMax-Text-01").unwrap()["type"],
        "json_object"
    );
}
```

- [ ] **Step 2: Run parser tests to verify they fail**

Run:

```powershell
cargo test -p tsr-collector --test llm_json_tests
```

Expected: FAIL because `tsr_collector::llm_json` does not exist.

- [ ] **Step 3: Add the shared JSON parser**

Create `collector/src/llm_json.rs`:

```rust
use serde::de::DeserializeOwned;

pub fn minimax_json_response_format(model: &str) -> Option<serde_json::Value> {
    if model.trim().eq_ignore_ascii_case("MiniMax-Text-01") {
        Some(serde_json::json!({ "type": "json_object" }))
    } else {
        None
    }
}

pub fn parse_json_object(content: &str) -> Option<serde_json::Value> {
    let object_text = extract_json_object_text(content)?;
    serde_json::from_str(&object_text).ok()
}

pub fn parse_json_object_as<T: DeserializeOwned>(content: &str) -> Option<T> {
    let value = parse_json_object(content)?;
    serde_json::from_value(value).ok()
}

pub fn extract_json_object_text(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(decoded) = decode_json_string(trimmed) {
        if decoded.trim() != trimmed {
            if let Some(value) = extract_json_object_text(&decoded) {
                return Some(value);
            }
        }
    }

    let unfenced = remove_code_fence(trimmed);
    if serde_json::from_str::<serde_json::Value>(&unfenced)
        .ok()
        .filter(|value| value.is_object())
        .is_some()
    {
        return Some(unfenced);
    }

    balanced_object_slice(&unfenced)
}

pub fn remove_code_fence(content: &str) -> String {
    let trimmed = content.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("```json") {
        return trimmed[7..]
            .trim()
            .trim_end_matches("```")
            .trim()
            .to_string();
    }
    if trimmed.starts_with("```") {
        return trimmed[3..]
            .trim()
            .trim_end_matches("```")
            .trim()
            .to_string();
    }
    trimmed.to_string()
}

fn decode_json_string(value: &str) -> Option<String> {
    serde_json::from_str::<String>(value).ok()
}

fn balanced_object_slice(value: &str) -> Option<String> {
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    let start = start?;
                    let end = index + ch.len_utf8();
                    let candidate = value[start..end].trim().to_string();
                    if serde_json::from_str::<serde_json::Value>(&candidate)
                        .ok()
                        .filter(|parsed| parsed.is_object())
                        .is_some()
                    {
                        return Some(candidate);
                    }
                }
            }
            _ => {}
        }
    }

    None
}
```

- [ ] **Step 4: Export the parser module**

Modify `collector/src/lib.rs` to include:

```rust
pub mod llm_json;
```

- [ ] **Step 5: Use the parser in `visual_analysis.rs`**

Replace local `parse_model_summary_json`, `parse_model_window_summary_value`, and `strip_json_fence` usage with:

```rust
fn parse_model_summary_json(content: &str) -> Option<ModelSummaryJson> {
    crate::llm_json::parse_json_object_as(content)
}

fn parse_model_window_summary_value(content: &str) -> Option<serde_json::Value> {
    crate::llm_json::parse_json_object(content)
}
```

Delete the local `strip_json_fence` function from `collector/src/visual_analysis.rs`.

- [ ] **Step 6: Use the parser in `insights.rs`**

Replace `parse_model_report_json`, `parse_model_daily_brief_json`, and `strip_json_fence` usage with:

```rust
fn parse_model_report_json(content: &str) -> Option<ModelReportJson> {
    crate::llm_json::parse_json_object_as(content)
}

fn parse_model_daily_brief_json(content: &str) -> Option<ModelDailyBriefJson> {
    crate::llm_json::parse_json_object_as(content)
}
```

In `MiniMaxDailyBriefReporter::brief_from_response_text`, replace raw JSON storage with:

```rust
raw_summary_json: crate::llm_json::parse_json_object(content)
    .unwrap_or_else(|| serde_json::json!({ "content": content.trim() })),
```

Delete the local `strip_json_fence` function from `collector/src/insights.rs`.

- [ ] **Step 7: Add conditional `response_format` to MiniMax request builders**

In each MiniMax request builder after constructing the `serde_json::json!` request, assign to a mutable value and add:

```rust
if let Some(response_format) = crate::llm_json::minimax_json_response_format(&self.config.model) {
    body["response_format"] = response_format;
}
```

Use this in:

- `MiniMaxAnalyzer::build_chat_completions_request`
- `MiniMaxAnalyzer::build_window_chat_completions_request`
- `MiniMaxInsightReporter::build_chat_completions_request`
- `MiniMaxInsightReporter::build_window_summary_chat_completions_request`
- `MiniMaxDailyBriefReporter::build_chat_completions_request`

- [ ] **Step 8: Run parser tests**

Run:

```powershell
cargo test -p tsr-collector --test llm_json_tests
```

Expected: PASS.

- [ ] **Step 9: Run existing affected backend tests**

Run:

```powershell
cargo test -p tsr-collector --test visual_analysis_tests --test insight_tests
```

Expected: PASS.

- [ ] **Step 10: Commit**

```powershell
git add collector/src/llm_json.rs collector/src/lib.rs collector/src/visual_analysis.rs collector/src/insights.rs collector/tests/llm_json_tests.rs
git commit -m "feat: harden MiniMax JSON extraction"
```

---

### Task 2: Visual Label Taxonomy And Window-Level Image Labels

**Files:**
- Create: `collector/src/visual_labels.rs`
- Modify: `collector/src/lib.rs`
- Modify: `collector/src/models.rs`
- Modify: `collector/src/visual_analysis.rs`
- Test: `collector/tests/visual_analysis_tests.rs`

- [ ] **Step 1: Write failing tests for labels in window output**

Append to `collector/tests/visual_analysis_tests.rs`:

```rust
#[test]
fn minimax_window_analysis_response_maps_identity_and_routine_tags() {
    let samples = vec![
        high_res_screenshot(1, "2026-06-03T10:00:10Z", "Code.exe"),
        high_res_screenshot(3, "2026-06-03T10:02:05Z", "Code.exe"),
        high_res_screenshot(5, "2026-06-03T10:04:15Z", "msedge.exe"),
    ];

    let summary = MiniMaxAnalyzer::window_summary_from_response_text(
        ts("2026-06-03T10:00:00Z"),
        ts("2026-06-03T10:05:00Z"),
        &select_window_samples(
            ts("2026-06-03T10:00:00Z"),
            ts("2026-06-03T10:05:00Z"),
            &samples,
        )
        .unwrap(),
        None,
        ts("2026-06-03T10:05:30Z"),
        "MiniMax-M3",
        r#"```json
        {
          "summaryText": "持续实现结构化视觉分析。",
          "continuity": "continued_focus",
          "primaryActivity": "coding",
          "projectHints": ["Time State Recorder"],
          "identityTags": ["software_builder", "not_a_real_tag"],
          "routineTags": ["coding_build"],
          "taskIntent": "实现窗口标签",
          "trajectory": [
            {
              "minuteMark": 1,
              "observation": "编辑Rust代码",
              "activityCategory": "coding",
              "projectHints": ["Time State Recorder"],
              "identityTags": ["software_builder"],
              "routineTags": ["coding_build"]
            },
            {
              "minuteMark": 3,
              "observation": "检查prompt",
              "activityCategory": "coding",
              "projectHints": ["Time State Recorder"],
              "identityTags": ["software_builder"],
              "routineTags": ["coding_build"]
            },
            {
              "minuteMark": 5,
              "observation": "阅读API约束",
              "activityCategory": "research",
              "projectHints": ["Time State Recorder"],
              "identityTags": ["software_builder"],
              "routineTags": ["learning_exploration"]
            }
          ],
          "switchingLevel": "low",
          "switchingEvidence": "同一项目内切换。",
          "loafingLevel": "none",
          "loafingEvidence": "没有无关内容。",
          "visibleApps": ["Code.exe", "msedge.exe"],
          "visibleTextHints": ["visual_analysis.rs"],
          "riskFlags": [],
          "confidence": 0.86
        }
        ```"#,
    )
    .unwrap();

    assert_eq!(summary.identity_tags, vec!["software_builder"]);
    assert_eq!(summary.routine_tags, vec!["coding_build"]);
    assert_eq!(summary.trajectory[0].identity_tags, vec!["software_builder"]);
    assert_eq!(summary.trajectory[2].routine_tags, vec!["learning_exploration"]);
}
```

- [ ] **Step 2: Run the failing visual analysis test**

Run:

```powershell
cargo test -p tsr-collector --test visual_analysis_tests minimax_window_analysis_response_maps_identity_and_routine_tags
```

Expected: FAIL because label fields do not exist.

- [ ] **Step 3: Add taxonomy helpers**

Create `collector/src/visual_labels.rs`:

```rust
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

pub fn sanitize_identity_tags(values: Option<Vec<String>>) -> Vec<String> {
    sanitize_tags(values, IDENTITY_TAGS)
}

pub fn sanitize_routine_tags(values: Option<Vec<String>>) -> Vec<String> {
    sanitize_tags(values, ROUTINE_TAGS)
}

fn sanitize_tags(values: Option<Vec<String>>, allowed: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for value in values.unwrap_or_default() {
        let normalized = value.trim().to_ascii_lowercase();
        if allowed.contains(&normalized.as_str()) && !out.contains(&normalized) {
            out.push(normalized);
        }
    }
    if out.is_empty() {
        out.push("unknown".to_string());
    }
    out
}

pub fn fallback_identity_tags(app: &str, title: &str) -> Vec<String> {
    let combined = format!("{} {}", app.to_ascii_lowercase(), title.to_ascii_lowercase());
    if contains_any(&combined, &["code", "cursor", "cargo", "rust", "typescript", "tsr"]) {
        vec!["software_builder".to_string()]
    } else if contains_any(&combined, &["stata", "r studio", "rstudio", "论文", "paper", "regression"]) {
        vec!["academic_researcher".to_string()]
    } else if contains_any(&combined, &["notion", "diary", "principles os"]) {
        vec!["knowledge_manager".to_string()]
    } else if contains_any(&combined, &["wechat", "weixin", "outlook", "mail"]) {
        vec!["operator_admin".to_string()]
    } else {
        vec!["unknown".to_string()]
    }
}

pub fn fallback_routine_tags(app: &str, title: &str) -> Vec<String> {
    let combined = format!("{} {}", app.to_ascii_lowercase(), title.to_ascii_lowercase());
    if contains_any(&combined, &["code", "cursor", "cargo", "rust", "typescript"]) {
        vec!["coding_build".to_string()]
    } else if contains_any(&combined, &["stata", "regression", "r studio", "rstudio"]) {
        vec!["empirical_analysis".to_string()]
    } else if contains_any(&combined, &["notion", "diary", "principles os"]) {
        vec!["knowledge_capture".to_string()]
    } else if contains_any(&combined, &["chrome", "msedge", "edge", "browser", "pdf"]) {
        vec!["literature_reading".to_string()]
    } else if contains_any(&combined, &["wechat", "weixin", "outlook", "mail"]) {
        vec!["email_messaging".to_string()]
    } else {
        vec!["unknown".to_string()]
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}
```

Modify `collector/src/lib.rs`:

```rust
pub mod visual_labels;
```

- [ ] **Step 4: Extend model structs**

In `collector/src/models.rs`, add fields:

```rust
pub struct VisualSummary {
    ...
    pub project_hints: Vec<String>,
    pub identity_tags: Vec<String>,
    pub routine_tags: Vec<String>,
    pub visible_apps: Vec<String>,
    ...
}

pub struct VisualObservation {
    ...
    pub project_hints: Vec<String>,
    pub identity_tags: Vec<String>,
    pub routine_tags: Vec<String>,
    pub visible_apps: Vec<String>,
    ...
}

pub struct VisualTrajectoryPoint {
    pub minute_mark: u8,
    pub screenshot_id: i64,
    pub observation: String,
    pub activity_category: ActivityCategory,
    pub project_hints: Vec<String>,
    pub identity_tags: Vec<String>,
    pub routine_tags: Vec<String>,
}

pub struct VisualWindowSummary {
    ...
    pub project_hints: Vec<String>,
    pub identity_tags: Vec<String>,
    pub routine_tags: Vec<String>,
    pub task_intent: String,
    ...
}
```

- [ ] **Step 5: Extend MiniMax response structs**

In `collector/src/visual_analysis.rs`, add:

```rust
identity_tags: Option<Vec<String>>,
routine_tags: Option<Vec<String>>,
```

to `ModelSummaryJson` and `ModelWindowSummaryJson`.

Extend `ModelTrajectoryPointJson`:

```rust
project_hints: Option<Vec<String>>,
identity_tags: Option<Vec<String>>,
routine_tags: Option<Vec<String>>,
```

- [ ] **Step 6: Map labels in local fallback and MiniMax parsing**

In `local_stub_visual_summary`, compute:

```rust
let identity_tags = crate::visual_labels::fallback_identity_tags(&app, &title);
let routine_tags = crate::visual_labels::fallback_routine_tags(&app, &title);
```

In `summary_from_response_text`, map:

```rust
identity_tags: crate::visual_labels::sanitize_identity_tags(
    parsed.as_ref().and_then(|value| value.identity_tags.clone()),
),
routine_tags: crate::visual_labels::sanitize_routine_tags(
    parsed.as_ref().and_then(|value| value.routine_tags.clone()),
),
```

Use local fallback values if parsing fails by passing `Some(local_fallback.identity_tags)` and `Some(local_fallback.routine_tags)`.

In `local_stub_visual_window_summary`, assign summary-level labels by deduping trajectory labels and use fallback labels for each trajectory point:

```rust
project_hints: project_hints_from_metadata(&app, &title),
identity_tags: crate::visual_labels::fallback_identity_tags(&app, &title),
routine_tags: crate::visual_labels::fallback_routine_tags(&app, &title),
```

In `window_summary_from_response_text`, set:

```rust
identity_tags: crate::visual_labels::sanitize_identity_tags(
    parsed.as_ref().and_then(|value| value.identity_tags.clone()),
),
routine_tags: crate::visual_labels::sanitize_routine_tags(
    parsed.as_ref().and_then(|value| value.routine_tags.clone()),
),
```

- [ ] **Step 7: Extend trajectory conversion**

Update `model_trajectory_to_points`:

```rust
VisualTrajectoryPoint {
    minute_mark: item.minute_mark,
    screenshot_id: sample.screenshot.id,
    observation: item.observation.clone(),
    activity_category: item
        .activity_category
        .as_deref()
        .and_then(ActivityCategory::from_db)
        .unwrap_or(ActivityCategory::Unknown),
    project_hints: dedupe_strings(item.project_hints.clone().unwrap_or_default()),
    identity_tags: crate::visual_labels::sanitize_identity_tags(item.identity_tags.clone()),
    routine_tags: crate::visual_labels::sanitize_routine_tags(item.routine_tags.clone()),
}
```

- [ ] **Step 8: Upgrade visual prompts**

Replace the system message for window analysis with:

```rust
"You analyze three screenshots from a 5-minute personal desktop window. Return one valid JSON object only. Do not wrap it in markdown fences. Do not include commentary before or after JSON. Use only the provided label values."
```

Update `window_summary_prompt` to include:

```rust
format!(
    "Analyze this 5-minute work window using exactly three screenshots from minute marks 1, 3, and 5. Use previousWindowSummary only as continuity context. Return one JSON object with keys: summaryText, continuity, primaryActivity, projectHints, identityTags, routineTags, taskIntent, trajectory, switchingLevel, switchingEvidence, loafingLevel, loafingEvidence, visibleApps, visibleTextHints, riskFlags, confidence. identityTags must use only [{}]. routineTags must use only [{}]. Each trajectory item must include minuteMark, observation, activityCategory, projectHints, identityTags, routineTags. Write human-facing strings in Chinese. windowStart={}, windowEnd={}, previousWindowSummary={}, samples={}",
    crate::visual_labels::identity_tag_list_for_prompt(),
    crate::visual_labels::routine_tag_list_for_prompt(),
    input.window_start.to_rfc3339(),
    input.window_end.to_rfc3339(),
    previous_summary
        .map(|value| serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()))
        .unwrap_or_else(|| "null".to_string()),
    serde_json::to_string(&samples_json).unwrap_or_else(|_| "[]".to_string())
)
```

- [ ] **Step 9: Run visual tests**

Run:

```powershell
cargo test -p tsr-collector --test visual_analysis_tests
```

Expected: PASS after updating existing sample constructors for the new fields.

- [ ] **Step 10: Commit**

```powershell
git add collector/src/visual_labels.rs collector/src/lib.rs collector/src/models.rs collector/src/visual_analysis.rs collector/tests/visual_analysis_tests.rs
git commit -m "feat: add identity and routine labels to visual analysis"
```

---

### Task 3: Storage, API, And TypeScript Contract Expansion

**Files:**
- Modify: `collector/src/storage.rs`
- Modify: `collector/tests/storage_tests.rs`
- Modify: `collector/tests/api_tests.rs`
- Modify: `src/types.ts`
- Modify: `src/lib/insights.ts`
- Test: `src/lib/insights.test.ts`

- [ ] **Step 1: Write failing storage round-trip assertions**

In `collector/tests/storage_tests.rs`, update the existing `visual_window_summaries_round_trip_by_window` test to include:

```rust
assert_eq!(rows[0].identity_tags, vec!["software_builder"]);
assert_eq!(rows[0].routine_tags, vec!["coding_build"]);
assert_eq!(rows[0].trajectory[0].identity_tags, vec!["software_builder"]);
assert_eq!(rows[0].trajectory[0].routine_tags, vec!["coding_build"]);
```

Update the sample `VisualWindowSummary` in that test to set:

```rust
identity_tags: vec!["software_builder".into()],
routine_tags: vec!["coding_build".into()],
```

and each trajectory point:

```rust
project_hints: vec!["Time State Recorder".into()],
identity_tags: vec!["software_builder".into()],
routine_tags: vec!["coding_build".into()],
```

- [ ] **Step 2: Run storage tests to verify failure**

Run:

```powershell
cargo test -p tsr-collector --test storage_tests visual_window_summaries_round_trip_by_window
```

Expected: FAIL until storage columns and row mapping are updated.

- [ ] **Step 3: Add SQLite columns**

In `Store::init`, add columns to `CREATE TABLE` statements for fresh databases:

```sql
identity_tags_json TEXT NOT NULL DEFAULT '["unknown"]',
routine_tags_json TEXT NOT NULL DEFAULT '["unknown"]',
```

Add them to:

- `visual_summaries`
- `visual_observations`
- `visual_window_summaries`

After current `ensure_column` calls, add:

```rust
self.ensure_column(
    "visual_summaries",
    "identity_tags_json",
    "TEXT NOT NULL DEFAULT '[\"unknown\"]'",
)?;
self.ensure_column(
    "visual_summaries",
    "routine_tags_json",
    "TEXT NOT NULL DEFAULT '[\"unknown\"]'",
)?;
self.ensure_column(
    "visual_observations",
    "identity_tags_json",
    "TEXT NOT NULL DEFAULT '[\"unknown\"]'",
)?;
self.ensure_column(
    "visual_observations",
    "routine_tags_json",
    "TEXT NOT NULL DEFAULT '[\"unknown\"]'",
)?;
self.ensure_column(
    "visual_window_summaries",
    "identity_tags_json",
    "TEXT NOT NULL DEFAULT '[\"unknown\"]'",
)?;
self.ensure_column(
    "visual_window_summaries",
    "routine_tags_json",
    "TEXT NOT NULL DEFAULT '[\"unknown\"]'",
)?;
```

- [ ] **Step 4: Persist and hydrate visual labels**

Update INSERT column lists and parameter lists in:

- `insert_visual_summary`
- `insert_visual_observation`
- `insert_visual_window_summary`

Use:

```rust
serde_json::to_string(&summary.identity_tags)?,
serde_json::to_string(&summary.routine_tags)?,
```

Update SELECT lists and row mappers:

```rust
identity_tags: parse_string_vec(&identity_tags_json)?,
routine_tags: parse_string_vec(&routine_tags_json)?,
```

For trajectory JSON, the existing `parse_visual_trajectory` continues to work after `VisualTrajectoryPoint` gains fields.

- [ ] **Step 5: Add API response assertions**

In `collector/tests/api_tests.rs`, update `serves_visual_window_summaries_for_date`:

```rust
assert_eq!(body["summaries"][0]["identityTags"][0], "software_builder");
assert_eq!(body["summaries"][0]["routineTags"][0], "coding_build");
assert_eq!(
    body["summaries"][0]["trajectory"][0]["identityTags"][0],
    "software_builder"
);
```

- [ ] **Step 6: Expand TypeScript types**

In `src/types.ts`, update:

```ts
export type VisualSummary = {
  ...
  projectHints: string[];
  identityTags: string[];
  routineTags: string[];
  visibleApps: string[];
  ...
};

export type VisualObservation = {
  ...
  projectHints: string[];
  identityTags: string[];
  routineTags: string[];
  visibleApps: string[];
  ...
};

export type VisualTrajectoryPoint = {
  minuteMark: number;
  screenshotId: number;
  observation: string;
  activityCategory: ActivityCategory;
  projectHints: string[];
  identityTags: string[];
  routineTags: string[];
};

export type VisualWindowSummary = {
  ...
  projectHints: string[];
  identityTags: string[];
  routineTags: string[];
  taskIntent: string;
  ...
};
```

- [ ] **Step 7: Add backward-compatible frontend parsing**

In `src/lib/insights.ts`, for `toVisualSummary`, `toVisualObservation`, and `toVisualWindowSummary`, read:

```ts
identityTags: readOptionalStringArray(value, "identityTags", ["unknown"]),
routineTags: readOptionalStringArray(value, "routineTags", ["unknown"]),
```

Add helper:

```ts
function readOptionalStringArray(
  record: Record<string, unknown>,
  key: string,
  fallback: string[],
): string[] {
  const value = record[key];
  if (value === null || value === undefined) {
    return fallback;
  }
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new Error(`API row has invalid ${key}`);
  }
  return value;
}
```

Keep existing strict `readStringArray` for required legacy fields.

- [ ] **Step 8: Run storage, API, and TS parsing tests**

Run:

```powershell
cargo test -p tsr-collector --test storage_tests --test api_tests
npm test -- src/lib/insights.test.ts
```

Expected: PASS.

- [ ] **Step 9: Commit**

```powershell
git add collector/src/storage.rs collector/tests/storage_tests.rs collector/tests/api_tests.rs src/types.ts src/lib/insights.ts src/lib/insights.test.ts
git commit -m "feat: persist visual identity routine labels"
```

---

### Task 4: Structured 5-Hour Project Reports

**Files:**
- Modify: `collector/src/models.rs`
- Modify: `collector/src/insights.rs`
- Modify: `collector/src/storage.rs`
- Test: `collector/tests/insight_tests.rs`
- Test: `collector/tests/storage_tests.rs`

- [ ] **Step 1: Write failing report structure test**

Append to `collector/tests/insight_tests.rs`:

```rust
#[test]
fn five_hour_report_groups_window_summaries_into_project_phases() {
    let windows = vec![
        sample_window_summary(
            1,
            "2026-06-03T05:00:00Z",
            "2026-06-03T05:05:00Z",
            vec![1, 3, 5],
            "开始实现结构化JSON解析。",
        ),
        sample_window_summary(
            2,
            "2026-06-03T05:05:00Z",
            "2026-06-03T05:10:00Z",
            vec![6, 8, 10],
            "继续实现结构化JSON解析。",
        ),
        sample_window_summary_with_project(
            3,
            "2026-06-03T06:00:00Z",
            "2026-06-03T06:05:00Z",
            vec![11, 13, 15],
            "阅读AMR论文。",
            "AMR reading",
            "academic_researcher",
            "literature_reading",
        ),
    ];

    let report = build_five_hour_report_from_window_summaries(
        ts("2026-06-03T05:00:00Z"),
        ts("2026-06-03T10:00:00Z"),
        &windows,
    );

    assert_eq!(report.timeline_phases.len(), 2);
    assert_eq!(report.timeline_phases[0].projects, vec!["Time State Recorder"]);
    assert_eq!(report.timeline_phases[0].evidence_count, 2);
    assert_eq!(report.project_narratives.len(), 2);
    assert!(report.main_thread.contains("Time State Recorder"));
    assert!(!report.summary_text.contains("\\n"));
}
```

Add helper near existing test helpers:

```rust
fn sample_window_summary_with_project(
    id: i64,
    start: &str,
    end: &str,
    screenshot_ids: Vec<i64>,
    summary_text: &str,
    project: &str,
    identity_tag: &str,
    routine_tag: &str,
) -> VisualWindowSummary {
    let mut summary = sample_window_summary(id, start, end, screenshot_ids, summary_text);
    summary.project_hints = vec![project.into()];
    summary.identity_tags = vec![identity_tag.into()];
    summary.routine_tags = vec![routine_tag.into()];
    summary
}
```

- [ ] **Step 2: Run failing report test**

Run:

```powershell
cargo test -p tsr-collector --test insight_tests five_hour_report_groups_window_summaries_into_project_phases
```

Expected: FAIL because `InsightReport` has no typed phase fields.

- [ ] **Step 3: Add report structs and fields**

In `collector/src/models.rs`, add:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTimelinePhase {
    pub label: String,
    pub start: String,
    pub end: String,
    pub narrative: String,
    pub projects: Vec<String>,
    pub identity_tags: Vec<String>,
    pub routine_tags: Vec<String>,
    pub evidence_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectNarrative {
    pub project: String,
    pub narrative: String,
    pub identity_tags: Vec<String>,
    pub routine_tags: Vec<String>,
    pub evidence_count: usize,
}
```

Extend `InsightReport`:

```rust
pub main_thread: String,
pub timeline_phases: Vec<ReportTimelinePhase>,
pub project_narratives: Vec<ProjectNarrative>,
pub attention_pattern: String,
pub uncertainty: String,
pub identity_tags: Vec<String>,
pub routine_tags: Vec<String>,
pub raw_summary_json: serde_json::Value,
```

- [ ] **Step 4: Add report storage columns**

In `insight_reports` table creation:

```sql
main_thread TEXT NOT NULL DEFAULT '',
timeline_phases_json TEXT NOT NULL DEFAULT '[]',
project_narratives_json TEXT NOT NULL DEFAULT '[]',
attention_pattern TEXT NOT NULL DEFAULT '',
uncertainty TEXT NOT NULL DEFAULT '',
identity_tags_json TEXT NOT NULL DEFAULT '["unknown"]',
routine_tags_json TEXT NOT NULL DEFAULT '["unknown"]',
raw_summary_json TEXT NOT NULL DEFAULT '{}',
```

In `Store::init`, add `ensure_column` calls for each column.

Update `insert_insight_report`, `list_insight_reports`, `list_insight_reports_between`, and `map_insight_report_row`.

- [ ] **Step 5: Build local structured report**

In `collector/src/insights.rs`, implement:

```rust
fn window_report_structure(
    window_summaries: &[VisualWindowSummary],
) -> (
    String,
    Vec<ReportTimelinePhase>,
    Vec<ProjectNarrative>,
    String,
    String,
    Vec<String>,
    Vec<String>,
    serde_json::Value,
) {
    if window_summaries.is_empty() {
        let uncertainty = "没有可用的5分钟窗口摘要。".to_string();
        return (
            "过去5小时没有足够证据还原项目轨迹。".to_string(),
            Vec::new(),
            Vec::new(),
            "证据不足。".to_string(),
            uncertainty.clone(),
            vec!["unknown".to_string()],
            vec!["unknown".to_string()],
            serde_json::json!({ "uncertainty": uncertainty }),
        );
    }

    let phases = coalesce_project_phases(window_summaries);
    let projects = project_narratives_from_phases(&phases);
    let identity_tags = dedupe_strings(
        phases.iter().flat_map(|phase| phase.identity_tags.clone()).collect(),
    );
    let routine_tags = dedupe_strings(
        phases.iter().flat_map(|phase| phase.routine_tags.clone()).collect(),
    );
    let main_thread = if projects.is_empty() {
        format!("过去5小时主要由{}个活动阶段构成，但项目归属不明确。", phases.len())
    } else {
        format!(
            "过去5小时主要围绕{}展开，共形成{}个可辨认阶段。",
            projects
                .iter()
                .map(|project| project.project.as_str())
                .collect::<Vec<_>>()
                .join("、"),
            phases.len()
        )
    };
    let high_switch_count = window_summaries
        .iter()
        .filter(|summary| summary.switching_level == "high")
        .count();
    let attention_pattern = if high_switch_count == 0 {
        "整体切换较低，主要在少数项目内推进。".to_string()
    } else {
        format!("检测到{high_switch_count}个高切换窗口，项目推进存在碎片化。")
    };
    let uncertainty = "报告仅基于屏幕窗口、截图摘要和模型标签，不能覆盖离屏活动。".to_string();
    let raw = serde_json::json!({
        "mainThread": main_thread,
        "timelinePhases": phases,
        "projectNarratives": projects,
        "attentionPattern": attention_pattern,
        "uncertainty": uncertainty,
        "identityTags": identity_tags,
        "routineTags": routine_tags
    });

    (
        main_thread,
        phases,
        projects,
        attention_pattern,
        uncertainty,
        identity_tags,
        routine_tags,
        raw,
    )
}
```

Implement `coalesce_project_phases` and `project_narratives_from_phases` directly below it. Coalesce adjacent windows when their first project hint matches; otherwise start a new phase. Limit phase labels to project name or primary activity.

- [ ] **Step 6: Return structured local report**

In `build_five_hour_report_from_window_summaries`, call `window_report_structure` and set:

```rust
summary_text: summary_text_from_report_parts(&main_thread, &timeline_phases, &attention_pattern, &uncertainty),
main_thread,
timeline_phases,
project_narratives,
attention_pattern,
uncertainty,
identity_tags,
routine_tags,
raw_summary_json,
```

Use:

```rust
fn summary_text_from_report_parts(
    main_thread: &str,
    phases: &[ReportTimelinePhase],
    attention_pattern: &str,
    uncertainty: &str,
) -> String {
    let phase_text = phases
        .iter()
        .take(4)
        .enumerate()
        .map(|(index, phase)| {
            format!(
                "{}. {}({}-{})：{}",
                index + 1,
                phase.label,
                phase.start,
                phase.end,
                phase.narrative
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    [main_thread, &phase_text, attention_pattern, uncertainty]
        .iter()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim())
        .collect::<Vec<_>>()
        .join(" ")
}
```

- [ ] **Step 7: Upgrade MiniMax report parsing**

Extend `ModelReportJson`:

```rust
main_thread: Option<String>,
timeline_phases: Option<Vec<ReportTimelinePhase>>,
project_narratives: Option<Vec<ProjectNarrative>>,
attention_pattern: Option<String>,
uncertainty: Option<String>,
identity_tags: Option<Vec<String>>,
routine_tags: Option<Vec<String>>,
raw_summary_json: Option<serde_json::Value>,
```

In `report_from_window_summary_response_text`, build local structure first, then override each field with parsed values when present:

```rust
let raw_summary_json = crate::llm_json::parse_json_object(content)
    .unwrap_or_else(|| serde_json::json!({ "content": content.trim() }));
```

Set `summary_text` to parsed `summaryText`, otherwise `summary_text_from_report_parts`.

- [ ] **Step 8: Upgrade MiniMax report prompt**

Replace the report system message with:

```rust
"You write structured Chinese personal desktop-work reports from 5-minute visual window summaries. Return one valid JSON object only. Do not wrap it in markdown fences. Do not include commentary. Restore project-level narratives instead of listing each 5-minute window."
```

Replace `window_summary_report_prompt` body with a schema request that includes the report contract from this plan and explicitly says:

```text
Compress 5-minute windows into at most 5 project phases. Do not produce a流水账. Use uncertainty when evidence is weak. Use only the identityTags and routineTags present in evidence unless unknown is needed.
```

- [ ] **Step 9: Run report tests**

Run:

```powershell
cargo test -p tsr-collector --test insight_tests --test storage_tests
```

Expected: PASS after updating existing sample `InsightReport` constructors.

- [ ] **Step 10: Commit**

```powershell
git add collector/src/models.rs collector/src/insights.rs collector/src/storage.rs collector/tests/insight_tests.rs collector/tests/storage_tests.rs
git commit -m "feat: structure five hour project reports"
```

---

### Task 5: Frontend Report Presentation Contract

**Files:**
- Modify: `src/types.ts`
- Modify: `src/lib/insights.ts`
- Modify: `src/lib/insightPresentation.ts`
- Modify: `src/InsightFeedback.tsx`
- Modify: `src/DailyBriefPanel.tsx`
- Modify: `src/styles.css`
- Test: `src/lib/insightPresentation.test.ts`
- Test: `src/App.test.tsx`

- [ ] **Step 1: Write failing presentation tests**

Append to `src/lib/insightPresentation.test.ts`:

```ts
it("uses structured report fields before raw summary text", () => {
  const note = buildReportNote({
    ...report(),
    summaryText: '```json\n{"summaryText":"raw wrapper"}\n```',
    mainThread: "围绕Time State Recorder推进结构化报告。",
    timelinePhases: [
      {
        label: "结构化契约",
        start: "09:00",
        end: "10:00",
        narrative: "后端解析和前端呈现并行检查。",
        projects: ["Time State Recorder"],
        identityTags: ["software_builder"],
        routineTags: ["coding_build"],
        evidenceCount: 4,
      },
    ],
    projectNarratives: [
      {
        project: "Time State Recorder",
        narrative: "把5分钟摘要压缩成可读项目报告。",
        identityTags: ["software_builder"],
        routineTags: ["coding_build"],
        evidenceCount: 4,
      },
    ],
    attentionPattern: "低切换。",
    uncertainty: "仅基于屏幕证据。",
    identityTags: ["software_builder"],
    routineTags: ["coding_build"],
    rawSummaryJson: { summaryText: "raw wrapper" },
  });

  expect(note.mainThread).toBe("围绕Time State Recorder推进结构化报告。");
  expect(note.phases[0].label).toBe("结构化契约");
  expect(note.projects).toEqual(["Time State Recorder"]);
  expect(note.fullText).not.toContain("```json");
});
```

- [ ] **Step 2: Run failing frontend presentation test**

Run:

```powershell
npm test -- src/lib/insightPresentation.test.ts
```

Expected: FAIL because `InsightReport` lacks structured fields and `buildReportNote` does not read them.

- [ ] **Step 3: Expand `InsightReport` type**

In `src/types.ts`, add:

```ts
export type ReportTimelinePhase = {
  label: string;
  start: string;
  end: string;
  narrative: string;
  projects: string[];
  identityTags: string[];
  routineTags: string[];
  evidenceCount: number;
};

export type ProjectNarrative = {
  project: string;
  narrative: string;
  identityTags: string[];
  routineTags: string[];
  evidenceCount: number;
};
```

Extend `InsightReport`:

```ts
mainThread: string;
timelinePhases: ReportTimelinePhase[];
projectNarratives: ProjectNarrative[];
attentionPattern: string;
uncertainty: string;
identityTags: string[];
routineTags: string[];
rawSummaryJson: unknown;
```

- [ ] **Step 4: Parse structured report fields**

In `src/lib/insights.ts`, update `toInsightReport`:

```ts
mainThread: readOptionalString(value, "mainThread") ?? "",
timelinePhases: readOptionalReportPhases(value, "timelinePhases"),
projectNarratives: readOptionalProjectNarratives(value, "projectNarratives"),
attentionPattern: readOptionalString(value, "attentionPattern") ?? "",
uncertainty: readOptionalString(value, "uncertainty") ?? "",
identityTags: readOptionalStringArray(value, "identityTags", ["unknown"]),
routineTags: readOptionalStringArray(value, "routineTags", ["unknown"]),
rawSummaryJson: value.rawSummaryJson ?? null,
```

Add strict mappers for `ReportTimelinePhase` and `ProjectNarrative`, defaulting missing arrays to `[]` for older databases.

- [ ] **Step 5: Update report presentation model**

In `src/lib/insightPresentation.ts`, extend `ReportReviewNote`:

```ts
projectNarratives: ProjectNarrative[];
attentionPattern: string;
uncertainty: string;
identityTags: string[];
routineTags: string[];
```

Update `buildReportNote` to prefer structured fields:

```ts
const structuredMain = report.mainThread?.trim() ?? "";
const structuredPhases = report.timelinePhases ?? [];
const structuredProjects = report.projectNarratives ?? [];
const fullText = firstText(
  structuredMain,
  parsedSummaryText,
  stripJsonSummarySyntax(report.summaryText),
  report.summaryText,
);

return {
  mainThread: firstText(structuredMain, sectioned.mainThread, FALLBACK_REPORT_THREAD),
  phases: structuredPhases.length > 0
    ? structuredPhases.map((phase) => ({
        label: phase.label,
        detail: `${phase.start}-${phase.end}: ${phase.narrative}`,
      }))
    : sectioned.phases,
  projects: structuredProjects.length > 0
    ? structuredProjects.map((item) => item.project)
    : report.projectHints,
  projectNarratives: structuredProjects,
  attentionPattern: report.attentionPattern ?? "",
  uncertainty: report.uncertainty ?? "",
  identityTags: report.identityTags ?? [],
  routineTags: report.routineTags ?? [],
  fullText: normalizeReadableText(fullText),
  rawAvailable: parsed.parsed || parsed.looksRaw || report.rawSummaryJson !== null,
  rawText: parsed.rawText,
};
```

Add:

```ts
function normalizeReadableText(value: string): string {
  return value.replace(/\\n/g, "\n").replace(/\s+\n/g, "\n").replace(/\n{3,}/g, "\n\n").trim();
}
```

- [ ] **Step 6: Render project narratives and tags in Review Notes**

In `src/InsightFeedback.tsx`, inside `ReportContent`, after `ReportPhaseList` render:

```tsx
<ProjectNarrativeList narratives={note.projectNarratives} />
<TagRibbon values={[...note.identityTags, ...note.routineTags]} />
{note.attentionPattern ? <p className="reviewMicrocopy">{note.attentionPattern}</p> : null}
{note.uncertainty ? <p className="reviewMicrocopy">{note.uncertainty}</p> : null}
```

Add components:

```tsx
function ProjectNarrativeList({ narratives }: { narratives: ProjectNarrative[] }) {
  if (narratives.length === 0) return null;
  return (
    <div className="projectNarratives" aria-label="Project narratives">
      {narratives.slice(0, 4).map((item) => (
        <article key={item.project}>
          <strong>{item.project}</strong>
          <p>{item.narrative}</p>
          <small>{item.evidenceCount} windows</small>
        </article>
      ))}
    </div>
  );
}

function TagRibbon({ values }: { values: string[] }) {
  const unique = [...new Set(values.filter((value) => value && value !== "unknown"))];
  if (unique.length === 0) return null;
  return (
    <div className="tagRibbon" aria-label="Identity and routine tags">
      {unique.slice(0, 6).map((value) => (
        <span key={value}>{tagLabel(value)}</span>
      ))}
    </div>
  );
}
```

- [ ] **Step 7: Reuse report presentation in Daily Brief**

In `src/DailyBriefPanel.tsx`, import `buildReportNote` and use it in `ReportRow`:

```tsx
const note = buildReportNote(report);
...
{canShowText ? (
  <>
    <p>{note.mainThread}</p>
    {note.projects.length > 0 ? (
      <div className="tagRibbon">
        {note.projects.slice(0, 3).map((project) => <span key={project}>{project}</span>)}
      </div>
    ) : null}
  </>
) : (
  <p className="redactedText">Report text hidden in redacted mode.</p>
)}
```

- [ ] **Step 8: Add CSS for structured report display**

Append near Review Notes CSS in `src/styles.css`:

```css
.projectNarratives {
  display: grid;
  gap: 8px;
}

.projectNarratives article {
  display: grid;
  gap: 4px;
  border: 1px solid #e3e9e2;
  border-radius: 7px;
  background: #fbfcfa;
  padding: 10px;
}

.projectNarratives strong {
  color: var(--pine);
  font-family: var(--review-serif);
  font-size: 0.94rem;
}

.projectNarratives p,
.reviewMicrocopy {
  margin: 0;
  color: var(--muted-ink) !important;
  font-family:
    Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
    sans-serif !important;
  font-size: 0.84rem !important;
  line-height: 1.45 !important;
}

.projectNarratives small {
  color: var(--muted-ink);
  font-size: 0.72rem;
  font-weight: 750;
}

.tagRibbon {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  min-width: 0;
}

.tagRibbon span {
  max-width: min(100%, 220px);
  overflow: hidden;
  border-radius: 999px;
  background: #eef4f2;
  color: #315a52;
  font-size: 0.72rem;
  font-weight: 750;
  padding: 3px 8px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

- [ ] **Step 9: Update app tests**

In `src/App.test.tsx`, update mock report objects to include the new fields. Add assertions to the Review Notes test:

```ts
expect(await screen.findByText(/围绕Time State Recorder推进/)).toBeInTheDocument();
expect(await screen.findByText(/software builder/i)).toBeInTheDocument();
expect(screen.queryByText(/\\n/)).not.toBeInTheDocument();
```

- [ ] **Step 10: Run frontend tests**

Run:

```powershell
npm test -- src/lib/insightPresentation.test.ts src/lib/insights.test.ts src/App.test.tsx
```

Expected: PASS.

- [ ] **Step 11: Commit**

```powershell
git add src/types.ts src/lib/insights.ts src/lib/insightPresentation.ts src/InsightFeedback.tsx src/DailyBriefPanel.tsx src/styles.css src/lib/insightPresentation.test.ts src/lib/insights.test.ts src/App.test.tsx
git commit -m "feat: present structured visual insight reports"
```

---

### Task 6: End-To-End Verification And Product Check

**Files:**
- No planned source edits unless verification reveals a defect.

- [ ] **Step 1: Run backend tests**

Run:

```powershell
cargo test -p tsr-collector
```

Expected: PASS.

- [ ] **Step 2: Run frontend tests**

Run:

```powershell
npm test
```

Expected: PASS.

- [ ] **Step 3: Build frontend**

Run:

```powershell
npm run build
```

Expected: PASS and `dist/` updated.

- [ ] **Step 4: Check current Minimax request JSON shape with tests**

Run:

```powershell
cargo test -p tsr-collector --test visual_analysis_tests minimax_window_analysis_request_sends_three_images_and_previous_summary
cargo test -p tsr-collector --test insight_tests minimax_insight_report_request_uses_window_summaries
```

Expected: PASS, and assertions cover system prompt, schema text, three image blocks, label keys, and no unsupported `response_format` for `MiniMax-M3`.

- [ ] **Step 5: Optional local UI smoke test**

Run the app if the user wants visual QA:

```powershell
npm run dev
```

Expected: Vite serves on `http://127.0.0.1:5178/` if the current strict-port setup is active, otherwise the terminal prints the usable port. In raw mode, Review Notes and Daily Brief show structured project sections and tags; raw JSON is only under details.

- [ ] **Step 6: Final commit**

If any verification-only fixes were required:

```powershell
git add collector src package.json package-lock.json docs
git commit -m "test: verify structured visual insight contract"
```

If no verification-only fixes were required, do not create an empty commit.

---

## Self-Review

### Spec Coverage

- Fenced and unfenced MiniMax JSON: covered by Task 1 parser and parser tests.
- Request format and system prompt: covered by Task 1 conditional `response_format` helper and Task 2/4 prompt rewrites.
- Image labels around identities and daily routines: covered by Task 2 model/prompt/parser and Task 3 persistence/API/frontend types.
- 5-minute summaries too fine for 5-hour reports: covered by Task 4 phase coalescing and structured report output.
- Human-readable 5-hour report: covered by Task 4 report contract and Task 5 presentation model.
- Frontend product presentation and newline/raw JSON leakage: covered by Task 5 UI and tests.

### Placeholder Scan

This plan contains no placeholder implementation steps. Each code-changing task includes concrete files, snippets, commands, and expected results.

### Type Consistency

Backend snake_case fields serialize to frontend camelCase through existing `serde(rename_all = "camelCase")`. The planned Rust fields `identity_tags`, `routine_tags`, `main_thread`, `timeline_phases`, `project_narratives`, `attention_pattern`, `raw_summary_json` map to TypeScript fields `identityTags`, `routineTags`, `mainThread`, `timelinePhases`, `projectNarratives`, `attentionPattern`, `rawSummaryJson`.
