# Dokumentasi API Penggunaan NusaRoute AI Gateway

NusaRoute menyediakan satu endpoint API OpenAI-compatible untuk mengakses banyak provider AI melalui model alias seperti `smart-fast` dan `smart-pro`.

Default base URL:

```txt
http://127.0.0.1:1789
```

Base URL untuk aplikasi atau tool OpenAI-compatible:

```txt
http://127.0.0.1:1789/v1
```

API key default dari `config.example.yaml`:

```txt
dev-local-key
```

## 1. Menjalankan Server

Buat config lokal:

```bash
cp config.example.yaml config.yaml
```

Set environment variable:

```bash
export NUSAROUTE_CONFIG=config.yaml
export OPENROUTER_API_KEY="isi_api_key_openrouter"
```

Jalankan:

```bash
cargo run
```

Jika berhasil, server aktif di:

```txt
http://127.0.0.1:1789
```

## 2. Autentikasi

Endpoint `/v1/*` memakai bearer token:

```http
Authorization: Bearer dev-local-key
```

Token bisa diganti di `config.yaml`:

```yaml
server:
  gateway_api_keys:
    - "token-rahasia-kamu"
```

Setelah config diubah, restart server.

## 3. Health Check

Endpoint ini tidak butuh auth.

```bash
curl http://127.0.0.1:1789/healthz
```

Response:

```json
{
  "status": "ok"
}
```

## 4. Melihat Model Alias

```bash
curl -H "Authorization: Bearer dev-local-key" \
  http://127.0.0.1:1789/v1/models
```

Contoh response:

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

`id` adalah nama model yang dipakai client. Client tidak perlu tahu provider asli di belakangnya.

## 5. Chat Completion

Endpoint:

```txt
POST /v1/chat/completions
```

Contoh request:

```bash
curl http://127.0.0.1:1789/v1/chat/completions \
  -H "Authorization: Bearer dev-local-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "smart-pro",
    "messages": [
      { "role": "user", "content": "kamu siapa" }
    ]
  }'
```

Contoh response:

```json
{
  "id": "gen-xxx",
  "object": "chat.completion",
  "created": 1781140795,
  "model": "nvidia/nemotron-3-ultra-550b-a55b:free",
  "choices": [
    {
      "index": 0,
      "finish_reason": "stop",
      "message": {
        "role": "assistant",
        "content": "Saya adalah asisten AI..."
      }
    }
  ],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 110,
    "total_tokens": 130
  }
}
```

Catatan: response upstream diteruskan apa adanya. Jika provider seperti OpenRouter menambahkan field seperti `provider`, `cost`, atau `reasoning_details`, field itu tetap muncul.

## 6. Streaming

Tambahkan `"stream": true`.

```bash
curl -N http://127.0.0.1:1789/v1/chat/completions \
  -H "Authorization: Bearer dev-local-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "smart-fast",
    "stream": true,
    "messages": [
      { "role": "user", "content": "Tulis ringkasan pendek tentang Rust" }
    ]
  }'
```

Gateway akan meneruskan stream dari provider ke client.

## 7. Format Body Chat

Body mengikuti format OpenAI Chat Completions.

Minimal:

```json
{
  "model": "smart-fast",
  "messages": [
    {
      "role": "user",
      "content": "Halo"
    }
  ]
}
```

Dengan system prompt:

```json
{
  "model": "smart-pro",
  "messages": [
    {
      "role": "system",
      "content": "Jawab singkat dalam bahasa Indonesia."
    },
    {
      "role": "user",
      "content": "Apa itu API gateway?"
    }
  ]
}
```

Dengan parameter tambahan:

```json
{
  "model": "smart-fast",
  "temperature": 0.2,
  "max_tokens": 500,
  "messages": [
    {
      "role": "user",
      "content": "Buat fungsi validasi email di TypeScript"
    }
  ]
}
```

Parameter tambahan diteruskan ke provider upstream.

## 8. Integrasi Dengan Tool OpenAI-Compatible

Gunakan pengaturan berikut:

```txt
Base URL: http://127.0.0.1:1789/v1
API Key: dev-local-key
Model: smart-pro
```

Untuk model cepat:

```txt
Model: smart-fast
```

Untuk tool seperti OpenCode, Continue, Cline, RooCode, Cursor, atau client OpenAI-compatible lain, gunakan base URL di atas dan pilih model alias dari `/v1/models`.

## 9. Menggunakan Dari JavaScript

Contoh dengan `fetch`:

```js
const response = await fetch("http://127.0.0.1:1789/v1/chat/completions", {
  method: "POST",
  headers: {
    "Authorization": "Bearer dev-local-key",
    "Content-Type": "application/json"
  },
  body: JSON.stringify({
    model: "smart-pro",
    messages: [
      { role: "user", content: "Buat contoh express route" }
    ]
  })
});

const data = await response.json();
console.log(data.choices[0].message.content);
```

## 10. Menggunakan Dari Python

Contoh dengan `requests`:

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

## 11. Menambah Provider

Provider baru yang OpenAI-compatible cukup ditambahkan ke `config.yaml`.

Contoh:

```yaml
providers:
  my_provider:
    type: "openai-compatible"
    base_url: "https://provider.example.com/v1"
    api_key_env: "MY_PROVIDER_API_KEY"
    enabled: true
    priority: 5
```

Set API key:

```bash
export MY_PROVIDER_API_KEY="isi_api_key"
```

Lalu tambahkan provider itu ke model alias.

## 12. Menambah Model Alias

Contoh alias baru `coding-fast`:

```yaml
models:
  coding-fast:
    description: "Model cepat untuk coding"
    strategy: "lowest_latency"
    fallback: true
    candidates:
      - provider: "my_provider"
        model: "coder-fast"
      - provider: "openrouter"
        model: "openai/gpt-5.5-mini"
```

Pakai dari API:

```bash
curl http://127.0.0.1:1789/v1/chat/completions \
  -H "Authorization: Bearer dev-local-key" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "coding-fast",
    "messages": [
      { "role": "user", "content": "Review kode ini" }
    ]
  }'
```

## 13. Routing dan Fallback

Strategi yang tersedia:

```yaml
strategy: "lowest_latency"
```

Gateway memilih kandidat dengan latency rata-rata terendah. Jika belum ada data latency, urutan `priority` dipakai.

```yaml
strategy: "priority"
```

Gateway memilih provider berdasarkan angka `priority` terkecil.

Fallback:

```yaml
fallback: true
```

Jika provider pertama gagal, gateway mencoba kandidat berikutnya.

## 14. Metrics

```bash
curl http://127.0.0.1:1789/metrics
```

Contoh output:

```txt
# TYPE nusaroute_provider_requests_total counter
nusaroute_provider_requests_total{provider="openrouter"} 12
nusaroute_provider_failures_total{provider="openrouter"} 1
nusaroute_provider_latency_ewma_ms{provider="openrouter"} 923.400
```

Metrics ini bisa dipakai untuk melihat provider mana yang sering gagal atau lambat.

## 15. Error

Format error gateway:

```json
{
  "error": {
    "message": "unknown model alias 'x'",
    "type": "nusaroute_gateway_error"
  }
}
```

Status umum:

| Status | Penyebab |
| --- | --- |
| `401` | Token salah atau header `Authorization` tidak ada |
| `400` | Model alias tidak ada, body salah, atau tidak ada provider aktif |
| `502` | Semua upstream provider gagal |

## 16. Troubleshooting

### `missing or invalid gateway bearer token`

Tambahkan header:

```bash
-H "Authorization: Bearer dev-local-key"
```

### `unknown model alias`

Cek daftar model:

```bash
curl -H "Authorization: Bearer dev-local-key" \
  http://127.0.0.1:1789/v1/models
```

Pastikan `model` di request sama dengan alias di config.

### Provider tidak aktif

Cek `enabled`:

```yaml
providers:
  openrouter:
    enabled: true
```

### API key provider belum diset

Jika provider aktif, environment variable wajib ada.

```bash
export OPENROUTER_API_KEY="isi_api_key"
```

### Server tidak jalan

Cek:

```bash
curl http://127.0.0.1:1789/healthz
```

Jika gagal, jalankan ulang:

```bash
export NUSAROUTE_CONFIG=config.yaml
cargo run
```
