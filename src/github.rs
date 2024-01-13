use std::fmt::Display;

pub(crate) fn zip_download_branch_url(
    user: impl Display,
    repository: impl Display,
    branch: impl Display,
) -> String {
    return format!("https://codeload.github.com/{user}/{repository}/zip/refs/heads/{branch}");
}

pub(crate) fn zip_download_tag_url(
    user: impl Display,
    repository: impl Display,
    tag: impl Display,
) -> String {
    return format!("https://codeload.github.com/{user}/{repository}/zip/refs/tags/{tag}");
}

pub(crate) fn zip_download_commit_url(
    user: impl Display,
    repository: impl Display,
    commit: impl Display,
) -> String {
    return format!("https://codeload.github.com/{user}/{repository}/zip/{commit}");
}
