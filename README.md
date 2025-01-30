![Test](https://github.com/livesense-inc/fanlin-rs/actions/workflows/test.yaml/badge.svg?branch=main)

fanlin-rs
===============================================================================

This ia a web server to process and serve images.
The application is just a thin wrapper for image processing libraries.
Most of all jobs are done by awesome crates.
Although there are some todo yet,
this repository is aimed to be yet another [fanlin](https://github.com/livesense-inc/fanlin).

## Development

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

```
$ ./target/release/fanlin-rs --help
A web server to process and serve images

Usage: fanlin-rs [OPTIONS]

Options:
  -c, --conf <CONF>  Path of a setting file [default: fanlin.json]
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
