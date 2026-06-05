use serde::de::DeserializeOwned;

pub fn minimax_json_response_format(model: &str) -> Option<serde_json::Value> {
    if model.eq_ignore_ascii_case("MiniMax-Text-01") {
        Some(serde_json::json!({ "type": "json_object" }))
    } else {
        None
    }
}

pub fn parse_json_object(content: &str) -> Option<serde_json::Value> {
    let object_text = extract_json_object_text(content)?;
    let value = serde_json::from_str::<serde_json::Value>(&object_text).ok()?;
    value.is_object().then_some(value)
}

pub fn parse_json_object_as<T: DeserializeOwned>(content: &str) -> Option<T> {
    parse_json_object(content).and_then(|value| serde_json::from_value(value).ok())
}

pub fn extract_json_object_text(content: &str) -> Option<String> {
    extract_json_object_text_inner(content, 0)
}

pub fn remove_code_fence(content: &str) -> String {
    let trimmed = content.trim();
    let Some(after_opening_ticks) = trimmed.strip_prefix("```") else {
        return trimmed.to_string();
    };

    let body = after_opening_ticks
        .find('\n')
        .map(|newline| &after_opening_ticks[newline + 1..])
        .unwrap_or(after_opening_ticks)
        .trim_start_matches('\r')
        .trim();

    body.strip_suffix("```").unwrap_or(body).trim().to_string()
}

fn extract_json_object_text_inner(content: &str, depth: u8) -> Option<String> {
    if depth > 1 {
        return None;
    }

    let trimmed = remove_code_fence(content);
    if let Some(object_text) = extract_balanced_object_text(&trimmed) {
        return Some(object_text);
    }

    serde_json::from_str::<String>(trimmed.trim())
        .ok()
        .and_then(|decoded| extract_json_object_text_inner(&decoded, depth + 1))
}

fn extract_balanced_object_text(content: &str) -> Option<String> {
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in content.char_indices() {
        if start.is_none() {
            if ch == '{' {
                start = Some(index);
                depth = 1;
                in_string = false;
                escaped = false;
            }
            continue;
        }

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
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = index + ch.len_utf8();
                    let candidate = content[start?..end].trim();
                    if serde_json::from_str::<serde_json::Value>(candidate)
                        .ok()
                        .is_some_and(|value| value.is_object())
                    {
                        return Some(candidate.to_string());
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }

    None
}
