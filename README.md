fanlin-rs
===============================================================================

This application is just a thin wrapper for image processing libraries.
Most of all jobs are done by awesome crates.
Although there are some todo yet, this repository is aimed to be a yet another [fanlin](https://github.com/livesense-inc/fanlin).

## Development

```
$ docker compose up
$ make create-s3-bucket
$ make copy-object SRC=/path/to/local/image.png DEST=images/image.png
$ cargo run --release

# http://127.0.0.1:3000/foo/image.png?w=1000&h=500
# http://127.0.0.1:3000/bar/image.png?w=1000&h=500
```

## Processing parameter via query string

| parameter | description | example |
| --- | --- | --- |
| `w` | width | `w=200` |
| `h` | height | `h=100` |
| `rgb` | fill color | `rgb=32,32,32` |
| `quality` | encoding quality | `quality=85` |
| `crop` | cropping | `crop=true` |
| `webp` | encoding format | `webp=true` |

## Server settings with JSON

Please see an example file named with `fanlin.json` in the root directory.
