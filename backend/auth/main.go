package main

import (
	"bytes"
	"crypto/tls"
	"encoding/json"
	"io"
	"log"
	"my-auth/templates"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/jwtauth"
)

type Credentials struct {
	UserName string `json:"username"`
	Password string `json:"password"`
}

type TpassToken struct {
	Token string `json:"token"`
}

var tokenAuth *jwtauth.JWTAuth

func init() {
	tokenAuth = jwtauth.New("HS256", []byte("monkey jizz horse fart"), nil)
}

func genTokenHandler(w http.ResponseWriter, r *http.Request) {
	var cred Credentials
	err := json.NewDecoder(r.Body).Decode(&cred)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	jsonBit, err := json.Marshal(cred)
	if err != nil {
		log.Println(err)
		json.NewEncoder(w).Encode(map[string]string{"error": "username or password could not be read."})
		return
	}
	//reach out to tpass for the user pass check
	//"https://devsys01.tpassvms.com/TpassPVService/"
	// Create a custom HTTP client with a custom Transport
	// that ignores certificate validation
	tr := &http.Transport{
		TLSClientConfig: &tls.Config{InsecureSkipVerify: true},
	}
	client := &http.Client{Transport: tr}

	envVar := os.Getenv("TPASS_ADDR") + "api/token"
	log.Println(envVar)
	//response, err := http.Post("https://devsys01.tpassvms.com/TpassPVService/api/token", "application/json", bytes.NewBuffer(jsonBit))
	response, err := client.Post(envVar, "application/json", bytes.NewBuffer(jsonBit))
	if err != nil {
		log.Println(err)
		json.NewEncoder(w).Encode(map[string]string{"error": "could not reach tpass for auth check"})
		return
	}

	defer response.Body.Close()

	body, err := io.ReadAll(response.Body)
	if err != nil {
		log.Println(err)
		json.NewEncoder(w).Encode(map[string]string{"error": "could not get token from tpass"})
		return
	}

	var tpassTk TpassToken

	err = json.Unmarshal(body, &tpassTk)
	if err != nil {
		log.Println("Token no bueno")
		json.NewEncoder(w).Encode(map[string]string{"error": "invalid credentials!"})
		return
	}

	log.Println("======  the tpass token ===== ")
	log.Println(tpassTk)

	// this may seem odd since we just got a token, but that was just to piggyback on existing auth.
	// we really just want a true / false that the credentials are good but the api doesn't give us that, we get a token or nothing.
	// But we still want our ouwn jwtoken so that we can verify it and make sure it adheres to our rules.
	_, tokenString, err := tokenAuth.Encode(map[string]interface{}{
		"username": cred.UserName,
		"exp":      time.Now().Add(time.Hour * 72).Unix(),
	})
	if err != nil {
		json.NewEncoder(w).Encode(map[string]string{"error": "coul not create token"})
		return
	}

	cookie := http.Cookie{
		Name:     "fr_token",
		Value:    tokenString,
		HttpOnly: true,
		Path:     "/",
		Expires:  time.Now().Add(time.Hour * 72),
	}

	http.SetCookie(w, &cookie)

	json.NewEncoder(w).Encode(map[string]string{"token": tokenString})
}

func getAuthToken(r *http.Request) string {
	authHeader := r.Header.Get("Authorization")
	if authHeader == "" {
		return "" // should probably send an error and ensure login page is ready for another try
	}

	parts := strings.Split(authHeader, " ")
	if len(parts) != 2 || strings.ToLower(parts[0]) != "bearer" {
		return ""
	}

	return parts[1]
}

func verifyAuthHeader(w http.ResponseWriter, r *http.Request) {
	tokenString := getAuthToken(r)
	println(tokenString)

	if tokenString == "" {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		json.NewEncoder(w).Encode(map[string]string{"error": "auth token found"})
		return
	}

	_, err := jwtauth.VerifyToken(tokenAuth, tokenString)
	if err != nil {

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
	}
}

func verifyCookie(w http.ResponseWriter, r *http.Request) {
	cookie, err := r.Cookie("fr_token")
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		json.NewEncoder(w).Encode(map[string]string{"error": "auth token found"})
		return
	}

	_, err = jwtauth.VerifyToken(tokenAuth, cookie.Value)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		json.NewEncoder(w).Encode(map[string]string{"error": "auth token invalid"})
	}
}

func logoutHandler(w http.ResponseWriter, r *http.Request) {
	cookie := http.Cookie{
		Name:     "fr_token",
		Value:    "",
		HttpOnly: true,
		Path:     "/",
		Expires:  time.Unix(0, 0),
	}

	http.SetCookie(w, &cookie)
}

func renderLoginHandler(w http.ResponseWriter, r *http.Request) {
	component := templates.Index()
	component.Render(r.Context(), w)
}

func main() {
	r := chi.NewRouter()
	r.Use(middleware.Logger)

	workDir, _ := os.Getwd()
	filesDir := http.Dir(filepath.Join(workDir, "assets"))
	FileServer(r, "/assets", filesDir)

	// r.Get("/", templ.Handler(templates.Index(0, 0)).ServeHTTP)
	r.Post("/getToken", genTokenHandler)
	r.Get("/auth", renderLoginHandler)
	r.Get("/", renderLoginHandler)
	r.Get("/logout", logoutHandler)
	r.Get("/verify-auth-header", verifyAuthHeader)
	r.Get("/verify-cookie", verifyCookie)

	println("Server running on port 3001")
	http.ListenAndServe(":3001", r)
}

// FileServer conveniently sets up a http.FileServer handler to serve
// static files from a http.FileSystem.
func FileServer(r chi.Router, path string, root http.FileSystem) {
	if strings.ContainsAny(path, "{}*") {
		panic("FileServer does not permit any URL parameters.")
	}

	if path != "/" && path[len(path)-1] != '/' {
		r.Get(path, http.RedirectHandler(path+"/", 301).ServeHTTP)
		path += "/"
	}
	path += "*"

	r.Get(path, func(w http.ResponseWriter, r *http.Request) {
		rctx := chi.RouteContext(r.Context())
		pathPrefix := strings.TrimSuffix(rctx.RoutePattern(), "/*")
		fs := http.StripPrefix(pathPrefix, http.FileServer(root))
		fs.ServeHTTP(w, r)
	})
}
