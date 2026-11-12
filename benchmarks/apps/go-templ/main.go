// Command harbor-go-templ serves the Harbor Market storefront with net/http and templ.
package main

//go:generate templ generate
//go:generate tailwindcss --input assets/app.css --output assets/dist/app.css --minify

import (
	"context"
	"errors"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"os/signal"
	"runtime"
	"strconv"
	"syscall"
	"time"

	"github.com/klauspost/compress/gzhttp"

	"harbor-go-templ/internal/catalog"
)

func main() {
	if err := run(); err != nil {
		log.Fatal(err)
	}
}

func run() error {
	port := envOr("PORT", "3000")
	dataDir := envOr("BENCH_DATA_DIR", "../../data")
	threads, err := strconv.Atoi(envOr("BENCH_THREADS", "0"))
	if err != nil || threads < 0 {
		return fmt.Errorf("invalid BENCH_THREADS %q", os.Getenv("BENCH_THREADS"))
	}
	compression := envOr("BENCH_COMPRESSION", "off")
	if compression != "on" && compression != "off" {
		return fmt.Errorf("invalid BENCH_COMPRESSION %q", compression)
	}

	// 0 keeps the default, one P per CPU core.
	if threads > 0 {
		runtime.GOMAXPROCS(threads)
	}

	store, err := catalog.Load(dataDir)
	if err != nil {
		return fmt.Errorf("failed to load data: %w", err)
	}

	var handler http.Handler = routes(&app{store: store})
	if compression == "on" {
		handler = gzhttp.GzipHandler(handler)
	}
	server := &http.Server{
		Addr:              net.JoinHostPort("127.0.0.1", port),
		Handler:           handler,
		ReadHeaderTimeout: 10 * time.Second,
		IdleTimeout:       2 * time.Minute,
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	errs := make(chan error, 1)
	go func() { errs <- server.ListenAndServe() }()
	select {
	case err := <-errs:
		return err
	case <-ctx.Done():
	}
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := server.Shutdown(shutdownCtx); err != nil && !errors.Is(err, http.ErrServerClosed) {
		return err
	}
	return nil
}

func envOr(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}
