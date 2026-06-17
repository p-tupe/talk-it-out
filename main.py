import io
import struct

import httpx
import readabilipy
from kokoro_onnx import Kokoro
from quart import Quart, Response, request

app = Quart(__name__)
kokoro = Kokoro("./model.onnx", "./voices.bin")


@app.route("/stream")
async def stream():
    url = request.args.get("url")
    if not url:
        return {"error": "missing 'url'"}, 400

    html = httpx.get(url, timeout=10).text
    if html is None:
        return {"error": "invalid response from url"}, 500

    article = readabilipy.simple_json_from_html_string(html).get("plain_text") or []
    paragraphs = "\n".join(
        (p["text"] for p in article if isinstance(p.get("text"), str))
    )

    stream = kokoro.create_stream(
        paragraphs,
        voice="af_heart",
        speed=1.0,
        lang="en-us",
    )

    async def generate():
        first = True
        async for samples, sr in stream:
            if first:
                yield wav_header(sr)
                first = False

            yield (samples * 32767).astype("<i2").tobytes()

    return Response(generate(), mimetype="audio/wav")


def wav_header(sample_rate):
    buf = io.BytesIO()
    buf.write(b"RIFF")
    buf.write(b"\xff\xff\xff\xff")
    buf.write(b"WAVE")
    buf.write(b"fmt ")
    buf.write(struct.pack("<IHHIIHH", 16, 1, 1, sample_rate, sample_rate * 2, 2, 16))
    buf.write(b"data")
    buf.write(b"\xff\xff\xff\xff")
    return buf.getvalue()
