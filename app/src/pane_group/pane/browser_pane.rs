use url::Url;
use warpui::{AppContext, ModelHandle, View, ViewContext, ViewHandle};

use crate::app_state::LeafContents;

use super::{
    browser::{BrowserView, BrowserViewEvent},
    view::PaneView,
    DetachType, PaneConfiguration, PaneContent, PaneGroup, PaneId, ShareableLink,
    ShareableLinkError,
};

pub type BrowserPaneView = PaneView<BrowserView>;

pub struct BrowserPane {
    view: ViewHandle<BrowserPaneView>,
    pane_configuration: ModelHandle<PaneConfiguration>,
}

impl BrowserPane {
    pub fn from_view(browser_view: ViewHandle<BrowserView>, ctx: &mut AppContext) -> Self {
        let pane_configuration = browser_view.as_ref(ctx).pane_configuration();

        let view = ctx.add_typed_action_view(browser_view.window_id(ctx), |ctx| {
            let pane_id = PaneId::from_browser_pane_ctx(ctx);
            PaneView::new(pane_id, browser_view, (), pane_configuration.clone(), ctx)
        });

        Self {
            view,
            pane_configuration,
        }
    }

    pub fn new<V: View>(url: Option<String>, ctx: &mut ViewContext<V>) -> Self {
        let view = ctx.add_typed_action_view(move |ctx| BrowserView::new(url, ctx));
        Self::from_view(view, ctx)
    }

    pub fn browser_view(&self, ctx: &AppContext) -> ViewHandle<BrowserView> {
        self.view.as_ref(ctx).child(ctx)
    }
}

impl PaneContent for BrowserPane {
    fn id(&self) -> PaneId {
        PaneId::from_browser_pane_view(&self.view)
    }

    fn attach(
        &self,
        _group: &PaneGroup,
        focus_handle: crate::pane_group::focus_state::PaneFocusHandle,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        self.view
            .update(ctx, |view, ctx| view.set_focus_handle(focus_handle, ctx));

        let browser_view = self.browser_view(ctx);
        let pane_id = self.id();

        ctx.subscribe_to_view(&browser_view, move |pane_group, _, event, ctx| {
            let BrowserViewEvent::Pane(pane_event) = event;
            pane_group.handle_pane_event(pane_id, pane_event, ctx);
        });
        ctx.subscribe_to_view(&self.view, move |group, _, event, ctx| {
            group.handle_pane_view_event(pane_id, event, ctx);
        });
    }

    fn detach(
        &self,
        _group: &PaneGroup,
        _detach_type: DetachType,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        let browser_view = self.browser_view(ctx);
        ctx.unsubscribe_to_view(&browser_view);
        ctx.unsubscribe_to_view(&self.view);
    }

    fn snapshot(&self, _app: &AppContext) -> LeafContents {
        // Browser panes are transient until app-state grows a dedicated browser leaf.
        // Reuse the existing non-persisted leaf so session restore skips this pane.
        LeafContents::NetworkLog
    }

    fn has_application_focus(&self, ctx: &mut ViewContext<PaneGroup>) -> bool {
        self.view.is_self_or_child_focused(ctx)
    }

    fn focus(&self, ctx: &mut ViewContext<PaneGroup>) {
        self.browser_view(ctx)
            .update(ctx, |view, ctx| view.focus(ctx));
    }

    fn shareable_link(
        &self,
        ctx: &mut ViewContext<PaneGroup>,
    ) -> Result<ShareableLink, ShareableLinkError> {
        let url = self.browser_view(ctx).as_ref(ctx).current_url().to_string();
        Url::parse(&url)
            .map(|url| ShareableLink::Pane { url })
            .map_err(|_| ShareableLinkError::Expected)
    }

    fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    fn is_pane_being_dragged(&self, ctx: &AppContext) -> bool {
        self.view.as_ref(ctx).is_being_dragged()
    }
}
