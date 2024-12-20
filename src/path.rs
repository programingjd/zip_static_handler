pub(crate) fn path(zip_prefix: &str, name: &str) -> String {
    let name = &name[zip_prefix.len()..];
    let start = name.find(|c| c != '.' && c != '/').unwrap_or(0);
    if name.ends_with(".html") {
        if &name[start..] == "index.html" {
            "/".to_string()
        } else {
            format!("/{}/", &name[start..name.len() - 5])
        }
    } else if name.ends_with(".307") || name.ends_with(".308") {
        format!("/{}/", &name[start..name.len() - 4])
    } else {
        format!("/{}", &name[start..])
    }
}

pub(crate) fn filename(name: &str) -> &str {
    let byte_position = name.rfind('/').map(|it| it + 1).unwrap_or(0);
    &name[byte_position..]
}

pub(crate) fn extension(filename: &str) -> &str {
    let byte_position = filename.rfind('.').map(|it| it + 1).unwrap_or(0);
    &filename[byte_position..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_with_prefix() {
        let prefix = "/prefix";
        assert_eq!(path(prefix, "/prefix/index.html"), "/");
        assert_eq!(path(prefix, "/prefix/a/b"), "/a/b");
        assert_eq!(path(prefix, "/prefix/a/b.html"), "/a/b/");
        assert_eq!(path(prefix, "/prefix/a/b/"), "/a/b/");
        assert_eq!(path(prefix, "/prefix/a/b/c.jpg"), "/a/b/c.jpg");
        let prefix = "/prefix/";
        assert_eq!(path(prefix, "/prefix/index.html"), "/");
        assert_eq!(path(prefix, "/prefix/a/b"), "/a/b");
        assert_eq!(path(prefix, "/prefix/a/b.html"), "/a/b/");
        assert_eq!(path(prefix, "/prefix/a/b/"), "/a/b/");
        assert_eq!(path(prefix, "/prefix/a/b/c.jpg"), "/a/b/c.jpg");
    }

    #[test]
    fn path_no_prefix() {
        assert_eq!(path("", "/index.html"), "/");
        assert_eq!(path("", "/a/b"), "/a/b");
        assert_eq!(path("", "/a/b.html"), "/a/b/");
        assert_eq!(path("", "/a/b/"), "/a/b/");
        assert_eq!(path("", "/a/b/c.jpg"), "/a/b/c.jpg");
        assert_eq!(path("", "index.html"), "/");
        assert_eq!(path("", "a/b"), "/a/b");
        assert_eq!(path("", "a/b.html"), "/a/b/");
        assert_eq!(path("", "a/b/"), "/a/b/");
        assert_eq!(path("", "a/b/c.jpg"), "/a/b/c.jpg");
    }
}
