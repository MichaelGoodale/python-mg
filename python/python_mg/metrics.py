from typing import Literal

import numpy as np
import numpy.typing as npt

from python_mg._lib_name import Lexicon


def grammar_f1(
    preds: npt.NDArray[np.float64],
    correct: npt.NDArray[np.bool],
) -> dict[str, npt.NDArray[np.float64]]:
    if preds.shape != correct.shape:
        raise ValueError("correct and preds must have matching shapes")

    precision: npt.NDArray[np.float64] = np.exp(  # pyright: ignore[reportAny]
        np.logaddexp.reduce(
            np.where(correct, preds, -np.inf), axis=-1
        )  # pyright: ignore[reportAny]
    )

    total_bad: npt.NDArray[np.float64] = (  # pyright: ignore[reportAny]
        np.logaddexp.reduce(np.where(~correct, preds, -np.inf), axis=-1, keepdims=True)
    )
    better_than_bad = np.where(np.where(correct, preds, -np.inf) > total_bad, 1.0, 0.0)

    recall = np.where(correct, better_than_bad, 0.0).sum(  # pyright: ignore[reportAny]
        axis=-1
    ) / correct.sum(axis=-1)

    return {
        "f1": (2 * precision * recall) / (precision + recall),
        "precision": precision,
        "recall": recall,
    }


def grammar_f1_from_strings(
    lexicon: Lexicon,
    tokens: npt.NDArray[np.int_],
    preds: npt.NDArray[np.float64],
    category: str,
    min_log_prob: float | None = -128.0,
    move_prob: float = 0.5,
    max_steps: int | None = 64,
    n_beams: int | None = 256,
    reduction: Literal["none", "sentence_mean"] = "sentence_mean",
) -> dict[str, npt.NDArray[np.float64]]:
    conts = lexicon.token_continuations(
        tokens,
        category,
        min_log_prob=min_log_prob,
        move_prob=move_prob,
        max_steps=max_steps,
        n_beams=n_beams,
    )[..., :-1, :]

    d = grammar_f1(preds, conts)

    if reduction == "sentence_mean":
        mask = (tokens[..., :-1] != 2) & (  # pyright: ignore[reportAny]
            tokens[..., :-1] != 1
        )

        d = {
            k: np.where(mask, v, 0.0).sum(axis=-1)  # pyright: ignore[reportAny]
            / mask.sum(axis=-1)  # pyright: ignore[reportAny]
            for k, v in d.items()
        }
    elif reduction != "none":
        raise ValueError(
            f'"{reduction}" is not a valid reduction'
        )  # pyright: ignore[reportUnreachable]

    return d
