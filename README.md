# hippmem-py

**Python bindings for [HIPPMEM](https://github.com/hippmem/hippmem) — native associative memory engine for AI agents.**

[![CI](https://github.com/hippmem/hippmem-py/actions/workflows/ci.yml/badge.svg)](https://github.com/hippmem/hippmem-py/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/hippmem.svg)](https://pypi.org/project/hippmem/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

## Install

```bash
pip install hippmem
```

No GPU, API key, or network required — the deterministic fallback backend works offline.

## Quick Start

```python
from hippmem import Engine

engine = Engine.open()
engine.write("The user prefers Rust.", content_type="Preference")
engine.write("The user chose redb — pure Rust, fast compile.", content_type="Decision")

results = engine.retrieve("Why did the user choose redb?", top_k=3)
for r in results.results:
    print(f"[{r.score:.3f}] {r.content}")
    print(f"  dimensions: {r.dimensions}")

engine.close()
```

## Why associative memory?

Most memory solutions are vector databases — store embeddings, search by similarity.
HIPPMEM discovers **associations between memories at write time** (entity, causal,
temporal, semantic, topic) and retrieves via **spreading activation** over typed
edges. The result: it remembers WHY, not just WHAT.

## Documentation

| Document | Content |
|----------|---------|
| [Quick Start](docs/quickstart.md) | 5-minute setup and first run |
| [API Reference](docs/api-reference.md) | Method signatures and types |
| [Configuration](docs/configuration.md) | Storage location, backend selection |

For deeper architecture and algorithm details, see the
[main HIPPMEM documentation](https://github.com/hippmem/hippmem/tree/main/docs).

## Development

```bash
git clone https://github.com/hippmem/hippmem-py.git
cd hippmem-py
python3 -m venv .venv && source .venv/bin/activate
pip install maturin pytest
maturin develop
pytest
```

## License

Apache 2.0. See [LICENSE](LICENSE) and [COPYRIGHT](COPYRIGHT).

The underlying [hippmem-engine](https://crates.io/crates/hippmem-engine) is
AGPL-3.0-only. Importing hippmem-py does NOT subject your program to AGPL —
only the engine crate itself is AGPL. See the
[HIPPMEM licensing overview](https://github.com/hippmem/hippmem/blob/main/COPYRIGHT).
