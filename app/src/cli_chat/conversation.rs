// stub — replaced in Task 1.5

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Claude,
    Codex,
    Gemini,
    OpenCode,
}

#[derive(Debug, Clone)]
pub struct ChatConversation;

#[derive(Debug, Clone, Copy)]
pub enum ConversationBinding {
    None,
}
