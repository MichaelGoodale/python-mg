use crate::get_config;
use crate::{PyLexicon, PySyntacticStructure};
use minimalist_grammar_parser::PhonContent;
use minimalist_grammar_parser::parsing::beam::Continuation;
use numpy::PyUntypedArrayMethods;
use numpy::ndarray::ArrayD;
use numpy::{PyArray1, PyArrayDyn, PyReadonlyArrayDyn};
use pyo3::{exceptions::PyValueError, prelude::*};
use std::collections::{HashMap, hash_map::Entry};

const SOS: usize = 0;
const EOS: usize = 1;
const PAD: usize = 2;
const AFFIX: usize = 3;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TokenMap(HashMap<String, usize>, HashMap<usize, String>);

impl Default for TokenMap {
    fn default() -> Self {
        let v = HashMap::from([
            ("[PAD]".to_string(), PAD),
            ("[SOS]".to_string(), SOS),
            ("[EOS]".to_string(), EOS),
            ("[AFFIX]".to_string(), AFFIX),
        ]);

        let v_inv = v.iter().map(|(s, n)| (*n, s.clone())).collect();

        Self(v, v_inv)
    }
}

impl TokenMap {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn add_word(&mut self, s: &str) {
        let n = self.0.len();
        match self.0.entry(s.to_string()) {
            Entry::Vacant(vacant_entry) => {
                self.1.insert(n, s.to_string());
                vacant_entry.insert(n);
            }
            Entry::Occupied(_) => (),
        }
    }
}

fn to_phon_content(s: &[usize], lex: &TokenMap) -> PyResult<Vec<PhonContent<String>>> {
    let mut end = s.len() - 1;
    while s.get(end).map(|&x| x == PAD).unwrap_or(false) {
        end -= 1;
    }

    let w = *s
        .get(end)
        .ok_or(PyErr::new::<PyValueError, _>("Empty string"))?;

    if w != EOS {
        return Err(PyErr::new::<PyValueError, _>("No end symbol"));
    }

    let mut i = 0;
    let w = *s
        .get(i)
        .ok_or(PyErr::new::<PyValueError, _>("Empty string"))?;
    i += 1;
    if w != SOS {
        return Err(PyErr::new::<PyValueError, _>("No start symbol"));
    }

    let mut was_affixed = false;
    let mut affix_v = vec![];
    let mut v: Vec<PhonContent<String>> = vec![];

    while i < end {
        let c = *s.get(i).unwrap();
        if c == AFFIX {
            return Err(PyErr::new::<PyValueError, _>("Too many affix symbols"));
        }
        let w = lex
            .1
            .get(&c)
            .ok_or(PyErr::new::<PyValueError, _>("Out of vocabulary"))?
            .clone();

        let next_is_affix = s.get(i + 1).map(|&x| x == AFFIX).unwrap_or(false);
        if next_is_affix {
            i += 1;
            affix_v.push(w);
            was_affixed = true;
        } else {
            if was_affixed {
                affix_v.push(w);
                v.push(PhonContent::Affixed(std::mem::take(&mut affix_v)));
            } else {
                v.push(PhonContent::Normal(w));
            }
            was_affixed = false;
        }
        i += 1;
    }

    if !affix_v.is_empty() {
        return Err(PyErr::new::<PyValueError, _>("Trailing affix!"));
    }
    Ok(v)
}

#[pymethods]
impl PyLexicon {
    ///Gets a dictionary of the word to token ID mapping of this lexicon
    ///
    ///Returns
    ///-------
    ///dictionary of (str, int)
    ///    Dictionary with string to token ID mapping.
    fn tokens(&self) -> &HashMap<String, usize> {
        &self.word_id.0
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (x, category, min_log_prob=-128.0, move_prob=0.5, max_steps=64, n_beams=256))]
    ///Compute valid next token continuations for grammar sequences.
    ///
    ///Takes an array of token sequences in a grammar and returns a boolean mask
    ///indicating which tokens are valid continuations at each position.
    ///
    ///Parameters
    ///----------
    ///x : ndarray of uint, shape (..., N, L)
    ///    Input token sequences where N is the number of sequences and L is the
    ///    maximum sequence length.
    ///category : str
    ///    The syntactic category of the parsed strings
    ///min_log_prob : float or None, optional
    ///    Minimum log probability threshold for the parser to consider
    ///move_prob : float, optional
    ///    Probability of preferring a move over a merge when parsing.
    ///    Default is 0.5
    ///max_steps : int or None, optional
    ///    Maximum number of derivation steps. If None, will not be limited.
    ///    Default is 64.
    ///n_beams : int or None, optional
    ///    Number of beams to maintain while parsing. If none, will not be limited.
    ///    Default is 256.
    ///Returns
    ///-------
    ///ndarray of bool, shape (..., N, L, C)
    ///    Boolean mask indicating valid next tokens for each position, where C is
    ///    the number of tokens in the grammar vocabulary.
    ///
    ///Notes
    ///-----
    ///The output dimensions correspond to:
    ///
    ///    - `...`: Misc batch dimensions (preserved from input)
    ///    - `N`: Number of sequences
    ///    - `L`: Maximum sequence length
    ///    - `C`: Grammar vocabulary size
    fn token_continuations<'py>(
        slf: PyRef<'py, Self>,
        x: PyReadonlyArrayDyn<'py, usize>,
        category: String,
        min_log_prob: Option<f64>,
        move_prob: f64,
        max_steps: Option<usize>,
        n_beams: Option<usize>,
    ) -> PyResult<Bound<'py, PyArrayDyn<bool>>> {
        let original_shape: Vec<usize> = x.shape().to_vec();
        if original_shape.is_empty() {
            return Err(PyValueError::new_err("Target shape is empty!"));
        }
        let d: usize = original_shape
            .iter()
            .take(original_shape.len() - 1)
            .product();
        if d == 0 {
            return Err(PyValueError::new_err("Target shape has empty dimension!"));
        }

        let z = x.as_array();

        let z = z
            .to_shape((d, *original_shape.last().unwrap()))
            .map_err(|x| PyValueError::new_err(x.to_string()))?;

        let mut continuation_matrix = ArrayD::from_elem(
            vec![d, *original_shape.last().unwrap(), slf.word_id.len()],
            false,
        );

        let config = get_config(min_log_prob, move_prob, max_steps, n_beams)?;

        let tokens = &slf.word_id;
        for (i, row) in z.rows().into_iter().enumerate() {
            let s = row.to_slice().unwrap();
            let mut v = vec![];
            let mut last_was_affix = false;

            let mut j = 0;
            while j < s.len() {
                let c = *s.get(j).unwrap();
                if c == PAD || c == EOS {
                    break;
                }
                if c == SOS {
                    if j != 0 {
                        break;
                    }
                } else if c == AFFIX {
                    if last_was_affix || j == 0 || j == 1 {
                        //If affixed appears in these spots, it's ungrammatical
                        break;
                    }
                    last_was_affix = true;
                } else {
                    let w = tokens.1.get(&c).unwrap().clone();

                    let is_affix = s.get(j + 1).map(|&x| x == AFFIX).unwrap_or(false);

                    match (is_affix, last_was_affix) {
                        (_, true) => {
                            if let Some(PhonContent::Affixed(a)) = v.last_mut() {
                                a.push(w);
                            } else {
                                //if last was affix, we must have a affixed going.
                                break;
                            }
                        }
                        (true, false) => {
                            v.push(PhonContent::Affixed(vec![w]));
                        }
                        (false, false) => {
                            v.push(PhonContent::Normal(w));
                        }
                    }
                    last_was_affix = false;
                }

                let cont = slf
                    .lexicon
                    .valid_continuations(category.clone(), &v, &config)
                    .map_err(|x| PyValueError::new_err(x.to_string()))?;

                for next in cont {
                    match next {
                        Continuation::Word(w) => {
                            let c = *tokens.0.get(&w).unwrap();
                            (*continuation_matrix.get_mut([i, j, c]).unwrap()) = true;
                        }
                        Continuation::AffixedWord(items) => {
                            let n = items.len() * 2 - 1;

                            let items = items
                                .into_iter()
                                .flat_map(|w| [*tokens.0.get(&w).unwrap(), AFFIX].into_iter())
                                .take(n);

                            let mut last = None;
                            for (offset, c) in items.enumerate() {
                                if offset == 0 || s.get(j + offset) == last.as_ref() {
                                    (*continuation_matrix.get_mut([i, j + offset, c]).unwrap()) =
                                        true;
                                    last = Some(c);
                                } else {
                                    break;
                                }
                            }
                        }
                        Continuation::EndOfSentence => {
                            (*continuation_matrix.get_mut([i, j, EOS]).unwrap()) = true;
                        }
                    }
                }
                j += 1;
            }
        }
        let mut target_shape = original_shape;
        target_shape.push(slf.word_id.len());

        let py = slf.py();
        let v = PyArrayDyn::from_owned_array(
            py,
            continuation_matrix
                .into_shape_with_order(target_shape)
                .map_err(|e| PyValueError::new_err(e.to_string()))?,
        );

        Ok(v)
    }

    /// Convert a batch of sequence of tokens to their corresponding strings.
    ///
    /// Parameters
    /// ----------
    /// s : Sequence[Sequence[int]], npt.NDArray[np.uint] or list[npt.NDArray[np.uint]]
    ///     A sequence or array of token IDs to be converted to strings.
    ///
    /// Returns
    /// -------
    /// list[list[str]]
    ///     List of list of strings corresponding to the input tokens.
    fn detokenize_batch(&self, batch: Vec<Vec<usize>>) -> Vec<Vec<String>> {
        batch
            .iter()
            .map(|v| {
                v.iter()
                    .map(|x| {
                        self.word_id
                            .1
                            .get(x)
                            .cloned()
                            .unwrap_or_else(|| "[OOV]".to_string())
                    })
                    .collect()
            })
            .collect()
    }

    /// Convert a sequence of tokens to their corresponding strings.
    ///
    /// Parameters
    /// ----------
    /// s : Sequence[int] or npt.NDArray[np.uint]
    ///     A sequence or array of token IDs to be converted to strings.
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     List of strings corresponding to the input tokens.
    fn detokenize(&self, s: Vec<usize>) -> Vec<String> {
        s.into_iter()
            .map(|x| {
                self.word_id
                    .1
                    .get(&x)
                    .cloned()
                    .unwrap_or_else(|| "[OOV]".to_string())
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (s, category, min_log_prob=-128.0, move_prob=0.5, max_steps=64, n_beams=256, max_parses=None))]
    ///Converts a sequence of tokens into a list of SyntacticStructure. Will throw a ValueError if
    ///the tokens are not formatted properly (but the list will be  empty if there is no parse).
    ///
    ///Parameters
    ///----------
    ///x : ndarray of uint, shape (L,)
    ///    Input token sequences where L is the sequence length
    ///category : str
    ///    The syntactic category of the parsed strings
    ///min_log_prob : float or None, optional
    ///    Minimum log probability threshold for the parser to consider
    ///move_prob : float, optional
    ///    Probability of preferring a move over a merge when parsing.
    ///    Default is 0.5
    ///max_steps : int or None, optional
    ///    Maximum number of derivation steps. If None, will not be limited.
    ///    Default is 64.
    ///n_beams : int or None, optional
    ///    Number of beams to maintain while parsing. If none, will not be limited.
    ///    Default is 256.
    ///Returns
    ///-------
    ///    list of :meth:`python_mg.SyntacticStructure`
    ///    List of all parses of the token string
    fn parse_tokens(
        slf: PyRef<'_, Self>,
        s: Vec<usize>,
        category: String,
        min_log_prob: Option<f64>,
        move_prob: f64,
        max_steps: Option<usize>,
        n_beams: Option<usize>,
        max_parses: Option<usize>,
    ) -> PyResult<Vec<PySyntacticStructure>> {
        let v = to_phon_content(&s, &slf.word_id)?;
        PyLexicon::inner_parse(
            slf,
            &v,
            category,
            min_log_prob,
            move_prob,
            max_steps,
            n_beams,
            max_parses,
        )
    }
}

#[pymethods]
impl PySyntacticStructure {
    ///Converts the SyntacticStructure to a tokenized representation of its string.
    ///
    ///Returns
    ///-------
    ///ndarray of uint
    ///    the tokenized string.
    fn tokens<'py>(slf: PyRef<'py, Self>) -> Bound<'py, PyArray1<usize>> {
        let tokens = slf.lex.get().tokens();

        let mut output = vec![SOS];
        for c in slf.string.iter() {
            match c {
                PhonContent::Normal(w) => output.push(
                    *tokens
                        .get(w)
                        .expect("Invalid syntactic structure for this lexicon"),
                ),
                PhonContent::Affixed(items) => output.extend(
                    items
                        .iter()
                        .flat_map(|w| {
                            let token = *tokens
                                .get(w)
                                .expect("Invalid syntactic structure for this lexicon");
                            [token, AFFIX].into_iter()
                        })
                        .take(items.len() * 2 - 1), //Don't take the last affix
                ),
            };
        }
        output.push(EOS);
        let py = slf.py();
        PyArray1::from_vec(py, output)
    }
}
