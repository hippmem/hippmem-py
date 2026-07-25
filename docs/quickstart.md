# Quick Start

```bash
pip install hippmem
```

```python
from hippmem import Engine

# Open (creates hippmem_data file in current directory)
engine = Engine.open()

# Write memories
engine.write("The user prefers Rust.", content_type="Preference")
engine.write("The user chose redb — pure Rust, fast compile.", content_type="Decision")

# Retrieve — associative memory finds WHY, not just WHAT
results = engine.retrieve("Why did the user choose redb?", top_k=3)
for r in results.results:
    print(f"[{r.score:.3f}] {r.content}")
    print(f"  dimensions: {r.dimensions}")

engine.close()
```

No GPU, API key, or network required.
