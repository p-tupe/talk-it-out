FROM ghcr.io/astral-sh/uv:debian

WORKDIR /app

RUN curl -sL -o voices.bin \
    "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin"

RUN curl -sL -o model.onnx \
    "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.onnx"

COPY pyproject.toml uv.lock .python-version  ./
RUN  uv sync --frozen --no-dev

COPY main.py  ./

EXPOSE 8080

CMD ["uv", "run", "uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8080"]
