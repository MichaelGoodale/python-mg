from python_mg import Lexicon


grammar = """
::T= C
::T= +W C
s::=>V =D T
know::C= V
say::C= V
prefer::D= V
drink::D= V
king::N
wine::N
beer::N
queen::N
the::N= D
which::N= D -W
"""
lexicon = Lexicon(grammar)

tokens = lexicon.tokens()
rev_tokens = {v: k for k, v in tokens.items()}
for p in lexicon.parse("which beer the queen drinks", "C"):
    tree = p.to_tree()
    tree.to_image().show()  # pyright: ignore[reportUnknownMemberType]


for p in lexicon.generate_grammar("C", max_strings=50):
    print(p)
    print(p.latex())
    print(p.log_prob())
    print(p.prob())
    tree = p.to_tree()
    print(tree.normal_string())

    # A rich string which illustrates where movement was generated from
    print(tree.base_string())
