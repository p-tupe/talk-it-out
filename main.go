package main

import (
	"errors"
	"fmt"
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
var counter = 0

func main() {
	http.Handle("/stream", streamHandler(stream))

	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
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

	port, ok := os.LookupEnv("PORT")
	if !ok || port == "" {
		port = "8083"
	}

	slog.Info("TIO listening on 127.0.0.1:" + port)
	http.ListenAndServe("127.0.0.1:"+port, nil)
}

type streamHandler func(w http.ResponseWriter, r *http.Request) error

func (s streamHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	counter += 1
	c := counter

	slog.Info("Request:", "id", c, r.Method, r.URL)

	if err := s(w, r); err != nil {
		slog.Error("Response:", "id", c, "status", http.StatusInternalServerError, "error", err.Error())
		w.WriteHeader(500)
	}
}

func stream(w http.ResponseWriter, r *http.Request) error {
	select {
	case ttsLimit <- struct{}{}:
	case <-r.Context().Done():
		return errors.New("Timeout")
	}
	defer func() { <-ttsLimit }()

	query := r.URL.Query()

	if !query.Has("url") {
		slog.Error("Response:", "status", http.StatusBadRequest, "error", "url not present")
		w.WriteHeader(400)
		return nil
	}

	article, err := readability.FromURL(query.Get("url"), 5*time.Second)
	if err != nil {
		return fmt.Errorf("readability frim url error: %w", err)
	}

	tempF, err := os.CreateTemp("", time.Now().String()+".txt")
	if err != nil {
		return fmt.Errorf("create temp file error: %w", err)
	}
	defer tempF.Close()
	defer os.Remove(tempF.Name())

	err = article.RenderText(tempF)
	if err != nil {
		return fmt.Errorf("article render text error: %w", err)
	}

	// We'll use this pipe to stream output from
	// tts directly to http response
	readPipe, writePipe := io.Pipe()

	cmd := exec.Command(
		"kokoro-tts", tempF.Name(),
		"--model", "./model.onnx",
		"--voices", "./voices.bin",
		"--format", "mp3",
		"--voice", "af_heart:50,af_bella:50",
		"--stream",
	)

	cmd.Stdout = writePipe
	cmd.Start()
	defer cmd.Wait()

	go func() {
		<-r.Context().Done()
		cmd.Process.Signal(os.Kill)
	}()

	w.Header().Set("Content-Type", "audio/mpeg")
	io.Copy(w, readPipe)
	return nil
}
