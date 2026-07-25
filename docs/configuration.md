# Configuration

## Storage location

```python
# Default: ./hippmem_data
engine = Engine.open()

# Custom path
engine = Engine.open("/path/to/memory.redb")
```

## Backend selection

By default, hippmem uses a deterministic fallback backend (no API key required).

To use an OpenAI-compatible API for higher semantic accuracy, set the
following environment variables before importing:

```bash
export HIPPMEM_EMBEDDER_BACKEND=openai
export OPENAI_API_KEY=sk-...
export OPENAI_BASE_URL=https://api.openai.com/v1  # or your compatible endpoint
```

See [hippmem-engine configuration](https://github.com/hippmem/hippmem/blob/main/docs/configuration.md)
for full details (model selection, algorithm parameters, background workers).
