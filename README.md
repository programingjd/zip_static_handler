### zip_static_handler &nbsp;[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![crates.io Version](https://img.shields.io/crates/v/zip_static_handler.svg)](https://crates.io/crates/zip_static_handler) [![Documentation](https://docs.rs/zip_static_handler/badge.svg)](https://docs.rs/zip_static_handler) ![Rust 1.75](https://img.shields.io/badge/rustc-1.75-ab6000.svg)

HTTP handler implementation for serving static content from a zip archive. 

## Handler conventions

The conventions derive from a position that can be controversial.

***Urls for page documents should not include the file extension, and they should not have a trailing slash.***

  - `/about` rather than `/about/`, `/about.html`, `/about.php` or `/about.md`.
  - The technical choice of whether `/about` points to an html file or a directory index should be transparent.

Derived conventions:
   - HTML files are served without the `.html` prefix.


   - There's no directory index except for the root

     `/index.html` is served as `http(s)://domain.tld` 

The decision to remove trailing slashes is incompatible with directory indices because in an index file, you expect relative links to refer to the same directory. However, without the trailing slash, they would refer to the parent directory instead.


## (Pre)compression

Compressible content types can be pre-compressed by including the compressed version of the file in the zip archive. Only [brotli](https://caniuse.com/brotli) compression is supported.

The compressed file should be in the same directory as the original file, and have the same name with an additional `.br` suffix. (`about.html` and `about.html.br`).

The uncompressed file should also be present in the archive. It is used to compute the [Etag](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag).

Pre-compressed files are not checked. They are not decompressed to make sure that they match the content of the uncompressed file.

If a pre-compressed file is missing for a compressible content-type, the compressed version is computed during the creation of the Handler instance. This means that you do not have to include pre-compressed files in the zip archive, but the consequence is a substantial increase in time and cpu usage at the creation of the Handler instance.

## Usage

The only argument that the builder requires is the zip archive content as bytes.
```rust
let zip_bytes: &[u8] = download_zip();
let handler = Handler::builder()
    .with_zip(zip_bytes)
    .try_build()?;
```

There are helper functions to download a zip archive from a github repository (by branch, tag or commit hash).

You can specify a prefix both for the path and for the zip content. If the zip is the export of a github repository for instance, you probably want to get rid of the `repositiory-${branch_or_tag_or_commit_hash}/` prefix.

```rust
let zip_bytes = download(&zip_download_branch_url(
     "programingjd",
     "about.programingjd.me",
     "main",
))
.await?;
let handler = Handler::builder()
     .with_zip_prefix("about.programingjd.me-main/")
     .wit_path_prefix("about")
     .with_zip(zip_bytes)
     .try_build()?;
```

If you are creating a new handler after each repository update, you can provide the previous handler for diffing.
<br>This is particularly useful when the content is not pre-compressed and you let the handler take care of the compression.
All the unchanged files that need to be compressed will be copied from the old handler rather than compressed again.

```rust
let handler = Handler::builder()
     .with_zip_prefix("about.programingjd.me-main/")
     .with_zip(zip_bytes)
     .with_diff(&previous_handler)
     .try_build()?;
```

## Features

You can choose the implementation of HTTP request and response that you need by enabling the appropriate feature: 

- hyper

  example: [hyper.rs](./examples/hyper.rs)


- axum

  example: [axum.rs](./examples/axum.rs)


- actix

  example: [actix.rs](./examples/actix.rs)


- rocket

  example: [rocket.rs](examples/rocket.rs)


## Examples

There are examples for the different http implementations that can be enabled
with the matching feature.

The [hyper.rs](./examples/hyper.rs) example shows how to customize which file types are accepted and which headers are set on the responses. 

The [axum.rs](./examples/axum.rs) example shows how to add [tracing](https://github.com/tokio-rs/tracing).

The [auto_update.rs](./examples/auto_update.rs) example shows how to add a webhook that updates the handler by downloading a new version of the zip archive and rebuilding the handler from that new zip archive.
