# zip_static_handler

Http handler implementation for serving static content from a zip archive. 

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
