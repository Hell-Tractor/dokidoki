//! 解析 LLM 动作头；MVP 仅支持 `[REPLY]` 与 `|||` 分气泡。

pub fn parse_reply(raw: &str) -> Vec<String> {
    let mut text = raw.trim();
    if let Some(rest) = text.strip_prefix("[REPLY]") {
        text = rest.trim();
    }
    if text.is_empty() {
        return Vec::new();
    }
    text.split("|||")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_reply() {
        assert_eq!(
            parse_reply("[REPLY] 你好"),
            vec!["你好".to_owned()]
        );
    }

    #[test]
    fn parses_multiple_bubbles() {
        assert_eq!(
            parse_reply("[REPLY] 第一句|||第二句"),
            vec!["第一句".to_owned(), "第二句".to_owned()]
        );
    }

    #[test]
    fn empty_reply_returns_empty_vec() {
        assert!(parse_reply("[REPLY]").is_empty());
    }
}
