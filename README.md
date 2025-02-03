![Test](https://github.com/livesense-inc/fanlin-rs/actions/workflows/test.yaml/badge.svg?branch=main)
![Release](https://github.com/livesense-inc/fanlin-rs/actions/workflows/release.yaml/badge.svg)

fanlin-rs
===============================================================================

This ia a web server to process and serve images.
The application is just a thin wrapper for image processing libraries.
Most of all jobs are done by awesome crates.
Although there are some todo yet,
this repository is aimed to be yet another [fanlin](https://github.com/livesense-inc/fanlin).

## Development

* https://rustup.rs/
* https://docs.docker.com/manuals/
* https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html

```
$ docker compose up
$ make create-s3-bucket
$ make copy-object SRC=/path/to/local/image.png DEST=images/image.png
$ cargo run --release
```

* http://127.0.0.1:3000/foo/image.png?w=1618&h=1000
  * AWS S3 backend
* http://127.0.0.1:3000/bar/image.png?w=1618&h=1000
  * WEB service backend
* http://127.0.0.1:3000/baz/image.png?w=1618&h=1000
  * Local file backend

```
$ cargo run --release -- --help
    Finished `release` profile [optimized] target(s) in 0.21s
     Running `target/release/fanlin-rs --help`
A web server to process and serve images

Usage: fanlin-rs [OPTIONS]

Options:
  -c, --conf <CONF>  Path of a setting file [default: fanlin.json]
  -j, --json <JSON>  JSON data for setting
  -h, --help         Print help
  -V, --version      Print version
```

## Parameters for image processing via query string

| parameter | description | example |
| --- | --- | --- |
| `w` | width | `w=200` |
| `h` | height | `h=100` |
| `rgb` | fill color | `rgb=32,32,32` |
| `quality` | encoding quality | `quality=85` |
| `crop` | cropping | `crop=true` |
| `avif` | encoding format | `avif=true` |
| `webp` | encoding format | `webp=true` |

The aspect ratio is preserved at resizing. Also GIF animation too as well.

## Server settings with JSON

Please see an example file named with `fanlin.json` in the root directory.

Also, you can pass the settings as JSON in an argument like this:

```
$ cat fanlin.json | jq -c . | xargs -0 cargo run --release -- -j
```
