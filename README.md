<br />
<div style="width:50%" align="center"> <img src="./logo.png" alt="TIO Logo"> </div>
<p align="center"><strong>Turn any web page into an audio stream!</strong></p>
<br />

Check a demo out on: [app.priteshtupe.com/tio](https://app.priteshtupe.com/tio)

Keep in mind that the demo is running on a potato; TIO is meant for self-hosting.

## Setup

Docker is probably the easiest way to go -

```bash
git clone https://github.com/p-tupe/talk-it-out.git
cd talk-it-out
docker compose up -d
```

## Usage

This exposes a server with two routes: '/' and '/stream'

'/' serves a web page with an input for url and a player audio.

'/stream' is where the magic happens.

## Magic

'/stream' endpoint takes in a url as a query paramter. So if do something like:

```bash
curl -o output.mp3 'localhost:8083/stream?url=https://priteshtupe.com/posts/ip-on-tap/'
```

you'll get an `output.mp3` for whatever url you sent it. You can also stream it directly to a player if it accepts it.

```bash
cliamp 'localhost:8083/stream?url=https://priteshtupe.com/posts/ip-on-tap/'
```

The audio contains the meat of the webpage, cleaned up by using [`readabilipy`](https://github.com/alan-turing-institute/ReadabiliPy) and fed through [`kokoro-onnx`](https://github.com/thewh1teagle/kokoro-onnx) for generating audio and [`lameenc`](https://github.com/chrisstaite/lameenc) for encoding frames into mp3 for transport.

To embed an audio on your webpage, just do:

```html
<audio id="player" controls src='localhost:8083/stream?url=https://priteshtupe.com/posts/ip-on-tap/' />
```

> ofcourse, replace localhost with whatever ip/domain you're using and the url with that of your page.

And that's all!
