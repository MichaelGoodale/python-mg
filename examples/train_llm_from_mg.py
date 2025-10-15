# pyright: reportUnknownVariableType=false, reportUnknownMemberType=false, reportUnknownLambdaType=false
import typing
from collections.abc import Callable, Iterator

import numpy as np
import numpy.typing as npt
import torch
from scipy.special import log_softmax
from datasets import Dataset, DatasetDict
from python_mg import Lexicon, SyntacticStructure
from python_mg.metrics import grammar_f1_from_strings
from transformers import (
    LlamaConfig,
    LlamaForCausalLM,
    Trainer,
    TrainingArguments,
)


def generator(
    lex: Lexicon, category: str, max_steps: int = 40
) -> Callable[[], Iterator[dict[str, npt.NDArray[np.uint] | str | float | int]]]:
    """Takes a Lexicon and returns a generator which tokenizes samples from it"""

    def f():
        for p in lex.generate_grammar(
            category,
            max_steps=max_steps,
            n_beams=None,
            max_strings=None,
            min_log_prob=None,
        ):
            yield {
                "label_ids": p.tokens(),
                "text": str(p),
                "log_p": p.log_prob(),
                "n_steps": p.n_steps(),
            }

    return f


def collate(
    features: list[dict[str, npt.NDArray[np.int_]]],
) -> dict[str, torch.Tensor]:
    """Pads different tokens together"""
    n = max(len(d["label_ids"]) for d in features)
    X = np.full((len(features), n), 2)
    for i, d in enumerate(features):
        x = d["label_ids"]
        X[i, : len(x)] = x

    input_ids = torch.tensor(X)
    labels = input_ids.clone()
    labels[labels == 2] = -100  # -100 is a magic number in huggingface
    return {"input_ids": input_ids, "labels": labels}


def main():

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

    # creates a huggingface dataset from a lexicon
    corpus: Dataset = Dataset.from_generator(  # pyright: ignore[reportAssignmentType ]
        generator(lexicon, "C")
    )

    data: DatasetDict = DatasetDict(  # type: ignore
        {
            "train": corpus.filter(lambda x: x["n_steps"] < 30),
            "eval": corpus.filter(lambda x: x["n_steps"] >= 30)
            .shuffle()
            .select(range(1024)),  # random sample of longer derivations
            "test": corpus.filter(lambda x: x["n_steps"] >= 30)
            .shuffle()
            .select(range(25)),
        }
    )

    # define a Small LLama Model to train
    config = LlamaConfig(
        vocab_size=len(lexicon.tokens()),
        hidden_size=128,
        intermediate_size=256,
        num_hidden_layers=6,
        num_attention_heads=8,
        num_key_value_heads=8,
        max_position_embeddings=2048,
        rms_norm_eps=1e-6,
        initializer_range=0.02,
        use_cache=True,
        pad_token_id=2,
        bos_token_id=0,
        eos_token_id=1,
        tie_word_embeddings=False,
        rope_theta=10000.0,
    )

    config.pad_token = 2
    model = LlamaForCausalLM(config)

    print(model.num_parameters())

    # train a model
    training_args = TrainingArguments(
        output_dir="mg-training",
        eval_strategy="epoch",
        per_device_train_batch_size=64,
        per_device_eval_batch_size=256,
        learning_rate=1e-4,
        adam_beta1=0.95,
        adam_beta2=0.999,
        push_to_hub=False,
        num_train_epochs=100,
        torch_compile=True,
        torch_compile_backend="inductor",
        torch_compile_mode="default",
        save_strategy="no",
    )

    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=data["train"],
        eval_dataset=data["eval"],
        data_collator=collate,
    )
    trainer.train()

    # evaluate the model

    # go over 50 randomly sampled strings and see how many are grammatical
    starting_matrix = torch.zeros(50, 1, dtype=torch.long, device=model.device)
    generation = (
        model.generate(starting_matrix, do_sample=True)
    ).tolist()  # pyright: ignore[reportAttributeAccessIssue]
    generation = typing.cast(list[list[int]], generation)

    grammatical: list[int] = []
    for x in generation:
        text = " ".join(lexicon.detokenize(x))
        parses: list[SyntacticStructure] = []
        try:
            parses += lexicon.parse_tokens(x, category="C")
        except ValueError:
            pass
        if len(parses) != 0:
            print(f"{text} (grammatical)")
            grammatical.append(1)
        else:
            print(f"{text} (ungrammatical)")
            grammatical.append(0)

    print(
        f"{ (sum(grammatical) / len(grammatical)) * 100 }% grammatical generations",
    )

    # What is the F1 score of the
    pred = trainer.predict(data["test"])  # pyright: ignore[reportArgumentType]
    x = typing.cast(npt.NDArray[np.float64], log_softmax(pred.predictions, axis=-1))
    labels = typing.cast(npt.NDArray[np.int_], pred.label_ids)

    # Average F1 normalized by strings
    grammar_f1 = grammar_f1_from_strings(
        lexicon, labels, x[:, :-1, :], "C", reduction="sentence_mean"
    )["f1"].mean(axis=-1)

    print(
        f"The model has an average grammar F1 of {grammar_f1} over 50 random eval strings"
    )

    # Average F1 across lengths
    length_f1 = grammar_f1_from_strings(
        lexicon, labels, x[:, :-1, :], "C", reduction="length_mean"
    )["f1"]

    print(f"The model has the following F1 over lengths: {length_f1}")

    p = lexicon.detokenize(
        lexicon.parse(
            "the king know-s the queen know-s which wine the king prefer-s", "C"
        )[0].tokens()
    )

    # We can compare the average F1 across length with a specific string and figure out when the F1 is bad.
    f1_vs_string: list[tuple[float, str]] = []
    for i in range(len(length_f1)):
        f1_vs_string.append((float(length_f1[i]), p[i]))
    print(f1_vs_string)


if __name__ == "__main__":
    main()
