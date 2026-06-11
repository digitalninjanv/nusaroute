# Catatan Riset 2026

Riset dilakukan pada 2026-06-11.

## Temuan

- Cloudflare AI Gateway menekankan kontrol dan observability untuk aplikasi AI: analytics, logging, caching, rate limiting, retry, model fallback, dan banyak provider. Ini menguatkan bahwa fitur wajib gateway modern bukan hanya proxy, tetapi routing, kontrol biaya, dan metrik. Sumber: https://developers.cloudflare.com/ai-gateway/
- OpenRouter menyediakan unified API untuk ratusan model melalui satu endpoint, dengan fallback dan pemilihan opsi cost-effective. Ini validasi pola model alias + routing provider. Sumber: https://openrouter.ai/docs/quickstart
- LiteLLM Proxy mendukung format OpenAI ChatCompletions/Completions untuk 100+ LLM, cost tracking, auth, spend tracking, budgets, dan load balancing. Ini validasi fitur provider registry, fallback, rate limit, dan config-driven model list. Sumber: https://docs.litellm.ai/docs/proxy/quick_start
- Axum 0.8.9 adalah library routing/request handling Rust yang fokus pada ergonomics dan modularity, memakai ekosistem Tower untuk timeout, tracing, compression, authorization, dan middleware lain. Ini cocok untuk gateway HTTP ringan. Sumber: https://docs.rs/axum/latest/axum/
- OpenAI API docs terbaru masih mencantumkan Chat dan Responses API. Untuk kompatibilitas tooling, MVP memulai dari `/v1/chat/completions`; `/v1/responses` masuk roadmap. Sumber: https://developers.openai.com/api/reference/resources/chat dan https://developers.openai.com/api/reference/resources/responses/methods/create

## Rekomendasi Stack

Core production:

- Rust 2024
- Axum 0.8
- Tokio
- reqwest/hyper
- YAML config
- SQLite WAL atau PostgreSQL untuk usage log
- Prometheus/OpenTelemetry untuk observability

Kenapa bukan Python untuk core:

- Gateway adalah proxy I/O-bound yang harus menjaga overhead rendah.
- Rust memberi single binary, memory kecil, dan async runtime matang.
- Python tetap cocok untuk plugin/analytics, tetapi bukan jalur hot path utama.

Alternatif bila ingin MVP paling cepat dibuat:

- Bun + Hono + TypeScript untuk prototyping cepat dan edge runtime.
- Go + Chi/Fiber untuk gateway sederhana dengan learning curve lebih rendah.

Keputusan project ini: Rust + Axum untuk jalur hot path, dashboard bisa ditambahkan terpisah dengan React/Vite.
