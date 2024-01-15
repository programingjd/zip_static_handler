use std::fmt::Display;

pub fn zip_download_branch_url(
    user: impl Display,
    repository: impl Display,
    branch: impl Display,
) -> String {
    format!("https://codeload.github.com/{user}/{repository}/zip/refs/heads/{branch}")
}

pub fn zip_download_tag_url(
    user: impl Display,
    repository: impl Display,
    tag: impl Display,
) -> String {
    format!("https://codeload.github.com/{user}/{repository}/zip/refs/tags/{tag}")
}

pub fn zip_download_commit_url(
    user: impl Display,
    repository: impl Display,
    commit: impl Display,
) -> String {
    format!("https://codeload.github.com/{user}/{repository}/zip/{commit}")
}
