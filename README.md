[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/livesense-inc/fanlin-rs)
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
* Dependencies:
  * `liblcms2-dev`

```
$ docker compose up
$ make create-s3-bucket
$ make copy-object SRC=/path/to/local/image.png DEST=images/image.png
$ cp /path/to/local/image.png tmp/
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
| `blur` | blur image with sigma | `blur=10` |
| `grayscale` | grayscale colors | `grayscale=true` |
| `inverse` | inverse colors | `inverse=true` |
| `avif` | encoding format | `avif=true` |
| `webp` | encoding format | `webp=true` |

The aspect ratio is preserved at resizing. Also GIF animation too as well.

## Server settings with JSON

Please see an example file named with `fanlin.json` in the root directory.
Also, you can pass the settings as JSON in an argument like this:

```
$ cat fanlin.json | jq -c . | xargs -0 cargo run --release -- -j
```

## Benchmark
```
$ lscpu | grep -i 'model name'
Model name:                           13th Gen Intel(R) Core(TM) i7-13700HX
```

### fanlin-go
```
echo 'GET http://127.0.0.1:3000/baz/Lenna.jpg?w=300&h=200&rgb=32,32,32' | vegeta attack -header='user-agent: vegeta' -rate=50 -duration=180s | tee results.bin | vegeta report
Requests      [total, rate, throughput]         9000, 50.01, 50.00
Duration      [total, attack, wait]             3m0s, 3m0s, 30.845ms
Latencies     [min, mean, 50, 90, 95, 99, max]  28.919ms, 30.867ms, 30.842ms, 31.891ms, 32.196ms, 32.842ms, 52.06ms
Bytes In      [total, mean]                     87183000, 9687.00
Bytes Out     [total, mean]                     0, 0.00
Success       [ratio]                           100.00%
Status Codes  [code:count]                      200:9000
Error Set:
```

```
$ echo 'GET http://127.0.0.1:3000/baz/Lenna.jpg?w=300&h=200&rgb=32,32,32&webp=true&quality=20' | vegeta attack -header='user-agent: vegeta' -rate=50 -duration=180s | tee results.bin | vegeta report
Requests      [total, rate, throughput]         9000, 50.01, 50.00
Duration      [total, attack, wait]             3m0s, 3m0s, 35.812ms
Latencies     [min, mean, 50, 90, 95, 99, max]  34.405ms, 36.405ms, 36.351ms, 37.472ms, 37.768ms, 38.424ms, 41.586ms
Bytes In      [total, mean]                     24480000, 2720.00
Bytes Out     [total, mean]                     0, 0.00
Success       [ratio]                           100.00%
Status Codes  [code:count]                      200:9000
Error Set:
```

### fanlin-rs
```
$ echo 'GET http://127.0.0.1:3000/baz/lenna.jpg?w=300&h=200' | vegeta attack -header='user-agent: vegeta' -rate=50 -duration=180s | tee results.bin | vegeta report
Requests      [total, rate, throughput]         9000, 50.01, 50.00
Duration      [total, attack, wait]             3m0s, 3m0s, 17.893ms
Latencies     [min, mean, 50, 90, 95, 99, max]  17.066ms, 18.127ms, 18.058ms, 18.769ms, 19.02ms, 19.68ms, 22.793ms
Bytes In      [total, mean]                     144189000, 16021.00
Bytes Out     [total, mean]                     0, 0.00
Success       [ratio]                           100.00%
Status Codes  [code:count]                      200:9000
Error Set:
```

```
$ echo 'GET http://127.0.0.1:3000/baz/lenna.jpg?w=300&h=200&webp=true&quality=20' | vegeta attack -header='user-agent: vegeta' -rate=50 -duration=180s | tee results.bin | vegeta report
Requests      [total, rate, throughput]         9000, 50.01, 50.00
Duration      [total, attack, wait]             3m0s, 3m0s, 22.188ms
Latencies     [min, mean, 50, 90, 95, 99, max]  21.45ms, 22.448ms, 22.391ms, 23.07ms, 23.285ms, 23.698ms, 27.456ms
Bytes In      [total, mean]                     24156000, 2684.00
Bytes Out     [total, mean]                     0, 0.00
Success       [ratio]                           100.00%
Status Codes  [code:count]                      200:9000
Error Set:
```

```
$ echo 'GET http://127.0.0.1:3000/baz/lenna.jpg?w=300&h=200' | vegeta attack -header='user-agent: vegeta' -rate=500 -duration=180s | tee results.bin | vegeta report
Requests      [total, rate, throughput]         90000, 500.00, 499.93
Duration      [total, attack, wait]             3m0s, 3m0s, 26.676ms
Latencies     [min, mean, 50, 90, 95, 99, max]  17.075ms, 22.153ms, 20.524ms, 29.73ms, 32.962ms, 35.005ms, 66.47ms
Bytes In      [total, mean]                     1441890000, 16021.00
Bytes Out     [total, mean]                     0, 0.00
Success       [ratio]                           100.00%
Status Codes  [code:count]                      200:90000
Error Set:
```
