import torch
from datasets import Dataset, DatasetDict
import numpy as np
import numpy.typing as npt
from typing import Any
from python_mg import Lexicon, SyntacticStructure
from collections.abc import Callable, Iterator

from transformers import (
    TrainingArguments,
    Trainer,
    LlamaConfig,
    LlamaForCausalLM,
)


def generator(
    lex: Lexicon, category: str, max_steps: int = 40
) -> Callable[[], Iterator[dict[str, npt.NDArray[np.int_] | str | float]]]:

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


def collate(features: list[dict[str, npt.NDArray[np.int_]]]) -> dict[str, Any]:
    n = max(len(d["label_ids"]) for d in features)
    X = np.full((len(features), n), 2)
    for i, d in enumerate(features):
        x = d["label_ids"]
        X[i, : len(x)] = x

    input_ids = torch.tensor(X)
    labels = input_ids.clone()
    labels[labels == 2] = -100
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
    data = Dataset.from_generator(generator(lexicon, "C"))
    data = DatasetDict(
        {
            "train": data.filter(lambda x: x["n_steps"] < 30),
            "test": data.filter(lambda x: x["n_steps"] >= 30)
            .shuffle()
            .select(range(1024)),
        }
    )

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

    # Define a small Llama config from scratch
    config = LlamaConfig(
        vocab_size=len(lexicon.tokens()),  # Vocabulary size
        hidden_size=128,  # Embedding dimension (small model)
        intermediate_size=256,  # FFN hidden size (usually 4x hidden_size)
        num_hidden_layers=6,  # Number of transformer layers
        num_attention_heads=8,  # Number of attention heads
        num_key_value_heads=8,  # For grouped-query attention (set equal to num_attention_heads for standard)
        max_position_embeddings=2048,  # Maximum sequence length
        rms_norm_eps=1e-6,  # RMSNorm epsilon
        initializer_range=0.02,  # Weight initialization range
        use_cache=True,  # Enable KV caching for inference
        pad_token_id=2,
        bos_token_id=0,
        eos_token_id=1,
        tie_word_embeddings=False,  # Whether to tie input/output embeddings
        rope_theta=10000.0,  # RoPE base frequency
    )

    config.pad_token = 2
    model = LlamaForCausalLM(config)

    print(model.num_parameters())

    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=data["train"],
        eval_dataset=data["test"],
        data_collator=collate,
    )
    trainer.train()
    starting_matrix = torch.zeros(50, 1, dtype=torch.long, device=model.device)
    generation = model.generate(starting_matrix, do_sample=True)

    grammatical = []
    for x in generation.tolist():
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


if __name__ == "__main__":
    main()
