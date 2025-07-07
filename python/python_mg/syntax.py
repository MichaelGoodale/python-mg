from __future__ import annotations
from dataclasses import dataclass
from python_mg._lib_name import SyntacticStructure, MGNode, MGEdge
from PIL import Image
import rustworkx as rx
from rustworkx.visualization import graphviz_draw


def sort_key(G, e):
    (_, n) = G.get_edge_endpoints_by_index(e)
    return G.get_node_data(n).trace_id()


@dataclass
class Mover:
    s: list[str | Mover]
    trace: int


@dataclass
class Trace:
    trace: int


def node_attrs(node):
    return {"label": str(node), "ordering": "out"}


def edge_attrs(edge: MGEdge) -> dict[str, str]:
    if edge.is_move():
        return {"style": "dashed", "constraint": "false"}
    return {}


class ParseTree:

    def __init__(
        self, G: rx.PyDiGraph[MGNode, MGEdge], root: int, structure: SyntacticStructure
    ):
        self.root = root
        self.structure = structure
        movement_edges = sorted(
            [x for x in G.filter_edges(lambda x: x.is_move())],
            key=lambda x: sort_key(G, x),
            reverse=True,
        )
        movements = {}
        multitrace_2_single_trace = {}

        for e in movement_edges:
            (src, tgt) = G.get_edge_endpoints_by_index(e)
            G.remove_edge_from_index(e)
            if G.get_node_data(src).is_trace():
                new_trace_id = G.get_node_data(tgt).trace_id()
                trace_id = G.get_node_data(src).trace_id()
                multitrace_2_single_trace[new_trace_id] = trace_id
                movements[trace_id].append(tgt)
            elif G.get_node_data(tgt).is_trace():
                trace_id = G.get_node_data(tgt).trace_id()
                multitrace_2_single_trace[trace_id] = trace_id
                movements[trace_id] = [src, tgt]
        self.G = G
        self.multitrace_2_single_trace = multitrace_2_single_trace
        self.movements = movements
        self.movement_sources = {m[0]: i for i, m in self.movements.items()}

    def normal_string(self) -> str:
        return str(self.structure)

    def base_string(self) -> list[str | Mover | Trace]:
        linear_order = self.__explore(self.root)
        return linear_order

    def transform_movement(self) -> rx.PyDiGraph[MGNode, MGEdge]:
        G = self.G.copy()
        for _, (src, tgt) in self.movements.items():
            src_parent, _, src_weight = G.in_edges(src)[0]
            tgt_parent, _, tgt_weight = G.in_edges(tgt)[0]
            G.remove_edge(src_parent, src)
            G.remove_edge(tgt_parent, tgt)
            G.add_edge(tgt_parent, src, tgt_weight)
            G.add_edge(src_parent, tgt, src_weight)
            G.add_edge(tgt, src, MGEdge.move_edge())
        return G

    def to_dot(self, **kwargs) -> str | None:
        return self.transform_movement().to_dot(
            node_attr=node_attrs, edge_attr=edge_attrs, **kwargs
        )

    def to_image(self, **kwargs) -> Image.Image | None:
        return graphviz_draw(
            self.transform_movement(),
            node_attr_fn=node_attrs,
            edge_attr_fn=edge_attrs,
            **kwargs,
        )

    def __explore(self, n_i: int) -> list[str | Mover | Trace]:
        out = []
        children = [(str(e), n) for (_, n, e) in self.G.out_edges(n_i)]
        left_children = [n for (e, n) in children if e == "L"]
        right_children = [n for (e, n) in children if e != "L"]
        for child in left_children:
            out += self.__explore(child)

        node = self.G.get_node_data(n_i)
        s = self.G.get_node_data(n_i).lemma_string()
        if node.is_trace():
            out.append(Trace(self.multitrace_2_single_trace[node.trace_id()]))
        elif s != "":
            out.append(s)

        for child in right_children:
            out += self.__explore(child)

        if n_i in self.movement_sources:
            return [Mover(out, self.movement_sources[n_i])]

        return out


def to_tree(self) -> ParseTree:
    (nodes, edges, root) = self.__to_tree_inner()

    # This will usually be the identity function, but on the off chance its not, we do this.
    # Waste computation in exchange for not having a horrible headache
    old2new: dict[int, int] = {}

    G = rx.PyDiGraph()
    for old_node_i, node in nodes:
        new_node = G.add_node(node)
        old2new[old_node_i] = new_node

    for old_src, old_tgt, edge in edges:
        G.add_edge(old2new[old_src], old2new[old_tgt], edge)

    return ParseTree(G, old2new[root], self)


SyntacticStructure.to_tree = to_tree
