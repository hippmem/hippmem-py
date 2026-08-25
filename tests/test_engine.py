"""Tests for hippmem Engine Python binding."""

import tempfile
import os
import pytest
from hippmem import Engine


@pytest.fixture
def engine():
    """Create an engine with a temporary store."""
    with tempfile.TemporaryDirectory() as tmpdir:
        store_path = os.path.join(tmpdir, "test.redb")
        e = Engine.open(store_path)
        yield e
        e.close()


def test_open_and_close(engine):
    """Engine opens and closes without error."""
    assert engine is not None


# ── embedder parameter (D10) ──

def test_open_explicit_hash_embedder():
    """embedder='hash' is explicit and works."""
    with tempfile.TemporaryDirectory() as tmpdir:
        e = Engine.open(os.path.join(tmpdir, "t.redb"), embedder="hash")
        e.write("test")
        e.close()


def test_open_neural_requires_all_params():
    """embedder='neural' without api params raises TypeError."""
    with tempfile.TemporaryDirectory() as tmpdir:
        path = os.path.join(tmpdir, "t.redb")
        with pytest.raises(TypeError):
            Engine.open(path, embedder="neural")
        with pytest.raises(TypeError):
            Engine.open(path, embedder="neural", api_key="sk-x")
        with pytest.raises(TypeError):
            Engine.open(
                path,
                embedder="neural",
                api_base_url="https://api.openai.com/v1",
                api_key="sk-x",
            )


def test_open_unknown_embedder_raises():
    """Unknown embedder name raises ValueError."""
    with tempfile.TemporaryDirectory() as tmpdir:
        with pytest.raises(ValueError):
            Engine.open(os.path.join(tmpdir, "t.redb"), embedder="magic")


def test_open_neural_full_params_builds():
    """embedder='neural' with all params constructs the embedder (may fail on auth at embed time)."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Construction succeeds; embedding without a real key fails gracefully later.
        e = Engine.open(
            os.path.join(tmpdir, "t.redb"),
            embedder="neural",
            api_base_url="https://api.openai.com/v1",
            api_key="sk-test-invalid",
            model="text-embedding-3-small",
        )
        # write triggers an embedding; with an invalid key it should error, not panic.
        try:
            e.write("hello")
        except Exception:
            pass
        e.close()


# ── feedback / retrieval_id (D11: usage feedback loop) ──

def test_retrieve_exposes_retrieval_id(engine):
    """RetrieveOutput carries a retrieval_id for feedback."""
    engine.write("The user prefers Rust.")
    results = engine.retrieve("What does the user prefer?")
    assert isinstance(results.retrieval_id, int)
    assert results.retrieval_id > 0


def test_feedback_accepts_valid_signals(engine):
    """All four usage signals are accepted."""
    engine.write("The user prefers Rust.")
    results = engine.retrieve("What does the user prefer?")
    used = [r.memory_id for r in results.results]
    for signal in ["referenced", "user_confirmed_correct", "task_succeeded", "user_rejected"]:
        engine.feedback(results.retrieval_id, used, signal)


def test_feedback_unknown_signal_raises(engine):
    """Unknown signal name raises ValueError."""
    engine.write("The user prefers Rust.")
    results = engine.retrieve("What does the user prefer?")
    with pytest.raises(ValueError):
        engine.feedback(results.retrieval_id, [], "bogus_signal")


# ── 0.4.0 behavior contracts (E14) ──

def test_confirm_boosts_memory_across_retrievals(engine):
    """Confirming a memory raises its score in the next retrieval (recent channel,
    which counts confirmed signals only since 0.4.0/B2)."""
    engine.write("Xiaoming and Lihua are high school classmates in Beijing.")
    engine.write("Xiaoming is a computer science student at Peking University.")
    engine.write("Lihua works as a Java engineer at Alibaba Cloud.")
    q = "What is the relationship between Xiaoming and Lihua?"
    before = engine.retrieve(q)
    assert len(before.results) >= 3
    # Confirm the third-ranked memory: the top one is already the RRF max and
    # its normalized energy does not move (see engine-side test notes).
    target = before.results[2]
    engine.feedback(before.retrieval_id, [target.memory_id], "user_confirmed_correct")
    after = engine.retrieve(q)
    after_score = next(r.score for r in after.results if r.memory_id == target.memory_id)
    assert after_score > target.score, (
        f"confirmation must boost the memory: {target.score} -> {after_score}"
    )


def test_retrieve_is_deterministic(engine):
    """Repeated retrieval on the same store is bit-identical (0.4.0 F1/B2)."""
    engine.write("The user prefers Rust for backend services.")
    engine.write("The project uses redb for storage.")
    a = engine.retrieve("What storage does the project use?")
    b = engine.retrieve("What storage does the project use?")
    assert [r.score for r in a.results] == [r.score for r in b.results]
    assert [r.memory_id for r in a.results] == [r.memory_id for r in b.results]


def test_reject_does_not_boost_any_memory(engine):
    """An empty rejection must not lift any returned memory (0.4.1: retrieval-quality signal, no memory-side effects)."""
    engine.write("Zhouyu develops large-model applications at Alibaba Cloud.")
    engine.write("Zhaoqiang handles deployment and monitoring.")
    engine.write("Wangfang evaluates model quality.")
    q = "What work does Zhouyu do?"
    before = engine.retrieve(q)
    engine.feedback(before.retrieval_id, [], "user_rejected")  # empty rejection (no memory-side effects since 0.4.1)
    after = engine.retrieve(q)
    for r_after in after.results:
        r_before = next(
            (x for x in before.results if x.memory_id == r_after.memory_id), None
        )
        if r_before is not None:
            assert r_after.score <= r_before.score + 1e-9, (
                "rejection must not boost any memory"
            )


# ── consolidate ──

def test_consolidate_incremental(engine):
    """Incremental consolidation returns a report."""
    engine.write("The user prefers Rust.", content_type="Preference")
    report = engine.consolidate()
    assert report.memories_processed >= 1
    assert report.elapsed_ms >= 0


def test_consolidate_valid_scopes(engine):
    """All documented scopes are accepted."""
    engine.write("The user prefers Rust.")
    for scope in ["incremental", "full", "edges_only", "reindex"]:
        report = engine.consolidate(scope)
        assert report.elapsed_ms >= 0


def test_consolidate_unknown_scope_raises(engine):
    """Unknown scope name raises ValueError."""
    with pytest.raises(ValueError):
        engine.consolidate("bogus")


def test_write_single(engine):
    """Writing a memory returns expected output."""
    out = engine.write("The user prefers Rust.", content_type="Preference", importance=0.8)
    assert out.memory_id
    assert int(out.memory_id) > 0  # memory_id is a numeric string (u128)
    assert out.stage
    assert out.links_count == 0  # first memory has nothing to link to


def test_write_multiple(engine):
    """Multiple writes create associations."""
    out1 = engine.write("The user likes Rust.", content_type="Preference")
    out2 = engine.write("The user chose redb for embedded storage.", content_type="Decision")
    # Second write may have links to the first
    assert out2.links_count >= 0


def test_retrieve_finds_results(engine):
    """Retrieval returns results after writing."""
    engine.write("The user prefers Rust for backend development.")
    results = engine.retrieve("What language does the user prefer?", top_k=3)
    assert len(results.results) > 0
    assert results.latency_ms >= 0


def test_retrieve_result_structure(engine):
    """Each retrieval result has the expected fields."""
    engine.write("The user thinks PostgreSQL is great for OLTP workloads.", content_type="Preference", importance=0.9)
    results = engine.retrieve("PostgreSQL", top_k=3)
    assert len(results.results) > 0

    r = results.results[0]
    assert r.memory_id
    assert 0.0 <= r.score <= 1.0
    assert len(r.content) > 0
    assert r.content_type
    assert isinstance(r.dimensions, list)


def test_retrieve_with_hops(engine):
    """Retrieval with explicit max_hops works."""
    engine.write("Memory A")
    engine.write("Memory B related to A")
    results = engine.retrieve("Memory A", top_k=3, max_hops=2)
    assert results.hops_used >= 0


def test_custom_content_types(engine):
    """All supported content types work."""
    for ct in ["Decision", "Preference", "ProjectKnowledge", "Correction", "Event"]:
        out = engine.write(f"Test memory with {ct}", content_type=ct)
        assert out.memory_id


def test_default_values(engine):
    """Default parameters work."""
    out = engine.write("A simple memory.")
    assert out.memory_id
    results = engine.retrieve("simple memory")
    assert results.latency_ms >= 0


def test_open_extractor_param(tmp_path):
    """extractor param accepts hash/auto/neural (auto without key -> hash)."""
    import os
    from hippmem import Engine

    # auto without ANTHROPIC_API_KEY -> deterministic fallback, works offline
    e = Engine.open(str(tmp_path / "auto" / "store"), embedder="hash", extractor="auto")
    out = e.write("test extractor param")
    assert out.memory_id
    e.close()

    # explicit hash works
    e2 = Engine.open(str(tmp_path / "hash" / "store"), embedder="hash", extractor="hash")
    e2.write("test extractor hash")
    e2.close()

    # explicit neural without a key fails fast
    os.environ.pop("ANTHROPIC_API_KEY", None)
    try:
        Engine.open(str(tmp_path / "neural" / "store"), embedder="hash", extractor="neural")
        assert False, "neural extractor without ANTHROPIC_API_KEY must fail"
    except RuntimeError as exc:
        assert "ANTHROPIC_API_KEY" in str(exc)

    # invalid value rejected
    try:
        Engine.open(str(tmp_path / "bad" / "store"), embedder="hash", extractor="llm")
        assert False, "extractor='llm' must be rejected (hash|neural|auto)"
    except ValueError:
        pass
