import os
from python_mg import Lexicon
import math

DIR = "samples"


def to_string(sentences):
    s = []
    sentences.sort(key=lambda x: -x[1])
    for string, p in sentences:
        s.append(
            "".join(str(c) for c in string)
            + "\t"
            + str(int(math.exp(p / 10) * 1000) + 1)
        )
    return "\n".join(s)


N_PER_BIN = 10
bins = [0 for _ in range(100)]

bins[1] = N_PER_BIN
bins[8] = N_PER_BIN

lemmas = [str(c) for c in [0, 1, 2, 3, 4, 5, 6]]
n = 0

with open(os.path.join(DIR, "probs.prob"), "w") as prob_f:
    with open(os.path.join(DIR, "probs.hypo"), "w") as hypo_f:
        while not all(b >= N_PER_BIN for b in bins):
            print(bins)
            lexicon = Lexicon.random_lexicon(lemmas)
            mdl = -lexicon.mdl(len(lemmas))
            i = int(-mdl) - 6

            if i < 0:
                continue

            if len(bins) <= i:
                continue

            if bins[i] >= N_PER_BIN:
                continue

            strings = lexicon.generate_unique_strings(category="0", max_strings=100)

            if len(strings) < 10:
                continue

            bins[i] += 1
            with open(os.path.join(DIR, f"{n}.txt"), "w") as f:
                f.write(to_string(strings))
            prob_f.write(f"{mdl}\n")
            hypo_f.write(f"{str(lexicon)}\n")
            n += 1
