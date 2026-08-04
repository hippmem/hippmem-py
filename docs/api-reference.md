# API Reference

## Engine

### `Engine.open(path=None, embedder="hash", api_base_url=None, api_key=None, model=None)`

Open or create a HIPPMEM memory store.

| Param         | Type  | Default | Description |
|---------------|-------|---------|-------------|
| path          | str   | `"./hippmem_data"` | Path to the store file |
| embedder      | str   | `"hash"` | `"hash"` (offline SimHash) or `"neural"` (API-based) |
| api_base_url  | str   | `None`   | **Required** when `embedder="neural"` |
| api_key       | str   | `None`   | **Required** when `embedder="neural"` |
| model         | str   | `None`   | **Required** when `embedder="neural"` |

Examples:

```python
# Hash embedder — offline, zero config (default)
engine = Engine.open()

# Neural embedder — API-based, higher semantic accuracy
engine = Engine.open(
    embedder="neural",
    api_base_url="https://api.openai.com/v1",
    api_key="sk-...",
    model="text-embedding-3-small",
)
```

Raises:
- `TypeError` — `embedder="neural"` missing any of `api_base_url` / `api_key` / `model`
- `ValueError` — unknown embedder name

Returns: `Engine`

### `engine.write(content, content_type=None, importance=None)`

Write a memory and discover associations.

| Param        | Type  | Default | Description |
|-------------|-------|---------|-------------|
| content     | str   | —       | Memory text |
| content_type| str   | `None`  | One of: `"Decision"`, `"Preference"`, `"ProjectKnowledge"`, `"TaskState"`, `"Correction"`, `"Event"`, `"Reflection"` |
| importance  | float | `None`  | 0.0–1.0 hint |

Returns: `WriteOutput(memory_id, stage, links_count)`

### `engine.retrieve(query, top_k=5, max_hops=None)`

Retrieve memories via multi-channel seed recall + spreading activation.

| Param    | Type | Default | Description |
|----------|------|---------|-------------|
| query    | str  | —       | Search query |
| top_k    | int  | `5`     | Max results |
| max_hops | int  | `None`  | Graph traversal depth |

Returns: `RetrieveOutput(results, latency_ms, hops_used)`

### `engine.close()`

Flush full-text index. The underlying store is closed when the Engine object
is garbage collected.

## Types

### WriteOutput
- `memory_id: str` — unique memory identifier
- `stage: str` — processing stage reached
- `links_count: int` — number of associations created

### RetrievalResult
- `memory_id: str`
- `score: float` — relevance score
- `content: str` — memory text
- `content_type: str`
- `dimensions: list[str]` — matched dimensions (Entity, Semantic, Causal, etc.)

### RetrieveOutput
- `results: list[RetrievalResult]`
- `latency_ms: int`
- `hops_used: int` — graph traversal hops used
