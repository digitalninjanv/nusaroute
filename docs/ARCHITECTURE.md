# Arsitektur

```txt
Client / AI coding tool
        |
        v
NusaRoute AI Gateway
        |
        |-- auth bearer token
        |-- OpenAI-compatible request parser
        |-- model alias resolver
        |-- routing engine
        |-- provider adapter
        |-- fallback loop
        |-- streaming response proxy
        |-- metrics collector
        |
        v
OpenAI / OpenRouter / vLLM / custom OpenAI-compatible provider
```

## Keputusan Teknis

- Rust + Axum dipakai untuk core gateway karena server ini adalah proxy I/O-bound yang butuh concurrency tinggi, memory rendah, dan response streaming.
- `reqwest` dipakai dengan connection pooling, `rustls`, `tcp_nodelay`, dan timeout per request.
- Config YAML dipakai sebagai control plane sederhana agar provider dan model bisa ditambah tanpa compile ulang.
- Statistik latency disimpan in-memory sebagai EWMA. Ini cukup untuk single-node MVP dan tidak menambah dependency Redis.
- Fallback dilakukan hanya saat upstream error/non-2xx atau request gagal.

## Routing

`lowest_latency`:

- Kandidat dengan latency EWMA paling rendah dipilih lebih dulu.
- Provider yang belum punya data memakai urutan `priority`.

`priority`:

- Kandidat dipilih berdasarkan `providers.*.priority` terkecil.

## Menambah Adapter Provider

Struktur sekarang sengaja memulai dari `openai-compatible`. Langkah berikutnya untuk Anthropic/Gemini native:

1. Tambah enum di `ProviderKind`.
2. Buat fungsi adapter request/response native ke format internal.
3. Panggil adapter dari `chat_completions`.
4. Tambah test contract untuk payload dan fallback.

## Roadmap Produksi

- Hot reload config dengan validasi atomic.
- Rate limit per API key.
- Budget/cost tracking per key/model.
- SQLite WAL untuk usage log lokal.
- PostgreSQL untuk mode team.
- Admin dashboard React/Vite.
- OpenAPI JSON + Scalar UI.
- Provider health probe background.
- Optional Redis untuk cluster multi-node.
- Policy engine untuk allow/deny model, max tokens, dan metadata route.
