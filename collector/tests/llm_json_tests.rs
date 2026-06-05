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
    assert_eq!(parse_json_object(text).unwrap()["summaryText"], "完成分析");
}

#[test]
fn extracts_unfenced_json_object() {
    let text = "{\"summaryText\":\"完成分析\",\"confidence\":0.8}";

    assert_eq!(
        parse_json_object(text).unwrap()["confidence"]
            .as_f64()
            .unwrap(),
        0.8
    );
}

#[test]
fn extracts_json_object_from_wrapper_text() {
    let text = "Here is the JSON:\n{\"summaryText\":\"完成分析\"}\nNo more.";

    assert_eq!(parse_json_object(text).unwrap()["summaryText"], "完成分析");
}

#[test]
fn decodes_escaped_newline_json_string() {
    let text = "\"{\\n  \\\"summaryText\\\": \\\"完成分析\\\"\\n}\"";

    assert_eq!(parse_json_object(text).unwrap()["summaryText"], "完成分析");
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
