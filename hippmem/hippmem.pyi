"""Type stubs for HIPPMEM Python binding."""

from typing import List, Optional


class WriteOutput:
    """Result of writing a memory."""
    memory_id: str
    """Unique memory identifier."""
    stage: str
    """Processing stage reached."""
    links_count: int
    """Number of associations created."""


class RetrievalResult:
    """A single retrieval result."""
    memory_id: str
    """Unique memory identifier."""
    score: float
    """Relevance score (0.0–1.0)."""
    content: str
    """Memory text content."""
    content_type: str
    """Content type (Decision, Preference, ProjectKnowledge, etc.)."""
    dimensions: List[str]
    """Matched dimensions (e.g., Entity, Semantic, Causal)."""


class RetrieveOutput:
    """Result of a retrieval query."""
    results: List[RetrievalResult]
    """Ranked retrieval results."""
    latency_ms: int
    """Query latency in milliseconds."""
    hops_used: int
    """Graph traversal hops used."""


class Engine:
    """HIPPMEM memory engine.

    Open or create a memory store, write memories, and retrieve them
    via multi-channel associative recall.

    Usage::

        engine = Engine.open()
        engine.write("The user prefers Rust.", content_type="Preference")
        results = engine.retrieve("What language does the user prefer?")
        for r in results.results:
            print(f"[{r.score:.3f}] {r.content}")
        engine.close()
    """

    @staticmethod
    def open(path: Optional[str] = None) -> "Engine":
        """Open or create a HIPPMEM memory store.

        Args:
            path: Path to the store file. Defaults to ``"./hippmem_data"``.

        Returns:
            An Engine instance connected to the store.
        """
        ...

    def write(
        self,
        content: str,
        content_type: Optional[str] = None,
        importance: Optional[float] = None,
    ) -> WriteOutput:
        """Write a memory and discover associations.

        The engine extracts entities, topics, and causal links, then
        discovers associations (entity, temporal, semantic, topic, causal)
        with existing memories. Associations are stored as typed edges
        in the memory graph.

        Args:
            content: Memory text.
            content_type: One of ``"Decision"``, ``"Preference"``,
                ``"ProjectKnowledge"``, ``"TaskState"``, ``"Correction"``,
                ``"Event"``, ``"Reflection"``.
            importance: 0.0–1.0 importance hint.

        Returns:
            WriteOutput with the new memory's ID and link count.
        """
        ...

    def retrieve(
        self,
        query: str,
        top_k: int = 5,
        max_hops: Optional[int] = None,
    ) -> RetrieveOutput:
        """Retrieve memories via multi-channel associative recall.

        Uses 5 recall channels (BM25, entity index, semantic dense,
        semantic binary, topic cluster) fused by RRF, then spreads
        activation over the association graph.

        Args:
            query: Natural-language search query.
            top_k: Maximum number of results.
            max_hops: Graph traversal depth. ``None`` = auto.

        Returns:
            RetrieveOutput with ranked results, latency, and hop count.
        """
        ...

    def close(self) -> None:
        """Flush the full-text index. The store is closed on garbage collection."""
        ...
