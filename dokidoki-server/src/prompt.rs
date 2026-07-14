mod chat;
mod summary;
mod templates;

pub use chat::{
    build_icebreaker_system_prompt, build_system_prompt, build_system_prompt_with_scenes,
    format_availability_style, format_chat_scenes, format_current_state_section,
    format_icebreaker_user_message, format_memories_block, format_proactive_scene,
    format_proactive_user_message, format_summary_block, ChatSceneFlags, CurrentStatePrompt,
};
pub use summary::build_summary_request;
