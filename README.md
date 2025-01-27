fanlin-rs
===============================================================================

WIP, just an experiment, won't be released

```
$ docker compose up
$ make create-s3-bucket
$ make copy-object SRC=/path/to/local/foo.png DEST=foo.png
$ cargo run --release

# http://127.0.0.1:3000/foo.png?w=2000&h=1000
```
