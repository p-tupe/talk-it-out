import io
import logging
import struct

import httpx
import readabilipy
from kokoro_onnx import Kokoro
from quart import Quart, Response, request, send_from_directory

app = Quart(__name__)
kokoro = Kokoro("./model.onnx", "./voices.bin")

logging.basicConfig(
    level=logging.INFO,
    format="%(levelname)s: %(message)s",
)
logging.getLogger("httpx").setLevel(logging.ERROR)
logging.getLogger("phonemizer").setLevel(logging.ERROR)
log = logging.getLogger()


@app.route("/stream")
async def stream():
    url = request.args.get("url")
    if not url:
        log.error("missing url")
        return Response(generate("Error: missing url parameter"), mimetype="audio/wav")

    try:
        async with httpx.AsyncClient(timeout=10) as client:
            resp = await client.get(url)
            resp.raise_for_status()
            html = resp.text
    except httpx.HTTPStatusError as e:
        log.error("invalid response from url: %d", e.response.status_code)
        return Response(
            generate("Error: could not get a response from url"), mimetype="audio/wav"
        )
    except Exception as e:
        log.error("invalid response from url: %w", str(e))
        return Response(
            generate("Error: could not get a response from url"), mimetype="audio/wav"
        )

    article = readabilipy.simple_json_from_html_string(html).get("plain_text") or []
    content = "\n".join((p["text"] for p in article if isinstance(p.get("text"), str)))
    return Response(generate(content), mimetype="audio/wav")


async def generate(content: str):
    stream = kokoro.create_stream(
        content,
        voice="af_heart",
        speed=1.0,
        lang="en-us",
    )

    first = True
    async for samples, sr in stream:
        if first:
            yield wav_header(sr)
            first = False

        yield (samples * 32767).astype("<i2").tobytes()


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


@app.route("/")
async def home():
    return await send_from_directory("./", "index.html")
