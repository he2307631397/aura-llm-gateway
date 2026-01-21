# Provider Mapping Reference

This document describes how each LLM provider's native API maps to the Open Responses API types defined in `aura-types`.

## Overview

The Aura LLM Gateway translates between provider-specific formats and the Open Responses API. This mapping enables consistent behavior across providers while preserving provider-specific features.

## Type Mapping Summary

| Open Responses Type | OpenAI | Anthropic (Claude) | Google (Gemini) |
|---------------------|--------|-------------------|-----------------|
| `Role::User` | `user` | `user` | `user` |
| `Role::Assistant` | `assistant` | `assistant` | `model` |
| `Role::System` | `system` | System in first message | `system_instruction` |
| `Role::Tool` | `tool` | `tool_result` | `function_response` |
| `Item::Message` | `message` | `text` content block | `text` part |
| `Item::FunctionCall` | `function_call` / `tool_calls` | `tool_use` | `function_call` |
| `Item::FunctionCallOutput` | `function` / `tool` message | `tool_result` | `function_response` |
| `Item::Reasoning` | N/A (internal) | `thinking` block | N/A |

---

## OpenAI Mapping

### Request Transformation

```
CreateResponseRequest → OpenAI Chat Completion Request
─────────────────────────────────────────────────────

Open Responses                    OpenAI
──────────────                    ──────
model                          → model
input                          → messages (transformed)
instructions                   → system message (prepended)
max_output_tokens              → max_tokens
temperature                    → temperature
top_p                          → top_p
stream                         → stream
tools                          → tools (format preserved)
tool_choice                    → tool_choice
previous_response_id           → (context restored from storage)
```

### Input Item Transformation

```rust
// Open Responses InputItem → OpenAI Message
InputItem::Message { role, content }
    → { "role": role, "content": content }

// For system instructions
instructions: "Be helpful"
    → { "role": "system", "content": "Be helpful" }

// Function call output
InputItem::FunctionCallOutput { call_id, output }
    → { "role": "tool", "tool_call_id": call_id, "content": output }
```

### Response Transformation

```
OpenAI Chat Completion Response → Response
────────────────────────────────────────────

OpenAI                           Open Responses
──────                           ──────────────
id                            → id (prefixed with "resp_")
model                         → model
created                       → created_at
choices[0].message.content    → output[0] as Item::Message
choices[0].message.tool_calls → output[n] as Item::FunctionCall
choices[0].finish_reason      → status
  - "stop"                    → ResponseStatus::Completed
  - "length"                  → ResponseStatus::Incomplete (MaxTokens)
  - "tool_calls"              → ResponseStatus::Completed
  - "content_filter"          → ResponseStatus::Incomplete (ContentFilter)
usage.prompt_tokens           → usage.input_tokens
usage.completion_tokens       → usage.output_tokens
```

### Streaming Events Transformation

```
OpenAI SSE                       Open Responses SSE
──────────                       ──────────────────
(initial)                     → response.created
                              → response.in_progress
                              → response.output_item.added
delta.content                 → response.output_text.delta
delta.tool_calls[].function   → response.function_call_arguments.delta
choices[].finish_reason       → response.completed / response.incomplete
[DONE]                        → (end of stream)
```

---

## Anthropic (Claude) Mapping

### Request Transformation

```
CreateResponseRequest → Anthropic Messages Request
─────────────────────────────────────────────────

Open Responses                    Anthropic
──────────────                    ─────────
model                          → model
input                          → messages (transformed, system extracted)
instructions                   → system (top-level field)
max_output_tokens              → max_tokens (required)
temperature                    → temperature
top_p                          → top_p
stream                         → stream
tools                          → tools (format transformed)
tool_choice                    → tool_choice
```

### Message Format Differences

Claude uses a different structure for messages:

```rust
// Open Responses → Anthropic
InputItem::Message { role: User, content: "Hello" }
    → { "role": "user", "content": "Hello" }

// System message handling (Claude requires system at top level)
instructions: "Be helpful"
    → { "system": "Be helpful", "messages": [...] }

// Multi-part content
ContentPart::Text { text }
    → { "type": "text", "text": text }

ContentPart::Image { data, media_type }
    → { "type": "image", "source": { "type": "base64", "media_type": media_type, "data": data } }

// Tool use (Claude calls it tool_use)
InputItem::FunctionCallOutput { call_id, output }
    → { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": call_id, "content": output }] }
```

### Response Transformation

```
Anthropic Messages Response → Response
──────────────────────────────────────

Anthropic                        Open Responses
─────────                        ──────────────
id                            → id (prefixed with "resp_")
model                         → model
content[].type == "text"      → output[n] as Item::Message
content[].type == "tool_use"  → output[n] as Item::FunctionCall
content[].type == "thinking"  → output[n] as Item::Reasoning
stop_reason                   → status
  - "end_turn"                → ResponseStatus::Completed
  - "max_tokens"              → ResponseStatus::Incomplete (MaxTokens)
  - "tool_use"                → ResponseStatus::Completed
usage.input_tokens            → usage.input_tokens
usage.output_tokens           → usage.output_tokens
```

### Tool Definition Transformation

```rust
// Open Responses Tool → Anthropic Tool
Tool::Function { function: FunctionDefinition { name, description, parameters } }
    → { "name": name, "description": description, "input_schema": parameters }
```

### Streaming Events Transformation

```
Anthropic SSE                    Open Responses SSE
─────────────                    ──────────────────
message_start                 → response.created, response.in_progress
content_block_start           → response.output_item.added / content_part.added
content_block_delta (text)    → response.output_text.delta
content_block_delta (tool)    → response.function_call_arguments.delta
content_block_delta (think)   → response.reasoning.delta
content_block_stop            → response.output_text.done / content_part.done
message_stop                  → response.completed
error                         → response.failed
```

---

## Google (Gemini) Mapping

### Request Transformation

```
CreateResponseRequest → Gemini GenerateContent Request
──────────────────────────────────────────────────────

Open Responses                    Gemini
──────────────                    ──────
model                          → model (in URL path)
input                          → contents (transformed)
instructions                   → system_instruction
max_output_tokens              → generationConfig.maxOutputTokens
temperature                    → generationConfig.temperature
top_p                          → generationConfig.topP
stream                         → (different endpoint: streamGenerateContent)
tools                          → tools (format transformed)
tool_choice                    → toolConfig
```

### Role Mapping

Gemini uses `user` and `model` roles:

```rust
// Open Responses → Gemini
Role::User      → "user"
Role::Assistant → "model"
Role::System    → system_instruction (separate field)
Role::Tool      → "user" with functionResponse part
```

### Content Format

```rust
// Open Responses → Gemini
InputItem::Message { role: User, content: "Hello" }
    → { "role": "user", "parts": [{ "text": "Hello" }] }

// Multi-modal content
ContentPart::Image { data, media_type }
    → { "inlineData": { "mimeType": media_type, "data": data } }

// Function response
InputItem::FunctionCallOutput { call_id, output }
    → { "role": "user", "parts": [{ "functionResponse": { "name": call_id, "response": output } }] }
```

### Response Transformation

```
Gemini GenerateContent Response → Response
──────────────────────────────────────────

Gemini                           Open Responses
──────                           ──────────────
(generated ID)                → id
model                         → model
candidates[0].content.parts   → output (transformed)
  - text part                 → Item::Message
  - functionCall part         → Item::FunctionCall
candidates[0].finishReason    → status
  - "STOP"                    → ResponseStatus::Completed
  - "MAX_TOKENS"              → ResponseStatus::Incomplete (MaxTokens)
  - "SAFETY"                  → ResponseStatus::Incomplete (ContentFilter)
usageMetadata.promptTokenCount → usage.input_tokens
usageMetadata.candidatesTokenCount → usage.output_tokens
```

### Tool Definition Transformation

```rust
// Open Responses Tool → Gemini Tool
Tool::Function { function: FunctionDefinition { name, description, parameters } }
    → { "functionDeclarations": [{ "name": name, "description": description, "parameters": parameters }] }
```

### Streaming Events Transformation

```
Gemini SSE                       Open Responses SSE
──────────                       ──────────────────
(first chunk)                 → response.created, response.in_progress
candidates[0].content.parts   → response.output_item.added
text delta                    → response.output_text.delta
functionCall                  → response.function_call_arguments.delta
finishReason present          → response.completed / response.incomplete
```

---

## Error Mapping

| Open Responses Error | OpenAI | Anthropic | Gemini |
|---------------------|--------|-----------|--------|
| `invalid_request` | 400 Bad Request | 400 invalid_request_error | 400 INVALID_ARGUMENT |
| `authentication` | 401 Unauthorized | 401 authentication_error | 401 UNAUTHENTICATED |
| `rate_limit` | 429 Too Many Requests | 429 rate_limit_error | 429 RESOURCE_EXHAUSTED |
| `server_error` | 500 Internal Error | 500 api_error | 500 INTERNAL |
| `provider_error` | 502/503 | 529 overloaded_error | 503 UNAVAILABLE |

---

## Feature Support Matrix

| Feature | OpenAI | Anthropic | Gemini |
|---------|--------|-----------|--------|
| Text Messages | ✅ | ✅ | ✅ |
| System Prompt | ✅ | ✅ | ✅ |
| Image Input | ✅ (Vision models) | ✅ | ✅ |
| Audio Input | ✅ (Whisper) | ❌ | ✅ |
| Function Calling | ✅ | ✅ | ✅ |
| Parallel Tool Calls | ✅ | ✅ | ✅ |
| Streaming | ✅ | ✅ | ✅ |
| Reasoning/Thinking | ❌ | ✅ (Extended thinking) | ❌ |
| JSON Mode | ✅ | ✅ | ✅ |
| Structured Output | ✅ | ✅ | ✅ |

---

## Implementation Notes

### 1. ID Prefixing
All provider IDs are prefixed to ensure uniqueness across providers:
- OpenAI: `resp_oai_<original_id>`
- Anthropic: `resp_ant_<original_id>`
- Gemini: `resp_gem_<generated_uuid>`

### 2. System Message Handling
- **OpenAI**: System message as first message in array
- **Anthropic**: Separate `system` field at request level
- **Gemini**: Separate `system_instruction` field

### 3. Tool/Function Naming
- **OpenAI**: Uses `tools` with `type: "function"`
- **Anthropic**: Uses `tools` with direct function definition
- **Gemini**: Uses `tools.functionDeclarations`

### 4. Streaming Buffering
- Provider stream events are buffered and transformed
- Text deltas are accumulated for `output_text.done` events
- Function arguments are validated as complete JSON before `done` events

### 5. Context Window Tracking
- Each provider has different context limits
- Gateway tracks token usage to warn before limits
- Automatic truncation strategies available per provider

---

## Adding a New Provider

To add support for a new provider:

1. Create adapter in `crates/aura-core/src/provider/<name>.rs`
2. Implement the `Provider` trait
3. Add request/response transformers following this mapping guide
4. Add streaming event transformers
5. Update feature support matrix in this document
6. Add provider to registry

See `docs/IMPLEMENTATION_PLAN.md` for the full PR workflow.
