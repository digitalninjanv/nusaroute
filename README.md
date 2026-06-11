# NusaRoute AI Gateway

Low-latency self-hosted AI gateway for routing multiple AI providers through one OpenAI-compatible endpoint.

NusaRoute lets you expose one local or server API endpoint, define model aliases such as `smart-fast` and `smart-pro`, then route requests to OpenAI-compatible providers like OpenAI, OpenRouter, vLLM, LiteLLM upstreams, local LLM servers, or custom providers.

> Project status: early MVP. The current release focuses on OpenAI-compatible chat completions, provider routing, fallback, streaming proxy, bearer auth, and metrics.

## Features

- OpenAI-compatible API endpoint.
- `GET /v1/models`.
- `POST /v1/chat/completions`.
- Provider registry through YAML config.
- Model aliases, so clients use stable names like `smart-fast`.
- Routing strategies:
  - `lowest_latency`
  - `priority`
- Fallback across provider/model candidates.
- Streaming response proxy.
- Gateway bearer token authentication.
- Lightweight Prometheus-style metrics.
- Rust + Axum async runtime for low gateway overhead.

## Use Cases

- Use one API endpoint for multiple AI providers.
- Connect AI coding tools to a local OpenAI-compatible gateway.
- Switch providers without changing client configuration.
- Route fast tasks to cheap/low-latency models and harder tasks to stronger models.
- Add fallback when a provider is down, rate-limited, or slow.

## Quickstart

Clone and run:

```bash
cp config.example.yaml config.yaml
export NUSAROUTE_CONFIG=config.yaml
export OPENROUTER_API_KEY="your_openrouter_key"
cargo run
```

By default the server listens on:

```txt
http://127.0.0.1:1789
```

Health check:

```bash
curl http://127.0.0.1:1789/healthz
```

List model aliases:

```bash
curl -H "Authorization: Bearer dev-local-key" \
  http://127.0.0.1:1789/v1/models
```

Chat completion:

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

Streaming:

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

## Client Configuration

For OpenAI-compatible tools and SDKs:

```txt
Base URL: http://127.0.0.1:1789/v1
API Key: dev-local-key
Model: smart-pro
```

Use `/v1/models` to see the aliases available in your config.

## Configuration

NusaRoute is configured with YAML.

Minimal provider example:

```yaml
providers:
  openrouter:
    type: "openai-compatible"
    base_url: "https://openrouter.ai/api/v1"
    api_key_env: "OPENROUTER_API_KEY"
    enabled: true
    priority: 10
```

Model alias example:

```yaml
models:
  smart-pro:
    description: "Higher-quality route with fallback"
    strategy: "priority"
    fallback: true
    candidates:
      - provider: "openrouter"
        model: "nvidia/nemotron-3-ultra-550b-a55b:free"
```

Provider API keys are read from environment variables. Do not put provider secrets directly in YAML.

## Add a Provider

Any OpenAI-compatible provider can be added without code changes:

```yaml
providers:
  my_provider:
    type: "openai-compatible"
    base_url: "https://provider.example.com/v1"
    api_key_env: "MY_PROVIDER_API_KEY"
    enabled: true
    priority: 5
```

Then set the API key:

```bash
export MY_PROVIDER_API_KEY="your_provider_key"
```

Add it to a model alias:

```yaml
models:
  coding-fast:
    description: "Fast model for coding"
    strategy: "lowest_latency"
    fallback: true
    candidates:
      - provider: "my_provider"
        model: "coder-fast"
      - provider: "openrouter"
        model: "openai/gpt-5.5-mini"
```

## Metrics

```bash
curl http://127.0.0.1:1789/metrics
```

Example:

```txt
nusaroute_provider_requests_total{provider="openrouter"} 12
nusaroute_provider_failures_total{provider="openrouter"} 1
nusaroute_provider_latency_ewma_ms{provider="openrouter"} 923.400
```

## Security Notes

- Change the default gateway key before exposing the server outside your machine.
- Keep `config.yaml` local and commit only `config.example.yaml`.
- Store provider keys in environment variables, not in the repository.
- Put the gateway behind HTTPS and a trusted reverse proxy for public deployments.
- Treat all client input as untrusted.

## Documentation

- [API usage](docs/API.md)
- [Architecture](docs/ARCHITECTURE.md)
- [2026 research notes](docs/RESEARCH.md)

## Roadmap

- `/v1/responses` compatibility.
- Native Anthropic and Gemini adapters.
- Hot reload for config changes.
- Per-key rate limits.
- Usage and cost tracking.
- SQLite/PostgreSQL usage log.
- Admin dashboard.
- OpenAPI spec and hosted API docs UI.
- OpenTelemetry tracing.

## Development

Format:

```bash
cargo fmt
```

Check:

```bash
cargo check
```

Test:

```bash
cargo test
```
