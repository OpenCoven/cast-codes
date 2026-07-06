use crate::brand;

#[test]
fn brand_constants_lock_public_castcodes_names() {
    assert_eq!(brand::PRODUCT_NAME, "CastCodes");
    assert_eq!(brand::PRODUCT_SLUG, "cast-codes");
    assert_eq!(brand::ORG_ID, "castcodes");
    assert_eq!(brand::PUBLIC_APP_ID, "dev.castcodes.CastCodes");
    assert_eq!(brand::PUBLIC_URL_SCHEME, "castcodes");
    assert_eq!(
        brand::PUBLIC_USER_DOCS_URL,
        "https://github.com/OpenCoven/cast-codes/blob/main/README.md"
    );
    assert_eq!(brand::CONFIG_DIR, ".cast-codes");
    assert_eq!(brand::LEGACY_CONFIG_DIR, ".warp");
}
