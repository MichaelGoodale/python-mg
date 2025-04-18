from typing import Mapping
from python_mg._lib_name import Lexicon, SyntacticStructure, MGNode, MGEdge
import rustworkx as rx


def to_tree(self) -> rx.PyDiGraph[MGNode, MGEdge]:
    (nodes, edges) = self.__to_tree_inner()

    # This will usually be the identity function, but on the off chance its not, we do this.
    # Waste computation in exchange for not having a horrible headache
    old2new: Mapping[int, int] = {}

    G = rx.PyDiGraph()
    for old_node_i, node in nodes:
        new_node = G.add_node(node)
        old2new[old_node_i] = new_node

    for old_src, old_tgt, edge in edges:
        G.add_edge(old2new[old_src], old2new[old_tgt], edge)

    return G


SyntacticStructure.to_tree = to_tree
