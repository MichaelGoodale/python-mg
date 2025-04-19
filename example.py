from python_mg import Lexicon

from rustworkx.visualization import graphviz_draw


def display(node):
    return {"label": str(node)}


def edge_attr(edge):
    return {"color": "red", "label": str(edge)}


grammar = """
everyone::d -k -q
someone::d -k -q
likes::d= V -v
::v= +v +k +q t
::V= +k d= +q v
"""

lexicon = Lexicon(grammar)

for p in lexicon.generate_grammar("t", max_strings=100):
    print(p)
    print(p.latex())
    tree = p.to_tree()
    print(tree.normal_string())
    print(tree.base_string())
