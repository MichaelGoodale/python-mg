from typing import Literal

import numpy as np
import numpy.typing as npt

from python_mg._lib_name import Lexicon


def grammar_f1(
    preds: npt.NDArray[np.float64],
    correct: npt.NDArray[np.bool],
) -> dict[str, npt.NDArray[np.float64]]:
    """
    Compute grammar F1 scores from boolean arrays of valid next moves and predictions.
    The metric is described in  `Meta-Learning Neural Mechanisms rather than Bayesian Priors <https://aclanthology.org/2025.acl-long.860/>`_ (Goodale et al., ACL 2025)

    Parameters
    ----------
    preds : ndarray of float64
        Predicted log probabilities for each token. Shape (..., seq_length, vocab_size).
    correct: ndarray of int
        Boolean array for each valid token that can come next at that point in the sequence. Shape (..., seq_length, vocab_size).

    Returns
    -------
    dict of str : ndarray of float64
        Dictionary containing numpy arrays with keys:

        - 'precision': Precision scores
        - 'recall': Recall scores
        - 'f1': F1 scores
    """
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
    reduction: Literal["none", "sentence_mean", "length_mean"] = "sentence_mean",
) -> dict[str, npt.NDArray[np.float64]]:
    """
    Compute grammar F1 scores from token sequences and predictions.
    The metric is described in  `Meta-Learning Neural Mechanisms rather than Bayesian Priors <https://aclanthology.org/2025.acl-long.860/>`_ (Goodale et al., ACL 2025)


    Parameters
    ----------
    lexicon : Lexicon
    tokens : ndarray of int
        Token IDs representing the input sequences. Shape (..., seq_length).
    preds : ndarray of float64
        Predicted log probabilities for each token. Shape (..., seq_length, vocab_size).
    category : str
        The syntactic category of the parsed strings
    min_log_prob : float or None, optional
        Minimum log probability threshold for the parser to consider
    move_prob : float, optional
        Probability of preferring a move over a merge when parsing.
        Default is 0.5
    max_steps : int or None, optional
        Maximum number of derivation steps. If None, will not be limited.
        Default is 64.
    n_beams : int or None, optional
        Number of beams to maintain while parsing. If none, will not be limited.
        Default is 256.
    reduction : {'none', 'sentence_mean', 'length_mean'}, optional
        Method for reducing F1 scores across sequences:

        - 'none': Return individual scores per sequence
        - 'sentence_mean': Average over all sequences, ignoring padded tokens
        - 'length_mean': Average over lengths, ignoring padding tokens

        Default is 'sentence_mean'.

    Returns
    -------
    dict of str : ndarray of float64
        Dictionary containing numpy arrays with keys:

        - 'precision': Precision scores
        - 'recall': Recall scores
        - 'f1': F1 scores
    """

    if np.any(tokens < 0):
        raise ValueError(
            "Some tokens are negative which means they will be cast to unsigned integers incorrectly"
        )

    conts = lexicon.token_continuations(
        tokens.astype(np.uint64),
        category,
        min_log_prob=min_log_prob,
        move_prob=move_prob,
        max_steps=max_steps,
        n_beams=n_beams,
    )[..., :-1, :]

    d = grammar_f1(preds, conts)

    mask = (tokens[..., :-1] != 2) & (  # pyright: ignore[reportAny]
        tokens[..., :-1] != 1
    )

    if reduction == "sentence_mean":
        d = {
            k: np.where(mask, v, 0.0).sum(axis=-1)  # pyright: ignore[reportAny]
            / mask.sum(axis=-1)  # pyright: ignore[reportAny]
            for k, v in d.items()
        }
    elif reduction == "length_mean":
        d = {
            k: np.where(mask, v, 0.0).sum(  # pyright: ignore[reportAny]
                axis=tuple(range(tokens.ndim - 1))
            )
            / mask.sum(axis=tuple(range(tokens.ndim - 1)))  # pyright: ignore[reportAny]
            for k, v in d.items()
        }
    elif reduction != "none":
        raise ValueError(
            f'"{reduction}" is not a valid reduction'
        )  # pyright: ignore[reportUnreachable]

    return d
