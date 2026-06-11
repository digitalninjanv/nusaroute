# NusaRoute AI Gateway

> **Self-hosted AI Gateway** — Satu endpoint OpenAI-compatible untuk routing ke berbagai provider AI. Ringan, cepat, dan mudah dikonfigurasi.

[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Status](https://img.shields.io/badge/status-MVP-yellow)]()

NusaRoute adalah **AI Gateway** open-source dan self-hosted yang menyediakan satu endpoint API OpenAI-compatible untuk merutekan request ke berbagai provider AI seperti OpenAI, OpenRouter, vLLM, LiteLLM, atau server LLM lokal. Cukup definisikan model alias (contoh: `smart-fast`, `smart-pro`) sekali di konfigurasi, dan semua client cukup menggunakan satu URL dan satu API key.

**Kenapa NusaRoute?** Karena gonta-ganti provider AI tidak harus mengubah kode aplikasi. Cukup edit satu file YAML, simpan, dan server otomatis menyesuaikan — tanpa restart, tanpa downtime.

> **Status proyek:** Saat ini MVP (Minimum Viable Product). Fokus pada routing OpenAI-compatible chat completions, streaming proxy, metrics, dan hot-reload konfigurasi.

---

## Daftar Isi

- [Fitur](#fitur)
- [Use Cases](#use-cases)
- [Cara Kerja](#cara-kerja)
- [Prerequisites (Untuk Pemula)](#prerequisites-untuk-pemula)
- [Instalasi](#instalasi)
  - [Linux / macOS](#linux--macos)
  - [Windows](#windows)
  - [Verifikasi Instalasi](#verifikasi-instalasi)
- [Konfigurasi Awal](#konfigurasi-awal)
- [Quickstart (3 Menit)](#quickstart-3-menit)
- [Referensi Konfigurasi](#referensi-konfigurasi)
  - [Server Config](#1-server-config)
  - [Provider Config](#2-provider-config)
  - [Model Route Config](#3-model-route-config)
  - [Full Example YAML](#4-full-example-yaml)
- [Environment Variables](#environment-variables)
- [Endpoint API](#endpoint-api)
  - [GET /healthz](#1-get-healthz)
  - [GET /v1/models](#2-get-v1models)
  - [POST /v1/chat/completions](#3-post-v1chatcompletions)
  - [POST /v1/chat/completions (Streaming)](#4-post-v1chatcompletions-streaming)
  - [POST /v1/admin/reload](#5-post-v1adminreload)
  - [GET /metrics](#6-get-metrics)
- [Hot Reload — Edit Tanpa Restart](#hot-reload--edit-tanpa-restart)
- [Client Configuration](#client-configuration)
- [Integrasi Bahasa Pemrograman](#integrasi-bahasa-pemrograman)
  - [Python](#python)
  - [JavaScript / TypeScript](#javascript--typescript)
  - [cURL](#curl)
- [Troubleshooting](#troubleshooting)
  - [Server Tidak Bisa Start](#1-server-tidak-bisa-start)
  - [Error 401 Unauthorized](#2-error-401-unauthorized)
  - [Error 400 Unknown Model](#3-error-400-unknown-model)
  - [Provider Gagal / 502](#4-provider-gagal--502)
  - [Config Reload Gagal](#5-config-reload-gagal)
- [Menjalankan Sebagai Service (Linux)](#menjalankan-sebagai-service-linux)
- [Production Deployment](#production-deployment)
- [Security Checklist](#security-checklist)
- [Development](#development)
  - [Struktur Project](#struktur-project)
  - [Perintah Development](#perintah-development)
- [Roadmap](#roadmap)
- [Dokumentasi Tambahan](#dokumentasi-tambahan)

---

## Fitur

| Fitur | Keterangan |
|-------|-----------|
| **OpenAI-compatible API** | `GET /v1/models` dan `POST /v1/chat/completions` — kompatibel dengan tools dan SDK OpenAI |
| **Provider registry via YAML** | Tambah, ubah, atau nonaktifkan provider tanpa compile ulang |
| **Model aliases** | Client pakai nama stabil (`smart-fast`), backend bebas ganti provider kapan saja |
| **Routing strategies** | `lowest_latency` (pilih tercepat via EWMA) atau `priority` (urutan prioritas) |
| **Fallback otomatis** | Jika provider pertama gagal, coba kandidat berikutnya |
| **Streaming proxy** | Forward Server-Sent Events dari upstream ke client secara real-time |
| **Bearer auth** | Token authentication dengan constant-time comparison (anti timing attack) |
| **Prometheus metrics** | Request count, failure count, dan latency EWMA per provider |
| **Hot-reload config** | Edit YAML → simpan → server otomatis pakai yang baru dalam ≤2 detik |
| **Admin reload API** | `POST /v1/admin/reload` — trigger reload manual via curl |
| **Rust + Axum** | Binary tunggal, memory rendah (~10MB), startup cepat (<100ms) |

---

## Use Cases

1. **Satu endpoint untuk semua client** — Hubungkan OpenCode, Continue, Cline, Cursor, VS Code extensions, dan tools AI lain ke satu URL. Ganti provider dari OpenAI ke OpenRouter atau lokal vLLM tanpa menyentuh konfigurasi tools satu per satu.

2. **Efisiensi biaya** — Route tugas sederhana (chat ringan, summarization) ke model murah/low-latency. Route tugas kompleks (coding, reasoning) ke model kuat. Semua otomatis via model alias.

3. **Resilience** — Jika OpenAI down atau rate-limited, fallback otomatis ke OpenRouter atau server lokal. Tidak ada single point of failure di sisi provider.

4. **Privasi / Offline** — Jalankan server lokal, route ke vLLM/Ollama lokal untuk data sensitif yang tidak boleh keluar jaringan.

5. **Development & Testing** — Satu config developer dengan mock provider. Satu config production dengan provider sungguhan. Cukup ganti `NUSAROUTE_CONFIG`.

---

## Cara Kerja

```
┌──────────────┐     ┌─────────────────────────────────────┐     ┌──────────────────┐
│              │     │         NusaRoute Gateway            │     │                  │
│   Client     │────▶│  Auth → Resolve Alias → Route       │────▶│  OpenAI           │
│  (curl/SDK)  │     │  → Fallback Loop → Stream Proxy     │     │  OpenRouter       │
│              │     │  → Metrics                          │     │  vLLM (lokal)     │
│  Model:      │     │                                     │     │  LiteLLM          │
│  "smart-pro" │     │  Config: config.yaml (hot-reload)   │     │  Custom Provider  │
└──────────────┘     └─────────────────────────────────────┘     └──────────────────┘
```

1. Client mengirim request `POST /v1/chat/completions` dengan model alias (misal: `smart-pro`)
2. Gateway memvalidasi token bearer
3. Gateway mencari definisi model alias `smart-pro` di konfigurasi
4. Strategi routing menentukan urutan provider (berdasarkan latency atau priority)
5. Gateway mencoba provider pertama. Jika gagal → fallback ke berikutnya
6. Response (atau stream) diteruskan ke client
7. Metrics diperbarui (request count, latency, failure)

---

## Prerequisites (Untuk Pemula)

Sebelum memulai, pastikan komputer Anda memiliki:

### 1. Rust Programming Language

| Minimum Versi | Cara Cek |
|--------------|----------|
| **Rust 1.85+** | Buka terminal/command prompt, ketik: |

```bash
rustc --version
```

Jika muncul `rustc 1.85.0` atau lebih baru, Anda siap. Jika belum:

**Linux / macOS:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Ikuti instruksi, pilih default (1)
source "$HOME/.cargo/env"
```

**Windows:**
- Download `rustup-init.exe` dari [https://rustup.rs](https://rustup.rs)
- Jalankan, pilih default
- Restart PowerShell/CMD

### 2. Git

```bash
git --version
# Harus muncul: git version 2.x.x
```

Jika belum: https://git-scm.com/downloads

### 3. cURL (untuk testing)

```bash
curl --version
```

Jika belum: `sudo apt install curl` (Linux) atau `brew install curl` (macOS).

### 4. System Requirements

| OS | Memory | Disk | Catatan |
|----|--------|------|---------|
| Linux (Ubuntu 22.04+, Debian 12+, Arch) | 256 MB+ | 50 MB (binary) + ~500 MB (build cache) | Produksi & development |
| macOS 12+ | 256 MB+ | 50 MB + ~500 MB | Development |
| Windows 10/11 | 512 MB+ | 50 MB + ~500 MB | Development via MSVC build tools |

> Build pertama membutuhkan kompilasi dependensi (~500 MB cache). Binary akhir hanya ~15-20 MB dan tidak perlu Rust untuk produksi (copy binary saja).

---

## Instalasi

### Linux / macOS

```bash
# 1. Clone repository
git clone https://github.com/your-org/nusaroute.git
cd nusaroute

# 2. Build binary production (butuh 1-3 menit pertama kali)
cargo build --release

# 3. (Opsional) Salin binary ke PATH
sudo cp target/release/nusaroute-ai-gateway /usr/local/bin/

# 4. Verifikasi
./target/release/nusaroute-ai-gateway --help
# Atau langsung lihat opsi: file akan error karena belum ada config — itu normal
```

### Windows

```powershell
# PowerShell (Run as Administrator)
git clone https://github.com/your-org/nusaroute.git
cd nusaroute

# Build
cargo build --release

# Binary siap di: .\target\release\nusaroute-ai-gateway.exe
```

### Verifikasi Instalasi

```bash
# Cek bahwa binary sudah terbentuk
ls -lh target/release/nusaroute-ai-gateway
# Output: -rwxr-xr-x ... 15M ... nusaroute-ai-gateway

# Cek bahwa Rust toolchain siap
rustc --version
cargo --version
```

---

## Konfigurasi Awal

NusaRoute membaca file YAML untuk konfigurasi. Salin file contoh dan edit:

```bash
# 1. Salin contoh config
cp config.example.yaml config.yaml

# 2. Edit sesuai kebutuhan
#    - Ganti API key minimal satu provider
#    - Atau nonaktifkan auth (kosongkan gateway_api_keys)
nano config.yaml   # atau vim, code, notepad++
```

> **Peringatan:** File `config.yaml` berisi API key dan rahasia. Jangan commit ke Git.
> File `config.example.yaml` aman untuk di-commit sebagai dokumentasi.

---

## Quickstart (3 Menit)

```bash
# Terminal 1: Jalankan server
export NUSAROUTE_CONFIG=config.yaml
export OPENROUTER_API_KEY="sk-or-v1-xxxxxxxxxxxxxxxxxxxx"
cargo run --release
```

```bash
# Terminal 2: Test
# 1. Health check
curl http://127.0.0.1:1789/healthz
# Output: {"status":"ok"}

# 2. Lihat daftar model alias
curl -H "Authorization: Bearer dev-local-key" \
  http://127.0.0.1:1789/v1/models

# 3. Chat completion dengan model alias
curl -X POST http://127.0.0.1:1789/v1/chat/completions \
  -H "Authorization: Bearer dev-local-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "smart-pro",
    "messages": [
      { "role": "user", "content": "Halo, siapa kamu?" }
    ]
  }'
```

Jika berhasil, server siap digunakan.

---

## Referensi Konfigurasi

### 1. Server Config

| Field | Tipe | Default | Deskripsi |
|-------|------|---------|-----------|
| `server.bind` | string (SocketAddr) | `"127.0.0.1:1789"` | Alamat dan port listen server |
| `server.request_timeout_ms` | integer | `45000` | Timeout HTTP request ke provider (ms) |
| `server.gateway_api_keys` | array of strings | `[]` | Daftar bearer token untuk auth. Kosongkan `[]` untuk disable auth |

```yaml
server:
  bind: "0.0.0.0:1789"         # Listen di semua interface (hati-hati!)
  request_timeout_ms: 60000     # 60 detik timeout
  gateway_api_keys:
    - "token-rahasia-saya"
    - "token-cadangan"
```

### 2. Provider Config

| Field | Tipe | Default | Deskripsi |
|-------|------|---------|-----------|
| `providers.<name>.type` | enum | — | Tipe provider. Saat ini hanya `"openai-compatible"` |
| `providers.<name>.base_url` | string | — | Base URL API provider (tanpa `/chat/completions`) |
| `providers.<name>.api_key_env` | string | — | Nama environment variable yang berisi API key |
| `providers.<name>.enabled` | boolean | `true` | Aktif/nonaktif. Berguna untuk mematikan sementara |
| `providers.<name>.priority` | integer | `100` | Prioritas routing. Semakin kecil semakin diprioritaskan |

```yaml
providers:
  openai:                                   # Nama internal, bebas
    type: "openai-compatible"               # Hanya ini yang didukung sekarang
    base_url: "https://api.openai.com/v1"   # Tanpa trailing slash
    api_key_env: "OPENAI_API_KEY"           # Wajib: environment variable
    enabled: true                           # false = skip saat routing
    priority: 20                            # priority 1 lebih tinggi dari 20
```

### 3. Model Route Config

| Field | Tipe | Default | Deskripsi |
|-------|------|---------|-----------|
| `models.<name>.description` | string | `""` | Deskripsi untuk ditampilkan di `/v1/models` |
| `models.<name>.strategy` | enum | `"lowest_latency"` | `"lowest_latency"` atau `"priority"` |
| `models.<name>.fallback` | boolean | `true` | Jika true, coba kandidat berikutnya saat gagal |
| `models.<name>.candidates` | array | `[]` | Daftar provider + model untuk dialiaskan |

Setiap candidate:

| Field | Tipe | Deskripsi |
|-------|------|-----------|
| `candidates[].provider` | string | Nama provider (harus cocok dengan `providers.*`) |
| `candidates[].model` | string | Nama model persis seperti yang dikenal provider |

```yaml
models:
  coder-cepat:
    description: "Model cepat untuk coding assistant"
    strategy: "lowest_latency"      # Pilih provider tercepat
    fallback: true                  # Coba provider lain jika gagal
    candidates:
      - provider: "local_vllm"
        model: "qwen-coder-fast"
      - provider: "openrouter"
        model: "openai/gpt-5.5-mini"
```

### 4. Full Example YAML

File lengkap: [`config.example.yaml`](config.example.yaml)

```yaml
server:
  bind: "127.0.0.1:1789"
  request_timeout_ms: 45000
  gateway_api_keys:
    - "dev-local-key"

providers:
  openrouter:
    type: "openai-compatible"
    base_url: "https://openrouter.ai/api/v1"
    api_key_env: "OPENROUTER_API_KEY"
    enabled: true
    priority: 10

  openai:
    type: "openai-compatible"
    base_url: "https://api.openai.com/v1"
    api_key_env: "OPENAI_API_KEY"
    enabled: false
    priority: 20

  local_vllm:
    type: "openai-compatible"
    base_url: "http://127.0.0.1:8000/v1"
    api_key_env: "LOCAL_LLM_API_KEY"
    enabled: false
    priority: 1

models:
  smart-fast:
    description: "Low-latency default for coding tools and chat"
    strategy: "lowest_latency"
    fallback: true
    candidates:
      - provider: "local_vllm"
        model: "qwen-coder-fast"
      - provider: "openrouter"
        model: "openai/gpt-5.5-mini"
      - provider: "openai"
        model: "gpt-5.5-mini"

  smart-pro:
    description: "Higher-quality route with fallback"
    strategy: "priority"
    fallback: true
    candidates:
      - provider: "openrouter"
        model: "nvidia/nemotron-3-ultra-550b-a55b:free"
      - provider: "openai"
        model: "gpt-5.5-mini"
```

---

## Environment Variables

| Variable | Wajib | Default | Deskripsi |
|----------|-------|---------|-----------|
| `NUSAROUTE_CONFIG` | Tidak | `config.example.yaml` | Path ke file konfigurasi YAML |
| `OPENROUTER_API_KEY` | Jika provider OpenRouter aktif | — | API key OpenRouter |
| `OPENAI_API_KEY` | Jika provider OpenAI aktif | — | API key OpenAI |
| `LOCAL_LLM_API_KEY` | Jika provider lokal butuh auth | — | API key server lokal (bisa dummy) |
| `RUST_LOG` | Tidak | — | Tracing level: `info`, `debug`, `trace`, `warn` |

Setiap provider yang `enabled: true` WAJIB memiliki environment variable sesuai `api_key_env`, atau server akan error saat startup.

Contoh set environment variable:

```bash
# Linux / macOS
export NUSAROUTE_CONFIG=config.yaml
export OPENROUTER_API_KEY="sk-or-v1-xxxx"
export RUST_LOG="nusaroute_ai_gateway=debug,tower_http=debug"

# Windows PowerShell
$env:NUSAROUTE_CONFIG="config.yaml"
$env:OPENROUTER_API_KEY="sk-or-v1-xxxx"
```

---

## Endpoint API

### 1. GET /healthz

Health check sederhana. Tidak perlu autentikasi.

```bash
curl http://127.0.0.1:1789/healthz
```

Response:
```json
{ "status": "ok" }
```

---

### 2. GET /v1/models

Mengembalikan daftar model alias yang terdaftar di konfigurasi. Format mengikuti OpenAI API.

```bash
curl -H "Authorization: Bearer dev-local-key" \
  http://127.0.0.1:1789/v1/models
```

Response:
```json
{
  "object": "list",
  "data": [
    {
      "id": "smart-fast",
      "object": "model",
      "owned_by": "nusaroute",
      "description": "Low-latency default for coding tools and chat"
    },
    {
      "id": "smart-pro",
      "object": "model",
      "owned_by": "nusaroute",
      "description": "Higher-quality route with fallback"
    }
  ]
}
```

> **Catatan:** `id` adalah nama model alias yang dipakai client di field `model`. Client tidak perlu tahu provider asli.

---

### 3. POST /v1/chat/completions

Endpoint utama untuk chat completion. Format body mengikuti OpenAI Chat Completions API.

**Request minimal:**
```bash
curl -X POST http://127.0.0.1:1789/v1/chat/completions \
  -H "Authorization: Bearer dev-local-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "smart-fast",
    "messages": [
      { "role": "user", "content": "Apa itu NusaRoute?" }
    ]
  }'
```

**Dengan system prompt:**
```bash
curl -X POST http://127.0.0.1:1789/v1/chat/completions \
  -H "Authorization: Bearer dev-local-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "smart-pro",
    "messages": [
      { "role": "system", "content": "Jawab dengan singkat dan dalam bahasa Indonesia." },
      { "role": "user", "content": "Jelaskan API gateway" }
    ]
  }'
```

**Dengan parameter tambahan:**
```bash
curl -X POST http://127.0.0.1:1789/v1/chat/completions \
  -H "Authorization: Bearer dev-local-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "smart-fast",
    "temperature": 0.2,
    "max_tokens": 500,
    "messages": [
      { "role": "user", "content": "Buat fungsi validasi email di TypeScript" }
    ]
  }'
```

> **Catatan:** Parameter tambahan (`temperature`, `max_tokens`, `top_p`, dll.) diteruskan ke provider upstream apa adanya. Response upstream juga diteruskan tanpa modifikasi.

---

### 4. POST /v1/chat/completions (Streaming)

Tambahkan `"stream": true` untuk menerima response streaming (Server-Sent Events).

```bash
curl -N http://127.0.0.1:1789/v1/chat/completions \
  -H "Authorization: Bearer dev-local-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "smart-fast",
    "stream": true,
    "messages": [
      { "role": "user", "content": "Tulis ringkasan tentang Rust dalam 3 kalimat" }
    ]
  }'
```

Parameter `-N` (--no-buffer) pada curl penting agar stream tidak tertunda.

---

### 5. POST /v1/admin/reload

Me-reload konfigurasi dari file tanpa restart server. Berguna untuk automation dan scripting.

```bash
curl -X POST http://127.0.0.1:1789/v1/admin/reload
```

Response sukses:
```json
{
  "status": "ok",
  "message": "config reloaded successfully"
}
```

Response gagal (config error):
```json
{
  "error": {
    "message": "config has no models",
    "type": "nusaroute_gateway_error"
  }
}
```

> **Penting:** Endpoint ini TIDAK memerlukan bearer token. Jika server di-expose ke jaringan, pastikan endpoint ini tidak bisa diakses publik (blokir via reverse proxy) atau aktifkan firewall.

---

### 6. GET /metrics

Mengembalikan metrik Prometheus-style untuk monitoring.

```bash
curl http://127.0.0.1:1789/metrics
```

Output:
```
# TYPE nusaroute_provider_requests_total counter
nusaroute_provider_requests_total{provider="openrouter"} 12

# TYPE nusaroute_provider_failures_total counter
nusaroute_provider_failures_total{provider="openrouter"} 1

# TYPE nusaroute_provider_latency_ewma_ms gauge
nusaroute_provider_latency_ewma_ms{provider="openrouter"} 923.400
```

Metrik ini bisa dikumpulkan oleh Prometheus dan divisualisasikan di Grafana.

---

## Hot Reload — Edit Tanpa Restart

Salah satu fitur unggulan NusaRoute adalah **hot-reload konfigurasi**. Anda bisa mengubah provider, model, atau pengaturan lain kapan saja tanpa menghentikan server.

### Cara 1: Auto-reload (Otomatis)

Cukup edit dan simpan file `config.yaml`. Server memeriksa perubahan file setiap 2 detik.

```bash
# 1. Edit config
nano config.yaml

# 2. Simpan (Ctrl+S, Ctrl+X)
#    Server otomatis mendeteksi perubahan dalam ≤2 detik

# 3. Cek daftar model terbaru tanpa restart
curl -H "Authorization: Bearer dev-local-key" \
  http://127.0.0.1:1789/v1/models
```

**Jika config baru valid:**
- Config baru di-swap secara atomic
- Request yang sedang berlangsung tetap menggunakan config lama hingga selesai
- Request baru langsung menggunakan config baru
- Tidak ada downtime

**Jika config baru error:**
- Server menulis warning ke log
- Config lama tetap berjalan
- Server tidak crash

### Cara 2: Manual via API

Untuk skrip deployment atau automation:

```bash
curl -X POST http://127.0.0.1:1789/v1/admin/reload
```

Berguna ketika Anda mengganti file config dari luar (scp, rsync, git pull) dan ingin memicu reload secara eksplisit.

---

## Client Configuration

Untuk menghubungkan tools atau aplikasi ke NusaRoute, gunakan pengaturan berikut:

| Setting | Value |
|---------|-------|
| **Base URL** | `http://127.0.0.1:1789/v1` |
| **API Key** | `dev-local-key` (atau token yang Anda set di config) |
| **Model** | Salah satu alias dari `/v1/models` (contoh: `smart-fast`) |

### OpenAI-compatible tools yang sudah teruji:

| Tool | Cara Setup |
|------|-----------|
| **OpenCode** | Settings → API Provider → OpenAI Compatible → Base URL: `http://127.0.0.1:1789/v1`, API Key: `dev-local-key` |
| **Continue (VS Code)** | `config.json`: `{"models": [{"provider": "openai", "apiBase": "http://127.0.0.1:1789/v1", "apiKey": "dev-local-key", "model": "smart-fast"}]}` |
| **Cline (VS Code)** | Settings → API Provider → OpenAI Compatible → set Base URL dan API Key |
| **RooCode** | Settings → OpenAI Compatible → `http://127.0.0.1:1789/v1` |
| **Cursor** | Settings → Models → OpenAI API Key: `dev-local-key`, OpenAI Base URL: `http://127.0.0.1:1789/v1` |
| **OpenAI Python SDK** | `openai.base_url = "http://127.0.0.1:1789/v1"` + `openai.api_key = "dev-local-key"` |
| **OpenAI JS SDK** | `new OpenAI({ baseURL: "http://127.0.0.1:1789/v1", apiKey: "dev-local-key" })` |

---

## Integrasi Bahasa Pemrograman

### Python

```python
import requests

response = requests.post(
    "http://127.0.0.1:1789/v1/chat/completions",
    headers={
        "Authorization": "Bearer dev-local-key",
        "Content-Type": "application/json",
    },
    json={
        "model": "smart-pro",
        "messages": [
            {"role": "user", "content": "Buat contoh FastAPI endpoint"}
        ],
    },
)

data = response.json()
print(data["choices"][0]["message"]["content"])
```

Dengan OpenAI Python SDK v1.x:

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://127.0.0.1:1789/v1",
    api_key="dev-local-key",
)

response = client.chat.completions.create(
    model="smart-pro",
    messages=[{"role": "user", "content": "Halo"}],
)

print(response.choices[0].message.content)
```

### JavaScript / TypeScript

```javascript
const response = await fetch("http://127.0.0.1:1789/v1/chat/completions", {
  method: "POST",
  headers: {
    "Authorization": "Bearer dev-local-key",
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    model: "smart-pro",
    messages: [
      { role: "user", content: "Buat contoh Express route" },
    ],
  }),
});

const data = await response.json();
console.log(data.choices[0].message.content);
```

Dengan OpenAI Node SDK:

```javascript
import OpenAI from "openai";

const client = new OpenAI({
  baseURL: "http://127.0.0.1:1789/v1",
  apiKey: "dev-local-key",
});

const response = await client.chat.completions.create({
  model: "smart-pro",
  messages: [{ role: "user", content: "Halo" }],
});

console.log(response.choices[0].message.content);
```

### cURL

```bash
curl -X POST http://127.0.0.1:1789/v1/chat/completions \
  -H "Authorization: Bearer dev-local-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "smart-pro",
    "messages": [
      { "role": "user", "content": "Halo" }
    ]
  }' | jq .
```

---

## Troubleshooting

### 1. Server Tidak Bisa Start

**Gejala:** `cargo run` error atau binary langsung exit.

**Penyebab & Solusi:**

```
Error: failed to read config at config.yaml: No such file or directory
```
→ Buat file config: `cp config.example.yaml config.yaml`

```
Error: config has no models
```
→ Pastikan file config memiliki setidaknya satu model di bagian `models`.

```
Error: provider 'openrouter' expects missing environment variable 'OPENROUTER_API_KEY'
```
→ Set environment variable: `export OPENROUTER_API_KEY="sk-or-v1-xxxx"`. Atau set `enabled: false` untuk provider tersebut di config.

```
Error: failed to build upstream HTTP client
```
→ Cek koneksi internet. Atau jika offline, nonaktifkan provider yang butuh koneksi.

### 2. Error 401 Unauthorized

```json
{ "error": { "message": "missing or invalid gateway bearer token", ... } }
```

**Solusi:**

```bash
# Tambahkan header Authorization
curl -H "Authorization: Bearer dev-local-key" http://127.0.0.1:1789/v1/models
```

Atau nonaktifkan auth di config:
```yaml
server:
  gateway_api_keys: []   # kosong = no auth required
```

### 3. Error 400 Unknown Model

```json
{ "error": { "message": "unknown model alias 'gpt-4'", ... } }
```

**Solusi:**

```bash
# Cek daftar model yang tersedia
curl -H "Authorization: Bearer dev-local-key" http://127.0.0.1:1789/v1/models

# Gunakan model alias yang benar (dari field "id" di response)
```

### 4. Provider Gagal / 502

```json
{ "error": { "message": "all provider attempts failed: openrouter returned HTTP 401: ...", ... } }
```

**Penyebab:**
- API key provider salah atau expired → `export OPENROUTER_API_KEY="key_baru"`
- Provider sedang down → cek status provider
- Network error (firewall, proxy) → cek koneksi
- Model name tidak dikenal provider → cek nama model di dashboard provider

### 5. Config Reload Gagal

**Gejala:** Warning log "config reload failed" tapi server tetap jalan.

**Penyebab:**
- YAML syntax error → cek validasi YAML (gunakan `yamllint config.yaml`)
- Model reference provider yang tidak ada → cek nama provider
- Tidak ada models → tambahkan minimal satu model

**Cek error spesifik:**
```bash
# Manual reload untuk lihat error detail
curl -X POST http://127.0.0.1:1789/v1/admin/reload
```

---

## Menjalankan Sebagai Service (Linux)

Agar NusaRoute berjalan otomatis saat boot dan restart jika crash:

```bash
# 1. Buat systemd service
sudo tee /etc/systemd/system/nusaroute.service << 'EOF'
[Unit]
Description=NusaRoute AI Gateway
After=network.target

[Service]
Type=simple
User=your-user
WorkingDirectory=/opt/nusaroute
ExecStart=/usr/local/bin/nusaroute-ai-gateway
Restart=on-failure
RestartSec=5
Environment=NUSAROUTE_CONFIG=/opt/nusaroute/config.yaml
Environment=OPENROUTER_API_KEY=sk-or-v1-xxxx
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# 2. Salin binary dan config
sudo cp target/release/nusaroute-ai-gateway /usr/local/bin/
sudo mkdir -p /opt/nusaroute
sudo cp config.yaml /opt/nusaroute/

# 3. Aktifkan dan start
sudo systemctl daemon-reload
sudo systemctl enable nusaroute
sudo systemctl start nusaroute

# 4. Cek status
sudo systemctl status nusaroute
```

---

## Production Deployment

Untuk penggunaan produksi, rekomendasi:

1. **Reverse proxy (HTTPS):** Letakkan NusaRoute di belakang Nginx atau Caddy untuk TLS termination.

```
Nginx → https://gateway.example.com → http://127.0.0.1:1789
```

2. **Non-root user:** Jangan jalankan sebagai root. Buat user khusus.

3. **Firewall:** Blokir akses ke port 1789 dari luar. Hanya reverse proxy yang boleh akses.

4. **API keys di env:** Jangan hardcode API key di file atau systemd unit yang readable public.

5. **Resource limits:** Di systemd, tambah `MemoryMax=100M` dan `CPUQuota=50%`.

6. **Monitoring:** Integrasikan `/metrics` dengan Prometheus + Grafana.

7. **Logging:** Log JSON siap untuk dikumpulkan ke systemd journal, Loki, atau Datadog.

---

## Security Checklist

- [ ] Ganti `dev-local-key` dengan token unik sebelum expose ke jaringan
- [ ] Pastikan `config.yaml` tidak masuk Git (tambahkan ke `.gitignore`)
- [ ] API key provider hanya di environment variable, tidak di file
- [ ] Gunakan HTTPS via reverse proxy (Caddy, Nginx) untuk public deployment
- [ ] Blokir endpoint `/v1/admin/reload` dari akses publik via firewall / reverse proxy
- [ ] Jalankan dengan non-root user
- [ ] Update Rust toolchain dan dependensi secara berkala (`cargo update`)
- [ ] Batasi request timeout sesuai kebutuhan (default 45 detik)

---

## Development

### Struktur Project

```
nusaroute/
├── src/
│   ├── main.rs        # Entrypoint: server startup, config watcher, reload API
│   ├── config.rs      # Loader dan validator konfigurasi YAML
│   ├── gateway.rs     # HTTP handlers: chat, models, metrics, routing engine
│   └── error.rs       # Tipe error API dengan mapping HTTP status codes
├── docs/
│   ├── API.md         # Dokumentasi API lengkap
│   ├── ARCHITECTURE.md# Keputusan arsitektur dan teknis
│   └── RESEARCH.md    # Catatan riset 2026
├── config.example.yaml # Contoh konfigurasi
├── config.smoke.yaml   # Config untuk smoke test
├── Cargo.toml         # Dependensi dan metadata
└── README.md          # Dokumentasi ini
```

### Perintah Development

```bash
# Format code (wajib sebelum commit)
cargo fmt

# Cek error kompilasi (cepat, tanpa build binary)
cargo check

# Build development (debug, cepat untuk development)
cargo build

# Build production (optimized, untuk deployment)
cargo build --release

# Jalankan test
cargo test

# Linting
cargo clippy

# Update dependensi
cargo update

# Cek binary size
du -sh target/release/nusaroute-ai-gateway
```

---

## Roadmap

### ✅ Selesai
- [x] OpenAI-compatible chat completions
- [x] Provider registry via YAML
- [x] Model aliases dengan routing & fallback
- [x] Streaming proxy
- [x] Bearer token auth
- [x] Prometheus metrics
- [x] Hot-reload config (otomatis + via API)

### 🔜 Sedang Dikerjakan
- [ ] `/v1/responses` API compatibility (OpenAI Responses API)
- [ ] Provider adapter native untuk Anthropic (Claude) dan Google (Gemini)

### 📋 Rencana
- [ ] Rate limiting per API key
- [ ] Usage tracking & cost estimation
- [ ] SQLite / PostgreSQL untuk usage log
- [ ] Admin dashboard web
- [ ] OpenAPI spec + interaktif docs (Scalar)
- [ ] OpenTelemetry tracing
- [ ] Health probe background untuk provider

---

## Dokumentasi Tambahan

- [API Usage Guide (Bahasa)](docs/API.md) — Contoh lengkap semua endpoint, parameter, dan integrasi Python/JavaScript
- [Architecture & Technical Decisions](docs/ARCHITECTURE.md) — Penjelasan arsitektur, routing engine, dan keputusan teknis
- [Research Notes 2026](docs/RESEARCH.md) — Perbandingan dengan Cloudflare AI Gateway, LiteLLM, OpenRouter, dan rekomendasi stack
