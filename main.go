package main

import (
	"io"
	"log/slog"
	"net/http"
	"os"
	"os/exec"
	"time"

	"codeberg.org/readeck/go-readability/v2"
)

// max 4 concurrent kokoro processes
var ttsLimit = make(chan struct{}, 4)

func main() {
	println("\n\n\t------- TIO: Talk It Out -------\n\n")

	http.HandleFunc("/", rootHandler)
	http.HandleFunc("/test_page", func(w http.ResponseWriter, r *http.Request) {
		slog.Info("Request:", r.Method, r.URL)
		w.Header().Set("Content-Type", "text/html")
		w.Write([]byte(`
        <input id="url" placeholder="Enter URL" size="50">
        <button onclick="play()">Play Now</button>
        <audio controls id="player" hidden></audio>
        <script>
            function play() {
                const url = document.getElementById('url').value;
                document.getElementById('player').src = '/?url=' + encodeURIComponent(url);
            }
        </script>
    `))
	})
	http.ListenAndServe("127.0.0.1:8080", nil)
}

func rootHandler(w http.ResponseWriter, r *http.Request) {
	slog.Info("Request:", r.Method, r.URL)

	select {
	case ttsLimit <- struct{}{}:
	case <-r.Context().Done():
		http.Error(w, "timeout", 503)
		return
	}
	defer func() { <-ttsLimit }()

	query := r.URL.Query()

	if !query.Has("url") {
		slog.Error("Response:", "status", http.StatusBadRequest, "error", "url not present")
		w.WriteHeader(400)
		return
	}

	article, err := readability.FromURL(query.Get("url"), 5*time.Second)
	if err != nil {
		slog.Error("Response:", "status", http.StatusInternalServerError, "error", err.Error())
		w.WriteHeader(500)
		return
	}

	tempF, err := os.CreateTemp("", time.Now().String()+".txt")
	if err != nil {
		slog.Error("Response:", "status", http.StatusInternalServerError, "error", err.Error())
		w.WriteHeader(500)
		return
	}
	defer tempF.Close()

	err = article.RenderText(tempF)
	if err != nil {
		slog.Error("Response:", "status", http.StatusInternalServerError, "error", err.Error())
		w.WriteHeader(500)
		return
	}
	defer tempF.Close()

	home, err := os.UserHomeDir()
	if err != nil {
		slog.Error("Response:", "status", http.StatusInternalServerError, "error", err.Error())
		w.WriteHeader(500)
		return
	}

	cmd := exec.Command(
		"kokoro-tts", tempF.Name(),
		"--model", home+"/.local/share/kokoro-tts/kokoro-v1.0.onnx",
		"--voices", home+"/.local/share/kokoro-tts/voices-v1.0.bin",
		"--format", "mp3",
		"--voice", "af_heart:50,af_bella:50",
		"--stream",
	)

	out, err := cmd.StdoutPipe()
	if err != nil {
		slog.Error("Response:", "status", http.StatusInternalServerError, "error", err.Error())
		w.WriteHeader(500)
		return
	}

	cmd.Start()
	defer cmd.Wait()

	w.Header().Set("Content-Type", "audio/mpeg")
	io.Copy(w, out)
}
