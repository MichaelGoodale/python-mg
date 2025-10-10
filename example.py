from python_mg import Lexicon
import numpy as np
import numpy.typing as npt


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

print(lexicon.tokens())
for p in lexicon.parse("which beer the queen drinks", "C"):
    tree = p.to_tree()
    tree.to_image().show()


batch: list[npt.NDArray[np.int_]] = []
for p in lexicon.generate_grammar("C", max_strings=10):
    batch.append(p.tokens())
print(lexicon.detokenize_batch(batch))

for p in lexicon.generate_grammar("C", max_strings=50):
    print(p)
    tokens = p.tokens()
    print(tokens)
    print(lexicon.detokenize(tokens))
    print(lexicon.detokenize(tokens.tolist()))
    print(lexicon.parse_tokens(tokens, "C"))
    print(p.latex())
    print(p.log_prob())
    print(p.prob())
    tree = p.to_tree()
    print(tree.normal_string())
    print(tree.base_string())
