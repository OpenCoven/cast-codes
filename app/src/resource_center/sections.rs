use warp_core::{context_flag::ContextFlag, features::FeatureFlag};
use warpui::ViewContext;

use super::{
    ContentItem, ContentSectionData, FeatureItem, FeatureSection, FeatureSectionData,
    ResourceCenterMainView, Section, Tip, TipAction, TipHint,
};

pub fn sections(ctx: &mut ViewContext<ResourceCenterMainView>) -> Vec<Section> {
    let mut sections = vec![Section::Changelog()];

    if FeatureFlag::AvatarInTabBar.is_enabled() {
        return sections;
    }

    let get_started = FeatureSectionData {
        section_name: FeatureSection::GettingStarted,
        items: vec![
            FeatureItem::new(
                "Create your first block",
                "Run a command to see your command and output grouped.",
                Tip::Hint(TipHint::CreateBlock),
                ctx,
            ),
            FeatureItem::new(
                "Navigate blocks",
                "Click to select a block and navigate with arrow keys.",
                Tip::Hint(TipHint::BlockSelect),
                ctx,
            ),
            FeatureItem::new(
                "Act on a block",
                "Right click on a block to copy, rerun, or open more actions.",
                Tip::Hint(TipHint::BlockAction),
                ctx,
            ),
            FeatureItem::new(
                "Open command palette",
                "Access all of CastCodes via the keyboard.",
                Tip::Action(TipAction::CommandPalette),
                ctx,
            ),
            FeatureItem::new(
                "Set your theme",
                "Make CastCodes your own by choosing a theme.",
                Tip::Action(TipAction::ThemePicker),
                ctx,
            ),
        ],
    };
    sections.push(Section::Feature(get_started));

    let maximize_warp = FeatureSectionData {
        section_name: FeatureSection::MaximizeWarp,
        items: maximize_warp_items(ctx),
    };
    sections.push(Section::Feature(maximize_warp));

    let advanced_setup = ContentSectionData {
        section_name: FeatureSection::AdvancedSetup,
        items: vec![
            ContentItem {
                title: "Local-first CastCodes",
                description: "Review what this OSS build includes and which hosted services stay unavailable.",
                url: "https://github.com/OpenCoven/cast-codes#current-scope",
                button_label: "Open README",
            },
            ContentItem {
                title: "Coven-powered workspace",
                description: "See how projects, agent lanes, verification, and handoff records fit together.",
                url: "https://github.com/OpenCoven/cast-codes/blob/main/docs/COVEN-POWERED-CASTCODES.md",
                button_label: "Open direction doc",
            },
            ContentItem {
                title: "Build from source",
                description: "Use the repo build commands and package checks for this fork.",
                url: "https://github.com/OpenCoven/cast-codes#build",
                button_label: "Open README",
            },
        ],
    };
    sections.push(Section::Content(advanced_setup));

    sections
}

fn maximize_warp_items(ctx: &mut ViewContext<ResourceCenterMainView>) -> Vec<FeatureItem> {
    let mut maximize_warp_items = vec![];

    maximize_warp_items.push(FeatureItem::new(
        "Command search",
        "Find and run previously executed commands, workflows, and more.",
        Tip::Action(TipAction::CommandSearch),
        ctx,
    ));

    maximize_warp_items.push(FeatureItem::new(
        "AI command search",
        "Generate shell commands with natural language.",
        Tip::Action(TipAction::AiCommandSearch),
        ctx,
    ));

    if ContextFlag::CreateNewSession.is_enabled() {
        maximize_warp_items.push(FeatureItem::new(
            "Split panes",
            "Split tabs into multiple panes to make your ideal layout.",
            Tip::Action(TipAction::SplitPane),
            ctx,
        ));
    }

    if ContextFlag::LaunchConfigurations.is_enabled() {
        maximize_warp_items.push(FeatureItem::new(
            "Launch configuration",
            "Save your current configuration of windows, tabs, and panes.",
            Tip::Action(TipAction::SaveNewLaunchConfig),
            ctx,
        ));
    }

    maximize_warp_items
}

#[cfg(test)]
mod tests {
    const SECTIONS_SOURCE: &str = include_str!("sections.rs");

    #[test]
    fn resource_center_copy_stays_castcodes_owned_and_local_first() {
        let upstream_docs_host = ["docs", "warp", "dev"].join(".");
        let upstream_blog_host = ["www", "warp", "dev"].join(".");
        let stale_share_hint = ["copy/paste", "share", "more"].join(", ");

        assert!(!SECTIONS_SOURCE.contains(&stale_share_hint));
        assert!(!SECTIONS_SOURCE.contains(&upstream_docs_host));
        assert!(!SECTIONS_SOURCE.contains(&upstream_blog_host));
        assert!(SECTIONS_SOURCE.contains("docs/COVEN-POWERED-CASTCODES.md"));
    }
}
