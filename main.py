import asyncio
import logging
import os

import httpx
import lameenc
import onnxruntime as rt
import readabilipy
from dotenv import load_dotenv
from kokoro_onnx import Kokoro
from piper import PiperVoice
from quart import Quart, Response, request, send_from_directory

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
kokoro.create("Hello.", voice="af_heart")
voices = {
    "fr-fr": PiperVoice.load("./voices/fr_FR-siwis-medium.onnx"),
    # "en-us": PiperVoice.load("./voices/en_US-lessac-high.onnx"),
}


load_dotenv()
MAC_TTS_URL = os.getenv("MAC_TTS_URL", "")


@app.route("/tts")
async def tts():
    text = request.args.get("text", "")
    lang = request.args.get("lang", "en-us")
    if not text:
        return Response(b"missing text", status=400)
    response = Response(tts_stream(text, lang), mimetype="audio/mpeg")
    response.timeout = None
    return response


@app.route("/stream")
async def stream():
    url = request.args.get("url")
    if not url:
        log.error("missing url")
        return Response(
            tts_stream("error: missing url parameter"), mimetype="audio/mpeg"
        )

    try:
        async with httpx.AsyncClient(timeout=10) as client:
            resp = await client.get(url)
            resp.raise_for_status()
            html = resp.text
    except httpx.HTTPStatusError as e:
        log.error("invalid response from url: %d", e.response.status_code)
        return Response(
            tts_stream("error: could not get a response from url"),
            mimetype="audio/mpeg",
        )
    except Exception as e:
        log.error("invalid response from url: %w", str(e))
        return Response(
            tts_stream("error: could not get a response from url"),
            mimetype="audio/mpeg",
        )

    article = readabilipy.simple_json_from_html_string(html).get("plain_text") or []
    content = "\n".join(p["text"] for p in article if isinstance(p.get("text"), str))
    response = Response(tts_stream(content), mimetype="audio/mpeg")
    response.timeout = (
        None  # streaming can take longer than Quart's 60s RESPONSE_TIMEOUT
    )
    return response


@app.route("/")
async def home():
    return await send_from_directory("./", "index.html")


@app.route("/logo.png")
async def static_files():
    return await send_from_directory("./", "logo.png")


async def tts_stream(content: str, lang="en-us"):
    """Try Mac if connected, else run wherever this is being served."""
    if MAC_TTS_URL:
        try:
            async with (
                httpx.AsyncClient(timeout=5) as client,
                client.stream(
                    "GET",
                    MAC_TTS_URL,
                    params={"text": content, "lang": lang},
                ) as resp,
            ):
                resp.raise_for_status()
                async for chunk in resp.aiter_bytes():
                    yield chunk
            return
        except Exception:
            log.info("Mac TTS unavailable — falling back to local")

    async for chunk in gen_local(content, lang):
        yield chunk


async def gen_local(content: str, lang="en-us"):
    piper_stream, kokoro_steam = None, None

    if lang == "fr-fr":
        piper_stream = voices[lang]
    else:
        kokoro_steam = kokoro.create_stream(
            content,
            voice="af_heart",
            speed=1.0,
            lang=lang,
        )

    enc = lameenc.Encoder()
    enc.set_channels(1)
    enc.set_in_sample_rate(24000)
    enc.set_quality(5)
    enc.set_bit_rate(128)

    try:
        if piper_stream is not None:
            for chunk in piper_stream.synthesize(content):
                yield enc.encode(chunk.audio_int16_bytes)
        elif kokoro_steam is not None:
            async for chunk, _ in kokoro_steam:
                raw = (chunk * 32767).astype("<i2").tobytes()
                yield enc.encode(raw)
            yield enc.flush()
        else:
            log.error("no stream found")
            raise Exception("No stream found")
    except (asyncio.CancelledError, GeneratorExit):
        log.info("client disconnected")
    finally:
        enc = None
