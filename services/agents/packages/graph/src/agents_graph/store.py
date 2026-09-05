"""An in-memory, indexed graph store implementing the §7.1 schema's real
query semantics — standing in for FalkorDB, which needs a Docker daemon
this sandbox doesn't have (same substitution this project made for
`hmmlearn` in Phase 5's regime classifier: the real algorithm's shape is
kept, the specific engine is swapped for one this environment can actually
run and test). `KnowledgeGraph` is dependency-injectable, so a real
FalkorDB-backed implementation of the same interface (`GRAPH.QUERY` over
`redis`) can replace it without touching `agents_graph`'s ingest/query
callers — the same port/adapter split as `SimBroker`/`InMemoryCoreClient`
in earlier phases.

Every node upsert and edge upsert here is idempotent by id (a real Cypher
`MERGE`'s behavior), and both node-by-label and edge-by-(node, label)
lookups are O(1) dict indices rather than a linear scan — the data
structure choice that makes the §7.2 conditional-reliability query
benchmark in `queries.py` meaningful rather than accidentally fast only at
small N.
"""

from __future__ import annotations

from collections import defaultdict
from collections.abc import Iterable

from .schema import Edge, EdgeLabel, Node, NodeLabel


class KnowledgeGraph:
    def __init__(self) -> None:
        self._nodes: dict[str, Node] = {}
        self._nodes_by_label: dict[NodeLabel, dict[str, Node]] = defaultdict(dict)
        # Multi-edge safe: keyed by (label, source_id, target_id) so a
        # repeated upsert of the same relationship replaces it rather than
        # duplicating it, matching a real `MERGE` on a relationship.
        self._edges: dict[tuple[EdgeLabel, str, str], Edge] = {}
        self._out_index: dict[tuple[str, EdgeLabel], dict[str, Edge]] = defaultdict(dict)
        self._in_index: dict[tuple[str, EdgeLabel], dict[str, Edge]] = defaultdict(dict)

    def upsert_node(self, node: Node) -> None:
        self._nodes[node.id] = node
        self._nodes_by_label[node.label][node.id] = node

    def upsert_edge(self, edge: Edge) -> None:
        key = (edge.label, edge.source_id, edge.target_id)
        self._edges[key] = edge
        self._out_index[(edge.source_id, edge.label)][edge.target_id] = edge
        self._in_index[(edge.target_id, edge.label)][edge.source_id] = edge

    def get_node(self, node_id: str) -> Node | None:
        return self._nodes.get(node_id)

    def nodes_by_label(self, label: NodeLabel) -> Iterable[Node]:
        return self._nodes_by_label[label].values()

    def node_count(self) -> int:
        return len(self._nodes)

    def edge_count(self) -> int:
        return len(self._edges)

    def out_neighbors(self, node_id: str, label: EdgeLabel) -> list[tuple[Edge, Node]]:
        """Nodes reached by following `label` edges *out of* `node_id`."""
        result = []
        for target_id, edge in self._out_index[(node_id, label)].items():
            node = self._nodes.get(target_id)
            if node is not None:
                result.append((edge, node))
        return result

    def in_neighbors(self, node_id: str, label: EdgeLabel) -> list[tuple[Edge, Node]]:
        """Nodes that reach `node_id` by a `label` edge — i.e. `node_id` is
        each edge's target."""
        result = []
        for source_id, edge in self._in_index[(node_id, label)].items():
            node = self._nodes.get(source_id)
            if node is not None:
                result.append((edge, node))
        return result
