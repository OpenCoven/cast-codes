//! Code expression of the CastCodes Phase 1 design contract.
//!
//! The contract in `AGENTS.md` defines colors, radii, spacing and motion, but until now only the
//! color half had a code representation (`warp_core::ui::theme` tokens and accessors). Sizing was
//! written as bare literals at every call site, so there was nothing for new code to reference and
//! nothing to review against.
//!
//! These constants close that gap. Colors are deliberately *not* duplicated here — they belong to
//! the theme accessors so they can vary per theme, whereas the scales below are theme independent.
//!
//! Inherited Warp views still use bare literals. They are migrated opportunistically when a view is
//! touched for other reasons rather than in a single sweeping rewrite, which would churn hundreds of
//! unverifiable call sites across the fork. The scale is therefore declared in full even where the
//! fork has not adopted a given step yet: an incomplete vocabulary just pushes the next author back
//! to writing a bare literal.
#![allow(dead_code)]

/// Corner radii. The contract allows three steps; anything larger reads as marketing chrome.
///
/// `Radius::Percentage(50.)` is still the right choice for pills, dots and avatars — those are
/// fully rounded by intent rather than by a fixed pixel step.
pub mod radius {
    /// Compact items: chips, badges, list rows, small icon buttons.
    pub const COMPACT: f32 = 4.;
    /// Controls: buttons, inputs, dropdowns, tabs.
    pub const CONTROL: f32 = 6.;
    /// Larger surfaces: modals, panels, cards. The contract's maximum.
    pub const SURFACE: f32 = 8.;
}

/// Spacing steps for padding, margins and gaps, in logical pixels.
///
/// The scale is a 4px grid with a 2px hairline step for dense affordances.
pub mod space {
    /// Hairline gap between tightly coupled elements, e.g. an icon and its label.
    pub const HAIRLINE: f32 = 2.;
    pub const XS: f32 = 4.;
    pub const SM: f32 = 8.;
    pub const MD: f32 = 12.;
    pub const LG: f32 = 16.;
    pub const XL: f32 = 24.;
}

/// Type scale in logical pixels.
///
/// Prefer `Appearance::ui_font_size()` for primary UI text so the user's font-size preference is
/// respected. Use these only for text that must hold a fixed relationship to a fixed-size element
/// (badge counts, avatar initials, status chips).
pub mod font_size {
    /// Badge counts, avatar initials and status chips.
    pub const MICRO: f32 = 10.;
    /// Secondary and supporting text.
    pub const SMALL: f32 = 12.;
    /// Default body text.
    pub const BODY: f32 = 14.;
    /// Section headings.
    pub const LARGE: f32 = 16.;
    /// Modal and page titles. The contract's maximum for product UI; larger sizes read as
    /// marketing copy rather than an editor-grade workspace.
    pub const TITLE: f32 = 20.;
}

/// Animation durations in milliseconds. The contract calls for 100-150ms ease-in-out and no
/// bouncy easing.
///
/// `warpui` has no shared transition primitive, so these cannot be wired into a generic animation
/// helper yet; they are the reference for hand-rolled animations until one exists.
pub mod motion {
    /// Hover and press feedback.
    pub const FAST_MS: u64 = 100;
    /// Surface transitions: panels opening, rows expanding.
    pub const BASE_MS: u64 = 150;
}
