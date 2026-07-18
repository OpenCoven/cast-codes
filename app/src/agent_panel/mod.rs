//! The unified CastCodes agent panel — one list + transcript + composer over
//! both CLI-agent and Coven-daemon conversations. Feature-flagged behind
//! [`warp_core::features::FeatureFlag::UnifiedAgentPanel`]; the old `cli_chat`
//! and `ai_assistant` panels are retired in favor of this in a later sub-phase
//! (2d).
//!
//! 2b builds the view shell (merged list + transcript + composer shell). It
//! reuses the `cli_chat` model layer and the shared `agent_transcript`
//! renderer; per-backend composer routing + the live daemon source arrive in
//! 2c. The composer submits via the shared `WorkspaceAction::SubmitChatPrompt`
//! (live CLI only in 2b).

#[cfg(feature = "cast-agent")]
pub mod daemon_turn;
pub mod feature_flag;
#[allow(dead_code)]
pub mod strings;
pub mod view;

#[cfg(test)]
mod view_tests;

use warpui::elements::{CrossAxisAlignment, Element, Expanded, Flex, MainAxisSize, ParentElement};
use warpui::{AppContext, Entity, ModelHandle, SingletonEntity, View, ViewContext, ViewHandle};

use crate::cli_chat::model::{ChatModel, ChatModelEvent};
use crate::view_components::{SubmittableTextInput, SubmittableTextInputEvent};
use crate::workspace::WorkspaceAction;

/// The unified agent panel view. Holds a strong handle to the singleton
/// [`ChatModel`] and a [`SubmittableTextInput`] composer, and re-renders on any
/// [`ChatModelEvent`].
pub struct AgentPanelView {
    pub(crate) chat_model: ModelHandle<ChatModel>,
    pub(crate) composer_input: ViewHandle<SubmittableTextInput>,
    /// Shared buffer for an in-flight daemon turn's streamed events, drained
    /// into the model by a 100ms UI poll. Unix-only (the daemon transport is a
    /// unix socket).
    #[cfg(all(feature = "cast-agent", unix))]
    daemon_stream: std::sync::Arc<std::sync::Mutex<DaemonStreamState>>,
}

/// Buffered state for one in-flight daemon turn. Written by the spawned stream
/// task, drained on the UI thread by [`AgentPanelView::poll_daemon_stream`].
#[cfg(all(feature = "cast-agent", unix))]
#[derive(Default)]
struct DaemonStreamState {
    /// Panel conversation the streamed entries belong to.
    conversation_id: String,
    /// Events streamed since the last UI drain.
    pending: Vec<::ai::cast_agent::CovenAgentEvent>,
    /// Whether the turn is still streaming (drives poll rescheduling).
    in_flight: bool,
    /// The spawned stream task, aborted if a new turn starts before it ends.
    active_task: Option<tokio::task::JoinHandle<()>>,
}

impl AgentPanelView {
    /// Build an `AgentPanelView` bound to the app's singleton `ChatModel`.
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let chat_model = ChatModel::handle(ctx);
        ctx.subscribe_to_model(
            &chat_model,
            |_view, _model, _event: &ChatModelEvent, ctx| {
                ctx.notify();
            },
        );

        let composer_input = Self::create_composer(ctx);

        Self {
            chat_model,
            composer_input,
            #[cfg(all(feature = "cast-agent", unix))]
            daemon_stream: std::sync::Arc::new(std::sync::Mutex::new(DaemonStreamState::default())),
        }
    }

    /// Construct bound to an explicit `ChatModel` handle (tests / non-singleton
    /// callers).
    #[allow(dead_code)]
    pub fn with_model(chat_model: ModelHandle<ChatModel>, ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(
            &chat_model,
            |_view, _model, _event: &ChatModelEvent, ctx| {
                ctx.notify();
            },
        );

        let composer_input = Self::create_composer(ctx);

        Self {
            chat_model,
            composer_input,
            #[cfg(all(feature = "cast-agent", unix))]
            daemon_stream: std::sync::Arc::new(std::sync::Mutex::new(DaemonStreamState::default())),
        }
    }

    fn create_composer(ctx: &mut ViewContext<Self>) -> ViewHandle<SubmittableTextInput> {
        let input = ctx.add_typed_action_view(|ctx| {
            let mut input = SubmittableTextInput::new(ctx);
            input.set_placeholder_text(strings::COMPOSER_PLACEHOLDER_ACTIVE, ctx);
            input.set_outer_margins(0., 0., ctx);
            input
        });

        ctx.subscribe_to_view(&input, |_view, _, event, ctx| match event {
            SubmittableTextInputEvent::Submit(text) => {
                ctx.dispatch_typed_action(&WorkspaceAction::SubmitAgentPrompt {
                    text: text.clone(),
                });
            }
            SubmittableTextInputEvent::Escape => {}
        });

        input
    }

    /// Launch a daemon turn for `conversation_id` and stream its events into the
    /// transcript. Mirrors `ai_assistant::panel`'s spawn-on-runtime + shared
    /// buffer + 100ms UI-poll pattern. Unix-only (the daemon transport is a unix
    /// socket); a no-op elsewhere.
    #[cfg(all(feature = "cast-agent", unix))]
    pub(crate) fn start_daemon_turn(
        &self,
        conversation_id: String,
        text: String,
        ctx: &mut ViewContext<Self>,
    ) {
        use futures::StreamExt;

        let Some(runtime) = ::ai::cast_agent::global() else {
            log::warn!("agent_panel: cast_agent runtime unavailable; cannot send daemon turn");
            return;
        };
        let agent = runtime.agent().clone();
        let msg = daemon_turn::build_daemon_message(&conversation_id, &text);
        let buffer = self.daemon_stream.clone();

        // Reset shared state for the new turn, aborting any still-running task.
        {
            let mut state = buffer.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(previous) = state.active_task.take() {
                previous.abort();
            }
            state.pending.clear();
            state.in_flight = true;
            state.conversation_id = conversation_id;
        }

        let task_buffer = buffer.clone();
        let join = runtime.handle().spawn(async move {
            match agent.stream_agent_events(msg).await {
                Ok(mut ev_stream) => {
                    while let Some(event) = ev_stream.next().await {
                        let terminal = matches!(
                            event,
                            ::ai::cast_agent::CovenAgentEvent::Done
                                | ::ai::cast_agent::CovenAgentEvent::Error { .. }
                        );
                        {
                            let mut state = task_buffer.lock().unwrap_or_else(|p| p.into_inner());
                            state.pending.push(event);
                        }
                        if terminal {
                            break;
                        }
                    }
                }
                Err(err) => {
                    log::info!("agent_panel: stream_agent_events failed: {err}");
                }
            }
            let mut state = task_buffer.lock().unwrap_or_else(|p| p.into_inner());
            state.in_flight = false;
        });
        {
            let mut state = buffer.lock().unwrap_or_else(|p| p.into_inner());
            state.active_task = Some(join);
        }

        self.poll_daemon_stream(ctx);
    }

    /// Drain streamed daemon events into the bound conversation every 100ms
    /// while a turn is in flight. Mirrors `ai_assistant::panel::poll_coven_stream`.
    #[cfg(all(feature = "cast-agent", unix))]
    fn poll_daemon_stream(&self, ctx: &mut ViewContext<Self>) {
        use std::time::Duration;
        use warpui::r#async::Timer;

        ctx.spawn(
            async move { Timer::after(Duration::from_millis(100)).await },
            |view, _, ctx| {
                let (drained, conversation_id, in_flight) = {
                    let mut state = view.daemon_stream.lock().unwrap_or_else(|p| p.into_inner());
                    let drained: Vec<_> = state.pending.drain(..).collect();
                    (drained, state.conversation_id.clone(), state.in_flight)
                };
                if !drained.is_empty() {
                    view.chat_model.update(ctx, |model, mctx| {
                        for event in drained {
                            if let Some(entry) =
                                daemon_turn::drain_event_to_entry(event, 0, chrono::Utc::now())
                            {
                                model.append_entry(&conversation_id, entry, mctx);
                            }
                        }
                    });
                }
                ctx.notify();
                if in_flight {
                    view.poll_daemon_stream(ctx);
                }
            },
        );
    }
}

impl Entity for AgentPanelView {
    type Event = ();
}

impl View for AgentPanelView {
    fn ui_name() -> &'static str {
        "AgentPanelView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let header = view::header::render(self, app);
        let list = view::conversation_list::render_list(self, app);
        let transcript = view::transcript::render_panel(self, app);
        let composer = view::composer::render_composer(self, app);

        // Right column: transcript (flex-expanded) + composer pinned at bottom.
        let right_column = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(Expanded::new(1.0, transcript).finish())
            .with_child(composer)
            .finish();

        // Main body: conversation list on the left, right column fills the rest.
        let body = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(list)
            .with_child(Expanded::new(1.0, right_column).finish())
            .finish();

        // Overall: header bar at top, body fills remaining space.
        Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(header)
            .with_child(Expanded::new(1.0, body).finish())
            .finish()
    }
}
