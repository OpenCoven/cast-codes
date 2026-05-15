//! `ChatModel`: the warpui `Entity` that backs the CastCodes chat panel.
//!
//! Subscribes to [`CLIAgentSessionsModel`] events and converts each raw
//! [`CLIAgentEvent`] into a typed [`ChatEntry`] appended to the appropriate
//! [`ChatConversation`]. See `specs/castcodes-chat-panel/TECH.md` for the
//! full data flow.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use warpui::{Entity, EntityId, ModelContext, SingletonEntity};

use crate::cli_chat::conversation::{AgentKind, ChatConversation, ConversationBinding};
use crate::cli_chat::entry::{ChatEntry, ChatEntryKind};
use crate::terminal::cli_agent_sessions::{
    event::CLIAgentEvent, CLIAgentSessionsModel, CLIAgentSessionsModelEvent,
};

/// Events emitted by [`ChatModel`] for view subscribers.
#[derive(Debug, Clone)]
pub enum ChatModelEvent {
    /// A new entry was appended to the conversation with this `session_id`,
    /// or its metadata (title, status, updated_at) changed.
    ConversationUpdated { session_id: String },
    /// A conversation was added or its ordering may have changed. Views
    /// rendering the conversation list should re-sort.
    ConversationListChanged,
    /// The active panel binding changed (Live/Past/None).
    BindingChanged,
    /// Reserved for future use: signals that a `CLIAgentEvent` was received
    /// with a protocol version this build cannot decode.
    #[allow(dead_code)]
    ProtocolIncompatibilityDetected,
}

/// In-memory model that aggregates `CLIAgentEvent`s into per-session
/// [`ChatConversation`]s for rendering in the CastCodes chat panel.
///
/// Persistence to sqlite is wired in Phase 3 (see PLAN.md).
pub struct ChatModel {
    conversations: HashMap<String, ChatConversation>,
    next_sequence: HashMap<String, u64>,
    binding: ConversationBinding,
}

impl Entity for ChatModel {
    type Event = ChatModelEvent;
}

impl SingletonEntity for ChatModel {}

impl ChatModel {
    /// Construct a `ChatModel` and subscribe to `CLIAgentSessionsModel`.
    ///
    /// The subscription pattern mirrors `AgentNotificationsModel::new` in
    /// `app/src/ai/agent_management/agent_management_model.rs:44`.
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let sessions = CLIAgentSessionsModel::handle(ctx);
        ctx.subscribe_to_model(&sessions, |me, event, ctx| {
            me.handle_sessions_event(event, ctx);
        });
        Self {
            conversations: HashMap::new(),
            next_sequence: HashMap::new(),
            binding: ConversationBinding::None,
        }
    }

    /// Construct a `ChatModel` without subscribing to any session source.
    ///
    /// Used by unit tests that drive [`ChatModel::apply_event`] directly
    /// without standing up a warpui app harness.
    #[cfg(test)]
    pub(crate) fn new_unwired() -> Self {
        Self {
            conversations: HashMap::new(),
            next_sequence: HashMap::new(),
            binding: ConversationBinding::None,
        }
    }

    /// The currently bound conversation, if any.
    pub fn binding(&self) -> &ConversationBinding {
        &self.binding
    }

    /// Look up a conversation by its `session_id`.
    pub fn conversation(&self, session_id: &str) -> Option<&ChatConversation> {
        self.conversations.get(session_id)
    }

    /// All known conversations, most recently updated first.
    pub fn conversations_sorted_by_recency(&self) -> Vec<&ChatConversation> {
        let mut v: Vec<_> = self.conversations.values().collect();
        v.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        v
    }

    /// Bind the panel to a live (active) session.
    pub fn bind_live(
        &mut self,
        session_id: String,
        terminal_view_id: EntityId,
        ctx: &mut ModelContext<Self>,
    ) {
        self.binding = ConversationBinding::Live {
            session_id,
            terminal_view_id,
        };
        ctx.emit(ChatModelEvent::BindingChanged);
    }

    /// Bind the panel to a past (read-only) session.
    pub fn bind_past(&mut self, session_id: String, ctx: &mut ModelContext<Self>) {
        self.binding = ConversationBinding::Past { session_id };
        ctx.emit(ChatModelEvent::BindingChanged);
    }

    /// Clear the panel binding.
    pub fn unbind(&mut self, ctx: &mut ModelContext<Self>) {
        self.binding = ConversationBinding::None;
        ctx.emit(ChatModelEvent::BindingChanged);
    }

    fn handle_sessions_event(
        &mut self,
        event: &CLIAgentSessionsModelEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        match event {
            CLIAgentSessionsModelEvent::EventReceived {
                terminal_view_id,
                event,
                ..
            } => {
                let outcome = self.apply_event(event, *terminal_view_id, Utc::now());
                self.emit_outcome(outcome, ctx);
            }
            CLIAgentSessionsModelEvent::Started { .. }
            | CLIAgentSessionsModelEvent::StatusChanged { .. }
            | CLIAgentSessionsModelEvent::InputSessionChanged { .. }
            | CLIAgentSessionsModelEvent::Ended { .. }
            | CLIAgentSessionsModelEvent::SessionUpdated { .. } => {
                // Status and lifecycle events are derived from the raw
                // EventReceived stream — nothing to do here.
            }
        }
    }

    fn emit_outcome(&mut self, outcome: ApplyOutcome, ctx: &mut ModelContext<Self>) {
        if outcome.binding_changed {
            ctx.emit(ChatModelEvent::BindingChanged);
        }
        if let Some(session_id) = outcome.updated_session_id {
            ctx.emit(ChatModelEvent::ConversationUpdated { session_id });
            ctx.emit(ChatModelEvent::ConversationListChanged);
        }
    }

    /// Apply a raw [`CLIAgentEvent`] to the in-memory transcript state.
    ///
    /// Returns an [`ApplyOutcome`] describing what changed so the caller
    /// can decide whether to emit `ChatModelEvent`s. Split from the
    /// subscription callback so unit tests can drive this directly
    /// without a warpui app harness (see `model_tests.rs`).
    pub(crate) fn apply_event(
        &mut self,
        event: &CLIAgentEvent,
        terminal_view_id: EntityId,
        now: DateTime<Utc>,
    ) -> ApplyOutcome {
        let mut outcome = ApplyOutcome::default();

        let Some(session_id) = event.session_id.clone() else {
            return outcome;
        };
        let Some(agent) = AgentKind::from_cli_agent(&event.agent) else {
            return outcome;
        };

        let is_new = !self.conversations.contains_key(&session_id);
        let conv = self
            .conversations
            .entry(session_id.clone())
            .or_insert_with(|| {
                let mut c = ChatConversation::new(session_id.clone(), agent, now);
                c.cwd = event.cwd.clone();
                c.project = event.project.clone();
                c
            });

        // Refresh context fields that may have arrived later in the
        // session (e.g. `cwd` is sometimes only on `PromptSubmit`).
        if conv.cwd.is_none() {
            conv.cwd = event.cwd.clone();
        }
        if conv.project.is_none() {
            conv.project = event.project.clone();
        }
        conv.updated_at = now;

        let next_seq = self.next_sequence.entry(session_id.clone()).or_insert(0);
        let mut appended = false;
        if let Some(entry) = ChatEntry::from_event(event, *next_seq, now) {
            // Auto-derive title from the first user prompt.
            if conv.title.is_empty() {
                if let ChatEntryKind::UserPrompt { text } = &entry.kind {
                    conv.title = text.chars().take(80).collect();
                }
            }

            // Special case: a Stop event with a non-empty `response`
            // also produces an AssistantResponse entry inserted *before*
            // the Stop entry, so the transcript reads naturally.
            let stop_response = if let ChatEntryKind::Stop {
                response: Some(text),
                ..
            } = &entry.kind
            {
                Some(text.clone())
            } else {
                None
            };

            if let Some(text) = stop_response {
                let assistant_entry = ChatEntry {
                    sequence: *next_seq,
                    created_at: now,
                    kind: ChatEntryKind::AssistantResponse { text },
                };
                conv.entries.push(assistant_entry);
                *next_seq += 1;

                let stop_entry = ChatEntry {
                    sequence: *next_seq,
                    created_at: now,
                    kind: entry.kind,
                };
                conv.entries.push(stop_entry);
                *next_seq += 1;
            } else {
                conv.entries.push(entry);
                *next_seq += 1;
            }
            appended = true;
        }

        if appended || is_new {
            outcome.updated_session_id = Some(session_id.clone());
        }

        // Auto-bind to the first session whose events arrive while
        // nothing is currently bound. Mirrors the Live state-transition
        // described in TECH.md ("None → Live(X) on first event").
        if matches!(self.binding, ConversationBinding::None) {
            self.binding = ConversationBinding::Live {
                session_id,
                terminal_view_id,
            };
            outcome.binding_changed = true;
        }

        outcome
    }
}

/// Result of [`ChatModel::apply_event`].
#[derive(Debug, Default)]
pub(crate) struct ApplyOutcome {
    pub updated_session_id: Option<String>,
    pub binding_changed: bool,
}
