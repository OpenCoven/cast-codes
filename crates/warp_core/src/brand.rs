//! Public CastCodes branding constants.
//!
//! Internal crate/module names still use historical Warp identifiers in this
//! pass. These constants are for user-visible product identity, public app IDs,
//! persisted public paths, URL schemes, and default fork-local service config.

pub const PRODUCT_NAME: &str = "CastCodes";
pub const PRODUCT_SLUG: &str = "cast-codes";
pub const ORG_ID: &str = "castcodes";
pub const PUBLIC_APP_ID: &str = "dev.castcodes.CastCodes";
pub const PUBLIC_URL_SCHEME: &str = "castcodes";

pub const CONFIG_DIR: &str = ".cast-codes";
pub const LEGACY_CONFIG_DIR: &str = ".warp";
pub const LOG_FILE_NAME: &str = "cast-codes.log";

pub const UNAVAILABLE_LOCALHOST_HTTP_URL: &str = "http://127.0.0.1:9";
pub const UNAVAILABLE_LOCALHOST_WS_URL: &str = "ws://127.0.0.1:9/graphql/v2";

pub const fn public_app_id_parts() -> (&'static str, &'static str, &'static str) {
    ("dev", ORG_ID, PRODUCT_NAME)
}
