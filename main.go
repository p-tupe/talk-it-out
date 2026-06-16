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

func main() {
	println("\n\n\t------- TIO: Talk It Out -------\n\n")
	http.HandleFunc("/", rootHandler)
	http.ListenAndServe("127.0.0.1:8080", nil)
}

func rootHandler(w http.ResponseWriter, r *http.Request) {
	slog.Info("Request:", r.Method, r.URL)

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
		"--voice", "af_heart:20,af_bella:80",
		"--stream",
	)

	out, err := cmd.StdoutPipe()
	if err != nil {
		slog.Error("Response:", "status", http.StatusInternalServerError, "error", err.Error())
		w.WriteHeader(500)
		return
	}

	cmd.Start()
	io.Copy(w, out)
	cmd.Wait()
}
