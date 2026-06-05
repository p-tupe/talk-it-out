# Talk It Out

Turn any web page into an audio stream

## Setup

It works in two parts - a Rust server you host to turn urls into audio, and a Javascript widget embedded on your side that sends the url to your server.

### Server

```bash
cargo ...
```

### Widget

```js
fetch("https://tio.yourdomain.com?url=https://example.com") ...
```
