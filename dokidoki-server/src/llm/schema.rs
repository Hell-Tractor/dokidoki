//! 角色回合 JSON Schema（OpenAI-compatible `response_format`）。

use serde_json::{json, Value};

/// `response_format.json_schema.schema` 本体。
pub fn character_turn_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "user_busy": { "type": "boolean" },
            "store_memories": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "content": { "type": "string" },
                        "memory_type": {
                            "type": "string",
                            "enum": ["trivial", "normal", "important", "permanent"]
                        },
                        "memory_key": { "type": ["string", "null"] }
                    },
                    "required": ["content", "memory_type", "memory_key"]
                }
            },
            "forget_memories": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "target": { "type": "string" }
                    },
                    "required": ["target"]
                }
            },
            "action": {
                "oneOf": [
                    {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "type": { "const": "no_reply" }
                        },
                        "required": ["type"]
                    },
                    {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "type": { "const": "reply" },
                            "bubbles": {
                                "type": "array",
                                "items": { "type": "string" },
                                "minItems": 1,
                                "maxItems": 4
                            }
                        },
                        "required": ["type", "bubbles"]
                    },
                    {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "type": { "const": "end_topic" },
                            "bubbles": {
                                "type": "array",
                                "items": { "type": "string" },
                                "minItems": 1,
                                "maxItems": 4
                            }
                        },
                        "required": ["type", "bubbles"]
                    }
                ]
            }
        },
        "required": ["user_busy", "store_memories", "forget_memories", "action"]
    })
}

/// 发给 API 的 `response_format` 对象。
pub fn response_format_payload(mode: &str) -> Option<Value> {
    match mode {
        "json_schema" => Some(json!({
            "type": "json_schema",
            "json_schema": {
                "name": "character_turn",
                "strict": true,
                "schema": character_turn_schema()
            }
        })),
        "json_object" => Some(json!({
            "type": "json_object"
        })),
        "off" | "" => None,
        other => {
            tracing::warn!(mode = %other, "unknown llm.response_format; treating as off");
            None
        }
    }
}
