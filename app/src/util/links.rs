use crate::channel::ChannelState;

pub const USER_DOCS_URL: &str = "https://github.com/OpenCoven/cast-codes/blob/main/README.md";
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const GITHUB_ISSUES_URL: &str = "https://github.com/OpenCoven/cast-codes/issues";
pub const PRIVACY_POLICY_URL: &str =
    "https://github.com/OpenCoven/cast-codes/blob/main/CASTCODES.md";

pub fn feedback_form_url() -> String {
    let mut url = url::Url::parse("https://github.com/OpenCoven/cast-codes/issues/new/choose")
        .expect("Should not fail to parse");
    if let Some(version) = ChannelState::app_version() {
        url.query_pairs_mut().append_pair("warp-version", version);
    }
    url.query_pairs_mut()
        .append_pair("os-version", &os_info::get().version().to_string());
    url.to_string()
}
