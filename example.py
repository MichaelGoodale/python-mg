from python_mg import Lexicon
from python_mg.metrics import grammar_f1, grammar_f1_from_strings
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

tokens = lexicon.tokens()
rev_tokens = {v: k for k, v in tokens.items()}
for p in lexicon.parse("which beer the queen drinks", "C"):
    tree = p.to_tree()
    tree.to_image().show()


batch: list[npt.NDArray[np.int_]] = []
for p in lexicon.generate_grammar("C", max_strings=100):
    batch.append(p.tokens())
m = max(len(b) for b in batch)
z = np.zeros((len(batch), m), dtype=batch[0].dtype)
z.fill(2)

for i in range(len(batch)):
    z[i, : len(batch[i])] = batch[i]

cont = lexicon.token_continuations(z, "C")

out = np.eye(len(tokens))[z[:, 1:]]
out = np.log(out / out.sum(axis=-1, keepdims=True))

print(grammar_f1_from_strings(lexicon, z, out, "C"))

for i in range(len(z[0]) - 1):
    print(lexicon.detokenize(batch[0]))
    print([rev_tokens[s] for s in cont[0, i, :].nonzero()[0]])


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
