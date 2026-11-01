use super::error::Result;
use super::types::{ChatCompletionChunk, ChatCompletionResponse};

/// True when a buffered HTTP body looks like Server-Sent Events rather than a JSON object.
pub fn is_sse_body(body: &str) -> bool {
    body.lines().any(|line| line.starts_with("data:"))
}

/// Replay a full HTTP body as SSE `data:` payloads.
///
/// Mock transports and gateways that ignore `stream:true` return a normal
/// chat-completion JSON object; that is converted into a single delta event.
pub fn replay_body_as_sse_events(
    body: &str,
    on_event: &mut dyn FnMut(&str) -> Result<()>,
) -> Result<()> {
    if is_sse_body(body) {
        let mut parser = SseParser::new();
        parser.push(body, on_event)?;
        parser.finish(on_event)?;
        return Ok(());
    }

    if let Some(content) = completion_message_content(body) {
        let delta = serde_json::json!({
            "choices": [{ "delta": { "content": content } }]
        })
        .to_string();
        on_event(&delta)?;
        on_event("[DONE]")?;
    }

    Ok(())
}

pub fn completion_message_content(body: &str) -> Option<String> {
    let payload: ChatCompletionResponse = serde_json::from_str(body).ok()?;
    payload
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_ref())
        .map(|text| text.trim())
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

pub fn apply_chunk_payload(content: &mut String, thinking: &mut bool, payload: &str) -> Result<()> {
    let trimmed = payload.trim();
    if trimmed.is_empty() || trimmed == "[DONE]" {
        return Ok(());
    }

    let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(trimmed) else {
        return Ok(());
    };

    let Some(choice) = chunk.choices.first() else {
        return Ok(());
    };

    let has_reasoning = choice
        .delta
        .reasoning_content
        .as_deref()
        .or(choice.delta.reasoning.as_deref())
        .is_some_and(|text| !text.is_empty());
    if has_reasoning {
        *thinking = true;
    }

    if let Some(piece) = choice.delta.content.as_deref() {
        if !piece.is_empty() {
            content.push_str(piece);
            *thinking = false;
        }
    }

    Ok(())
}

pub struct SseParser {
    pending: String,
}

impl SseParser {
    pub fn new() -> Self {
        Self {
            pending: String::new(),
        }
    }

    pub fn push(&mut self, text: &str, on_event: &mut dyn FnMut(&str) -> Result<()>) -> Result<()> {
        self.pending
            .push_str(&text.replace("\r\n", "\n").replace('\r', "\n"));

        while let Some(idx) = self.pending.find("\n\n") {
            let event: String = self.pending.drain(..idx + 2).collect();
            emit_event(&event, on_event)?;
        }

        Ok(())
    }

    pub fn leftover(&self) -> &str {
        &self.pending
    }

    pub fn finish(&mut self, on_event: &mut dyn FnMut(&str) -> Result<()>) -> Result<()> {
        if self.pending.trim().is_empty() {
            self.pending.clear();
            return Ok(());
        }

        let leftover = std::mem::take(&mut self.pending);
        emit_event(&leftover, on_event)
    }
}

fn emit_event(event: &str, on_event: &mut dyn FnMut(&str) -> Result<()>) -> Result<()> {
    let mut data_lines = Vec::new();
    for line in event.lines() {
        if let Some(data) = sse_data_payload(line) {
            data_lines.push(data);
        }
    }

    if data_lines.is_empty() {
        return Ok(());
    }

    on_event(&data_lines.join("\n"))
}

fn sse_data_payload(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("data:")?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(body: &str) -> Vec<String> {
        let mut out = Vec::new();
        replay_body_as_sse_events(body, &mut |payload| {
            out.push(payload.to_string());
            Ok(())
        })
        .expect("replay");
        out
    }

    #[test]
    fn replay_converts_json_completion_to_one_delta() {
        let body = r#"{"choices":[{"message":{"content":"hello"}}]}"#;
        let events = collect(body);
        assert_eq!(events.len(), 2);
        assert!(events[0].contains("hello"));
        assert_eq!(events[1], "[DONE]");
    }

    #[test]
    fn parser_splits_sse_events() {
        let mut parser = SseParser::new();
        let mut events = Vec::new();
        parser
            .push(
                "data: {\"choices\":[{\"delta\":{\"content\":\"he\"}}]}\n\n\
                 data: {\"choices\":[{\"delta\":{\"content\":\"llo\"}}]}\n\n\
                 data: [DONE]\n\n",
                &mut |payload| {
                    events.push(payload.to_string());
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(
            events,
            vec![
                r#"{"choices":[{"delta":{"content":"he"}}]}"#.to_string(),
                r#"{"choices":[{"delta":{"content":"llo"}}]}"#.to_string(),
                "[DONE]".to_string(),
            ]
        );
    }

    #[test]
    fn apply_chunk_ignores_reasoning_and_accumulates_content() {
        let mut content = String::new();
        let mut thinking = false;
        apply_chunk_payload(
            &mut content,
            &mut thinking,
            r#"{"choices":[{"delta":{"reasoning_content":"plan"}}]}"#,
        )
        .unwrap();
        assert!(thinking);
        assert!(content.is_empty());

        apply_chunk_payload(
            &mut content,
            &mut thinking,
            r##"{"choices":[{"delta":{"content":"Hi"}}]}"##,
        )
        .unwrap();
        assert!(!thinking);
        assert_eq!(content, "Hi");
    }
}
