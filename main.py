import logging
import os

import httpx
import readabilipy
from kokoro_onnx import Kokoro
from quart import Quart, Response, request, send_from_directory
import lameenc
import onnxruntime as rt
import asyncio

n_threads = os.cpu_count() or 4

app = Quart(__name__)

logging.basicConfig(
    level=logging.INFO,
    format="%(levelname)s: %(message)s",
)
logging.getLogger("httpx").setLevel(logging.ERROR)
logging.getLogger("phonemizer").setLevel(logging.ERROR)
log = logging.getLogger()


sess_opts = rt.SessionOptions()
sess_opts.intra_op_num_threads = n_threads
sess_opts.inter_op_num_threads = max(1, n_threads // 2)
sess_opts.graph_optimization_level = rt.GraphOptimizationLevel.ORT_ENABLE_ALL
sess_opts.enable_cpu_mem_arena = False

sess = rt.InferenceSession(
    "./model.onnx", sess_opts, providers=["CPUExecutionProvider"]
)

kokoro = Kokoro.from_session(sess, "./voices.bin")
kokoro.create("Hello.", voice="af_heart")  # warmup


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
    return Response(generate(content), mimetype="audio/mpeg")


async def generate(content: str):
    stream = kokoro.create_stream(
        content,
        voice="af_heart",
        speed=1.0,
        lang="en-us",
    )

    enc = lameenc.Encoder()
    enc.set_channels(1)
    enc.set_in_sample_rate(24000)
    enc.set_quality(5)
    enc.set_bit_rate(128)

    try:
        async for samples, _ in stream:
            raw = (samples * 32767).astype("<i2").tobytes()
            yield enc.encode(raw)
        yield enc.flush()
    except (asyncio.CancelledError, GeneratorExit):
        log.info("client disconnected")
    finally:
        enc = None
        await stream.aclose()


@app.route("/")
async def home():
    return await send_from_directory("./", "index.html")


@app.route("/logo.png")
async def static_files():
    return await send_from_directory("./", "logo.png")
