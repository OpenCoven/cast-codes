use warpui::{App, View};

use crate::agent_panel::view::conversation_list::backend_badge;
use crate::agent_panel::AgentPanelView;
use crate::cli_chat::conversation::{AgentKind, ConversationBackend};
use crate::cli_chat::model::ChatModel;
use crate::test_util::{add_window_with_terminal, terminal::initialize_app_for_terminal_view};

#[test]
fn backend_badge_distinguishes_cli_and_daemon() {
    assert_eq!(
        backend_badge(&ConversationBackend::Cli(AgentKind::Claude)),
        "CLI"
    );
    assert_eq!(
        backend_badge(&ConversationBackend::Daemon {
            harness: "coven-code".into()
        }),
        "Coven"
    );
}

#[test]
fn agent_panel_view_lays_out_without_panicking() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let term = add_window_with_terminal(&mut app, None);

        // Build the panel over an explicit, unwired in-memory model (avoids
        // needing the `ChatModel` singleton registered in the test harness).
        // Constructing exercises the composer + subscription; rendering forces a
        // layout pass. Neither must panic with an empty (unbound) model.
        let chat_model = app.add_model(|_ctx| ChatModel::new_unwired());
        let panel = term.update(&mut app, |_view, ctx| {
            ctx.add_view(|ctx| AgentPanelView::with_model(chat_model.clone(), ctx))
        });

        panel.read(&app, |view, ctx| {
            let _ = view.render(ctx);
        });
    })
}

#[test]
fn composer_active_for_live_daemon_only_when_runtime_up() {
    use crate::agent_panel::view::composer::composer_is_active_for;
    use crate::cli_chat::conversation::ConversationBinding;

    let daemon = ConversationBinding::LiveDaemon {
        session_id: "d1".into(),
    };
    assert!(composer_is_active_for(&daemon, true));
    assert!(!composer_is_active_for(&daemon, false));

    // A live CLI conversation is always sendable; past/none never.
    let past = ConversationBinding::Past {
        session_id: "p1".into(),
    };
    assert!(!composer_is_active_for(&past, true));
    assert!(!composer_is_active_for(&ConversationBinding::None, true));
}

#[test]
fn inactive_placeholder_names_the_daemon_fix() {
    use crate::agent_panel::strings;
    use crate::agent_panel::view::composer::{inactive_placeholder_for, DaemonLink};
    use crate::cli_chat::conversation::ConversationBinding;

    let daemon = ConversationBinding::LiveDaemon {
        session_id: "d1".into(),
    };
    // Daemon conversations get action-oriented hints per handshake outcome.
    let offline = inactive_placeholder_for(&daemon, DaemonLink::Offline);
    assert_eq!(offline, strings::COMPOSER_PLACEHOLDER_DAEMON_OFFLINE);
    // The hint must name the real CLI verb — `coven daemon start`.
    assert!(offline.contains("coven daemon start"));
    assert_eq!(
        inactive_placeholder_for(&daemon, DaemonLink::Incompatible),
        strings::COMPOSER_PLACEHOLDER_DAEMON_INCOMPATIBLE
    );

    // Non-daemon bindings keep the generic copy regardless of daemon state.
    let past = ConversationBinding::Past {
        session_id: "p1".into(),
    };
    assert_eq!(
        inactive_placeholder_for(&past, DaemonLink::Offline),
        strings::COMPOSER_PLACEHOLDER_INACTIVE
    );
    assert_eq!(
        inactive_placeholder_for(&ConversationBinding::None, DaemonLink::Incompatible),
        strings::COMPOSER_PLACEHOLDER_INACTIVE
    );
}
