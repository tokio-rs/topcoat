// Package assets embeds the stylesheet and the script and serves them under content-hashed URLs.
package assets

import (
	"bytes"
	_ "embed"
	"fmt"
	"hash/fnv"
	"net/http"
	"time"
)

var (
	// The stylesheet compiled by `go generate` with the Tailwind CSS CLI.
	//go:embed dist/app.css
	stylesheet []byte
	//go:embed app.js
	script []byte
)

var (
	// StylesheetPath is the URL of the stylesheet.
	StylesheetPath = hashedPath("app", stylesheet, ".css")
	// ScriptPath is the URL of the script.
	ScriptPath = hashedPath("app", script, ".js")
)

// Register adds the asset routes to mux.
func Register(mux *http.ServeMux) {
	mux.Handle("GET "+StylesheetPath, immutable(StylesheetPath, stylesheet))
	mux.Handle("GET "+ScriptPath, immutable(ScriptPath, script))
}

// immutable serves content that never changes under its URL, so browsers may cache it forever.
// The Content-Type is derived from the file extension.
func immutable(name string, content []byte) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "public, max-age=31536000, immutable")
		http.ServeContent(w, r, name, time.Time{}, bytes.NewReader(content))
	})
}

// hashedPath is a cache-busting URL with a 64-bit FNV-1a hash of the content.
func hashedPath(name string, content []byte, extension string) string {
	h := fnv.New64a()
	h.Write(content)
	return fmt.Sprintf("/assets/%s-%016x%s", name, h.Sum64(), extension)
}
