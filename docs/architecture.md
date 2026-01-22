# Aura LLM Gateway Architecture

## System Overview

Aura is a high-performance LLM gateway built in Rust that provides a unified API for multiple LLM providers. It implements the [Open Responses API](https://openresponses.org) specification for agentic workflows.

```
                                    ┌─────────────────────────────────────────────────┐
                                    │                 Aura Gateway                     │
                                    │                                                  │
┌──────────────┐                    │  ┌─────────────┐    ┌─────────────────────────┐ │
│              │   HTTP/SSE         │  │             │    │    Provider Registry    │ │
│   Clients    │◄──────────────────►│  │   Router    │───►│  ┌─────┐ ┌─────┐ ┌───┐ │ │
│  (Chat UI,   │   Open Responses   │  │   (Axum)    │    │  │OpenAI│ │Anthro│ │GCP│ │ │
│   Agents)    │   API              │  │             │    │  └─────┘ └─────┘ └───┘ │ │
│              │                    │  └─────────────┘    └─────────────────────────┘ │
└──────────────┘                    │         │                      │               │
                                    │         ▼                      ▼               │
                                    │  ┌─────────────┐    ┌─────────────────────────┐ │
                                    │  │  Middleware │    │    Cost Calculator      │ │
                                    │  │  - Tracing  │    │  - Per-model pricing    │ │
                                    │  │  - Metrics  │    │  - Token accounting     │ │
                                    │  │  - Auth     │    │  - Usage tracking       │ │
                                    │  └─────────────┘    └─────────────────────────┘ │
                                    │         │                      │               │
                                    │         ▼                      ▼               │
                                    │  ┌─────────────────────────────────────────────┐│
                                    │  │              Request Logger                 ││
                                    │  │         (PostgreSQL - Optional)             ││
                                    │  └─────────────────────────────────────────────┘│
                                    └─────────────────────────────────────────────────┘
                                                         │
                                                         ▼
                                    ┌─────────────────────────────────────────────────┐
                                    │              LLM Provider APIs                  │
                                    │  ┌─────────┐  ┌─────────────┐  ┌─────────────┐ │
                                    │  │ OpenAI  │  │  Anthropic  │  │   Google    │ │
                                    │  │   API   │  │     API     │  │ Gemini API  │ │
                                    │  └─────────┘  └─────────────┘  └─────────────┘ │
                                    └─────────────────────────────────────────────────┘
```

## Crate Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              Workspace Structure                                │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────┐                                                           │
│  │   aura-proxy    │  Binary crate - Main server entry point                   │
│  │   (bin)         │  - Axum routes & handlers                                 │
│  └────────┬────────┘  - Middleware stack                                       │
│           │           - Server configuration                                    │
│           │                                                                     │
│           ▼                                                                     │
│  ┌─────────────────┐                                                           │
│  │   aura-core     │  Library crate - Core business logic                      │
│  │   (lib)         │  - Provider implementations                               │
│  └────────┬────────┘  - Cost calculation                                       │
│           │           - HTTP client utilities                                   │
│           │           - Configuration management                                │
│           │                                                                     │
│           ├──────────────────────┐                                             │
│           ▼                      ▼                                             │
│  ┌─────────────────┐    ┌─────────────────┐                                    │
│  │   aura-types    │    │    aura-db      │                                    │
│  │   (lib)         │    │    (lib)        │                                    │
│  └─────────────────┘    └─────────────────┘                                    │
│  - Open Responses API    - SQLx/PostgreSQL                                      │
│    type definitions      - Connection pooling                                   │
│  - Request/Response      - Request logging                                      │
│    structures            - Pricing storage                                      │
│  - Stream events                                                                │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Request Flow

### Non-Streaming Request

```
┌────────┐     ┌─────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│ Client │     │  Axum   │     │ Provider │     │  Cost    │     │ Database │
│        │     │ Router  │     │ Registry │     │Calculator│     │ (opt)    │
└───┬────┘     └────┬────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
    │               │               │                │                │
    │ POST /v1/responses            │                │                │
    │──────────────►│               │                │                │
    │               │               │                │                │
    │               │ get_provider()│                │                │
    │               │──────────────►│                │                │
    │               │               │                │                │
    │               │◄──────────────│                │                │
    │               │  Arc<Provider>│                │                │
    │               │               │                │                │
    │               │ complete(req) │                │                │
    │               │───────────────┼───────────────►│                │
    │               │               │    LLM API     │                │
    │               │               │◄───────────────│                │
    │               │◄──────────────┼────────────────│                │
    │               │   Response    │                │                │
    │               │               │                │                │
    │               │ calculate_cost()               │                │
    │               │───────────────────────────────►│                │
    │               │◄───────────────────────────────│                │
    │               │   cost_usd                     │                │
    │               │               │                │                │
    │               │ enrich_response()              │                │
    │               │──────────────►│                │                │
    │               │               │                │                │
    │               │               │ log_request()  │                │
    │               │───────────────┼────────────────┼───────────────►│
    │               │               │                │    (async)     │
    │               │               │                │                │
    │◄──────────────│               │                │                │
    │ JSON Response │               │                │                │
    │ + cost + metadata             │                │                │
    │               │               │                │                │
```

### Streaming Request

```
┌────────┐     ┌─────────┐     ┌──────────┐     ┌──────────────┐
│ Client │     │  Axum   │     │ Provider │     │   LLM API    │
│        │     │ Router  │     │          │     │              │
└───┬────┘     └────┬────┘     └────┬─────┘     └──────┬───────┘
    │               │               │                  │
    │ POST /v1/responses            │                  │
    │ stream: true  │               │                  │
    │──────────────►│               │                  │
    │               │               │                  │
    │               │ complete_stream()                │
    │               │──────────────►│                  │
    │               │               │                  │
    │               │               │ SSE Connection   │
    │               │               │─────────────────►│
    │               │               │                  │
    │◄──────────────┼───────────────┼──────────────────│
    │ SSE: response.in_progress     │                  │
    │               │               │                  │
    │◄──────────────┼───────────────┼──────────────────│
    │ SSE: response.output_item.added                  │
    │               │               │                  │
    │◄──────────────┼───────────────┼──────────────────│
    │ SSE: response.output_text.delta (repeated)       │
    │               │               │                  │
    │◄──────────────┼───────────────┼──────────────────│
    │ SSE: response.completed       │                  │
    │ (enriched with cost + metadata)                  │
    │               │               │                  │
```

## Component Details

### Provider System

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn models(&self) -> &[&str];
    fn supports_model(&self, model: &str) -> bool;

    async fn complete(&self, request: CreateResponseRequest)
        -> Result<Response, ProviderError>;

    async fn complete_stream(&self, request: CreateResponseRequest)
        -> Result<EventStream, ProviderError>;
}
```

### Response Enrichment

Every response is enriched with Aura-specific metadata:

```json
{
  "id": "resp_abc123",
  "model": "gpt-4o",
  "status": "completed",
  "output": [...],
  "usage": {
    "input_tokens": 100,
    "output_tokens": 50,
    "cost_usd": 0.00075
  },
  "metadata": {
    "aura": {
      "request_id": "aura_550e8400-e29b-41d4-a716-446655440000",
      "model": "gpt-4o",
      "provider": "openai",
      "gateway_version": "0.1.7",
      "latency_ms": 523,
      "agentic": {
        "output_items_count": 2,
        "has_tool_calls": true,
        "tool_calls_count": 1,
        "tools_used": ["web_search"],
        "requires_action": false
      }
    }
  }
}
```

### Database Integration (Optional)

```
┌─────────────────────────────────────────────────────────────┐
│                      PostgreSQL                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────┐    ┌─────────────────────────────────┐│
│  │   providers     │    │        model_pricing            ││
│  ├─────────────────┤    ├─────────────────────────────────┤│
│  │ id              │    │ id                              ││
│  │ name            │◄───│ provider_id                     ││
│  │ display_name    │    │ model_id                        ││
│  │ api_base_url    │    │ input_per_million               ││
│  │ is_enabled      │    │ output_per_million              ││
│  └─────────────────┘    │ cached_input_per_million        ││
│                         │ effective_from                  ││
│                         │ effective_until                 ││
│                         └─────────────────────────────────┘│
│                                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                    request_logs                         ││
│  ├─────────────────────────────────────────────────────────┤│
│  │ id, response_id, provider_name, model_id                ││
│  │ input_tokens, output_tokens, cost_usd, latency_ms       ││
│  │ status, error_code, error_message, metadata             ││
│  │ created_at                                              ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow Summary

```
                    ┌─────────────────────────────────────────┐
                    │           Client Request                │
                    │  POST /v1/responses                     │
                    │  {model: "gpt-4o", input: [...]}        │
                    └──────────────────┬──────────────────────┘
                                       │
                                       ▼
                    ┌─────────────────────────────────────────┐
                    │         1. Route Matching               │
                    │    Axum extracts request body           │
                    └──────────────────┬──────────────────────┘
                                       │
                                       ▼
                    ┌─────────────────────────────────────────┐
                    │      2. Provider Resolution             │
                    │  model → provider mapping               │
                    │  "gpt-4o" → OpenAIProvider              │
                    └──────────────────┬──────────────────────┘
                                       │
                                       ▼
                    ┌─────────────────────────────────────────┐
                    │      3. Request Transformation          │
                    │  Open Responses → Provider format       │
                    │  Add auth headers, format messages      │
                    └──────────────────┬──────────────────────┘
                                       │
                                       ▼
                    ┌─────────────────────────────────────────┐
                    │      4. Provider API Call               │
                    │  HTTP request to OpenAI/Anthropic/etc   │
                    └──────────────────┬──────────────────────┘
                                       │
                                       ▼
                    ┌─────────────────────────────────────────┐
                    │      5. Response Transformation         │
                    │  Provider format → Open Responses       │
                    └──────────────────┬──────────────────────┘
                                       │
                                       ▼
                    ┌─────────────────────────────────────────┐
                    │      6. Response Enrichment             │
                    │  Add: cost_usd, request_id, provider    │
                    │  Add: latency_ms, agentic metadata      │
                    └──────────────────┬──────────────────────┘
                                       │
                                       ▼
                    ┌─────────────────────────────────────────┐
                    │      7. Request Logging (async)         │
                    │  Background task → PostgreSQL           │
                    └──────────────────┬──────────────────────┘
                                       │
                                       ▼
                    ┌─────────────────────────────────────────┐
                    │         Client Response                 │
                    │  {id, output, usage, metadata}          │
                    └─────────────────────────────────────────┘
```

## Configuration

```yaml
# aura.yaml
server:
  host: "0.0.0.0"
  port: 8080

providers:
  openai:
    api_key: ${OPENAI_API_KEY}
  anthropic:
    api_key: ${ANTHROPIC_API_KEY}
  google:
    api_key: ${GOOGLE_API_KEY}

database:
  url: ${DATABASE_URL}  # Optional
  max_connections: 10
```

## Future Architecture (Planned)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           Future Enhancements                                │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────┐    ┌────────────────┐    ┌────────────────┐            │
│  │   Rate Limiter │    │   Cache Layer  │    │  Load Balancer │            │
│  │    (Redis)     │    │    (Redis)     │    │  (Multi-node)  │            │
│  └────────────────┘    └────────────────┘    └────────────────┘            │
│                                                                              │
│  ┌────────────────┐    ┌────────────────┐    ┌────────────────┐            │
│  │ Pricing Scraper│    │   Webhooks     │    │    Admin UI    │            │
│  │  (Cron Job)    │    │  (Callbacks)   │    │  (Dashboard)   │            │
│  └────────────────┘    └────────────────┘    └────────────────┘            │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```
