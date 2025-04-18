from typing import Mapping
from python_mg._lib_name import Lexicon, SyntacticStructure, MGNode, MGEdge
import rustworkx as rx


def sort_key(G, e):
    (_, n) = G.get_edge_endpoints_by_index(e)
    return G.get_node_data(n).trace_id()


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
                movements[trace_id] = [src, tgt]
        self.G = G
        self.movements = movements

    def normal_string(self) -> str:
        return str(self.structure)

    def base_string(self):
        edges = rx.dfs_edges(self.G, source=self.root)
        nodes = [self.G.get_node_data(t).lemma_string() for _, t in edges]
        nodes.reverse()
        print(nodes)


def to_tree(self) -> ParseTree:
    (nodes, edges, root) = self.__to_tree_inner()

    # This will usually be the identity function, but on the off chance its not, we do this.
    # Waste computation in exchange for not having a horrible headache
    old2new: Mapping[int, int] = {}

    G = rx.PyDiGraph()
    for old_node_i, node in nodes:
        new_node = G.add_node(node)
        old2new[old_node_i] = new_node

    for old_src, old_tgt, edge in edges:
        G.add_edge(old2new[old_src], old2new[old_tgt], edge)

    return ParseTree(G, old2new[root], self)


SyntacticStructure.to_tree = to_tree
