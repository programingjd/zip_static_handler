pub(crate) fn path(zip_prefix: &str, name: &str) -> String {
    let name = &name[zip_prefix.len()..];
    let start = name.find(|c| c != '.' && c != '/').unwrap_or(0);
    let end = if name.ends_with("index.html") {
        name.len() - 10
    } else {
        name.len()
    };
    format!("/{}", &name[start..end])
}

pub(crate) fn filename(name: &str) -> &str {
    let byte_position = name.rfind(|c| c == '/').map(|it| it + 1).unwrap_or(0);
    &name[byte_position..]
}

pub(crate) fn extension(filename: &str) -> &str {
    let byte_position = filename.rfind(|c| c == '.').map(|it| it + 1).unwrap_or(0);
    &filename[byte_position..]
}
