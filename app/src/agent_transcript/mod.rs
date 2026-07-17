//! Backend-agnostic agent transcript: the render model (`ChatEntry`) and the
//! rich views that display it. Fed by adapters that live on the backend side
//! (`cli_chat` for OSC-777 CLI agents; `ai_assistant`/`cast_agent` for the
//! Coven daemon). This module must not depend on any backend or transport —
//! it is a leaf render module (enforced by `script/check_cli_chat_boundary`).

pub mod entry;
pub mod view;
