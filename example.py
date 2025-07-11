from python_mg import Lexicon


grammar = """
::V= C
::V= +W C
knows::C= =D V
says::C= =D V
prefers::D= =D V
drinks::D= =D V
king::N
wine::N
beer::N
queen::N
the::N= D
which::N= D -W
"""
lexicon = Lexicon(grammar)

for p in lexicon.parse("which beer the queen drinks", "C"):
    tree = p.to_tree()
    tree.to_image().show()

for p in lexicon.generate_grammar("C", max_strings=1):
    print(p)
    print(p.latex())
    tree = p.to_tree()
    print(tree.normal_string())
    print(tree.base_string())
