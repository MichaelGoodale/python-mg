use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    sync::Arc,
};

use anyhow::anyhow;
use logprob::LogProb;
use minimalist_grammar_parser::{
    Generator, ParsingConfig, PhonContent, Pronounciation,
    lexicon::{LexemeId, LexicalEntry, Lexicon, SemanticLexicon},
    parsing::beam::Continuation,
};
use pyo3::{exceptions::PyValueError, prelude::*};

pub mod graphing;
use graphing::{PyMgEdge, PyMgNode};

mod semantics;
mod syntax;
mod tokenizers;
use syntax::PySyntacticStructure;

use crate::{
    semantics::{
        PyMeaning, PyPossibleEvent, PyScenarioGenerator,
        lot_types::{PyActor, PyEvent},
        scenario::PyScenario,
    },
    tokenizers::TokenMap,
};

#[derive(Debug, Clone, Eq, PartialEq)]
enum PossiblySemanticLexicon {
    Normal(Lexicon<&'static str, &'static str>),
    Semantic(SemanticLexicon<'static, &'static str, &'static str>),
}

impl Display for PossiblySemanticLexicon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PossiblySemanticLexicon::Normal(lex) => write!(f, "{lex}"),
            PossiblySemanticLexicon::Semantic(lex) => write!(f, "{lex}"),
        }
    }
}

impl PossiblySemanticLexicon {
    fn new(s: &'static str) -> anyhow::Result<Self> {
        if let Ok(lex) = Lexicon::from_string(s) {
            Ok(PossiblySemanticLexicon::Normal(lex))
        } else {
            Ok(PossiblySemanticLexicon::Semantic(SemanticLexicon::parse(
                s,
            )?))
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SelfOwningLexicon {
    lexicon: PossiblySemanticLexicon,
    ///Must be last so it is dropped after previous things.
    string: Arc<String>,
}

impl SelfOwningLexicon {
    fn new(s: String) -> anyhow::Result<Self> {
        let string = Arc::new(s);
        let str: &'static str = unsafe { std::mem::transmute(string.as_str()) };

        Ok(SelfOwningLexicon {
            lexicon: PossiblySemanticLexicon::new(str)?,
            string,
        })
    }

    #[expect(clippy::needless_lifetimes)]
    fn lexicon<'a>(&'a self) -> &'a Lexicon<&'a str, &'a str> {
        match &self.lexicon {
            PossiblySemanticLexicon::Normal(lexicon) => lexicon,
            PossiblySemanticLexicon::Semantic(semantic_lexicon) => semantic_lexicon.lexicon(),
        }
    }

    fn semantic_lexicon<'a>(&'a self) -> Option<&'a SemanticLexicon<'a, &'a str, &'a str>> {
        match &self.lexicon {
            PossiblySemanticLexicon::Normal(_) => None,
            PossiblySemanticLexicon::Semantic(lex) => Some(lex),
        }
    }
}

impl Display for SelfOwningLexicon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lexicon)
    }
}

#[pyclass(
    name = "Lexicon",
    str,
    eq,
    frozen,
    module = "python_mg",
    from_py_object
)]
#[derive(Debug, Clone, Eq, PartialEq)]
///A MG grammar that can be used to generate SyntacticStructures or parse strings into
///SyntacticStructures.
///
///You may include semantic interpretations or not. You may also generate all valid sentences in the grammar.
///
///Parameters
///----------
///grammar : str
///     
///Raises
///------
///ValueError
///    If the string is not a valid lexicon.
///
///Examples
///--------
///Generating all sentences of a grammar.
///
///.. code-block:: python
///
///    grammar = """John::d
///    runs::=d v
///    Mary::d
///    likes::d= =d v"""
///    lexicon = Lexicon(grammar)
///    strings = [str(p) for p in lexicon.generate_grammar("v")]
///    assert strings == [
///        "John runs",
///        "Mary runs",
///        "Mary likes John",
///        "John likes John",
///        "John likes Mary",
///        "Mary likes Mary",
///    ]
///
///Creating a lexicon with interpretations and getting the interpretation of a sentence.
///
///.. code-block:: python
///
///    grammar = """John::d::a_John
///    run::=d v::lambda a x some_e(e, pe_run(e), AgentOf(x,e))
///    Mary::d::a_Mary
///    likes::d= =d v::lambda a x lambda a y some_e(e, pe_likes(e), AgentOf(y,e) & PatientOf(x, e))"""
///        semantic_lexicon = Lexicon(grammar)
///        assert semantic_lexicon.is_semantic()
///        s = semantic_lexicon.parse("John likes Mary", "v")
///        assert len(s) == 1
///        parse = s[0]
///        assert parse.meaning is not None
///        assert parse.meaning == [
///            "some_e(x, pe_likes(x), AgentOf(a_John, x) & PatientOf(a_Mary, x))"
///        ]
///    
struct PyLexicon {
    word_id: TokenMap,
    lexeme_to_id: HashMap<LexicalEntry<&'static str, &'static str>, LexemeId>,
    lemma_to_id: HashMap<Pronounciation<&'static str>, Vec<LexemeId>>,

    //Has to be last bc of the static strs elsewhere
    lexicon: SelfOwningLexicon,
}

impl Display for PyLexicon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MGLexicon{{\n{}\n}}", self.lexicon)
    }
}

impl PyLexicon {
    fn semantics<'a>(&'a self) -> Option<&'a SemanticLexicon<'a, &'a str, &'a str>> {
        self.lexicon.semantic_lexicon()
    }

    fn backing_string(&self) -> &Arc<String> {
        &self.lexicon.string
    }
}

#[pyclass]
struct GrammarIterator {
    generator: Generator<Lexicon<String, String>, String, String>,
    max_strings: Option<usize>,
    n_strings: usize,
    lexicon: Py<PyLexicon>,
}

#[pymethods]
impl GrammarIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PySyntacticStructure> {
        if let Some(n) = slf.max_strings
            && slf.n_strings >= n
        {
            return None;
        }

        if let Some((prob, string, rules)) = slf.generator.next() {
            slf.n_strings += 1;
            let py = slf.py();
            Some(PySyntacticStructure::new(
                slf.lexicon.clone_ref(py),
                prob,
                string,
                rules,
            ))
        } else {
            None
        }
    }
}

#[pyclass(name = "Continuation", str, eq, frozen, hash)]
#[derive(Debug, Eq, PartialEq, Hash)]
///A class to represent a possible continuation of a string according to some grammar.
struct PyContinuation(Continuation<String>);

impl Display for PyContinuation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Continuation::Word(s) => write!(f, "{s}"),
            Continuation::AffixedWord(s) => write!(f, "{}", s.join("-")),
            Continuation::EndOfSentence => write!(f, "[EOS]"),
        }
    }
}

#[pymethods]
impl PyContinuation {
    #[new]
    fn new(s: &str) -> Self {
        match s {
            "[EOS]" => PyContinuation(Continuation::EndOfSentence),
            _ => PyContinuation(Continuation::Word(s.to_string())),
        }
    }

    fn __repr__(&self) -> String {
        format!("Continuation({self})")
    }

    #[staticmethod]
    #[allow(non_snake_case)]
    ///Get the EndOfSentence marker
    ///
    ///Returns
    ///-------
    ///:meth:`python_mg.Continuation`
    ///    A continuation marking the end of a sentence. Equivalent to `Continuation("[EOS]")`
    fn EOS() -> PyContinuation {
        PyContinuation(Continuation::EndOfSentence)
    }

    ///Checks if a continuation is the end of string marker.
    ///
    ///Returns
    ///-------
    ///bool
    ///    ``True`` if the continuation is the end of string marker, else ``False``.
    fn is_end_of_string(&self) -> bool {
        matches!(self, PyContinuation(Continuation::EndOfSentence))
    }

    ///Checks if a continuation is a word
    ///
    ///Returns
    ///-------
    ///bool
    ///    ``True`` if the continuation is a word, else ``False``.
    fn is_word(&self) -> bool {
        matches!(self, PyContinuation(Continuation::Word(_)))
    }

    ///Checks if a continuation is a multi-word, or an affixed string (as the result of head
    ///movement)
    ///
    ///Returns
    ///-------
    ///bool
    ///    ``True`` if the continuation is a multi-word, else ``False``.
    fn is_multi_word(&self) -> bool {
        matches!(self, PyContinuation(Continuation::AffixedWord(_)))
    }
}

fn map_string(s: &str) -> Vec<PhonContent<&str>> {
    match s.trim() {
        "" => vec![],
        _ => s
            .split(' ')
            .map(|x| {
                let x = x.split('-').collect::<Vec<_>>();
                if x.len() == 1 {
                    PhonContent::Normal(*x.first().unwrap())
                } else {
                    PhonContent::Affixed(x)
                }
            })
            .collect(),
    }
}

fn get_config(
    min_log_prob: Option<f64>,
    move_prob: f64,
    max_steps: Option<usize>,
    n_beams: Option<usize>,
) -> anyhow::Result<ParsingConfig> {
    let mut config = ParsingConfig::empty()
        .with_move_prob(LogProb::from_raw_prob(move_prob).map_err(|x| anyhow!(x.to_string()))?);

    if let Some(min_log_prob) = min_log_prob {
        config = config
            .with_min_log_prob(LogProb::new(min_log_prob).map_err(|x| anyhow!(x.to_string()))?);
    }
    if let Some(max_steps) = max_steps {
        config = config.with_max_steps(max_steps);
    }
    if let Some(n_beams) = n_beams {
        config = config.with_max_beams(n_beams);
    }
    Ok(config)
}

impl PyLexicon {
    fn from_lexicon(lexicon: SelfOwningLexicon) -> PyResult<Self> {
        //unsafe here because the lexicon has the lifetime of the reference of the SelfOwningLexicon.
        //We are owning it in the arc, so we have to make sure we can refer to it.

        let lexeme_to_id: HashMap<_, LexemeId> = lexicon
            .lexicon()
            .lexemes_and_ids()
            .map_err(|e| anyhow!(e))?
            .map(|(id, entry)| {
                let entry: LexicalEntry<&'static str, &'static str> =
                    unsafe { std::mem::transmute(entry) };
                (entry, id)
            })
            .collect();

        let mut lemma_to_id = HashMap::default();
        let mut word_id = TokenMap::default();

        for leaf in lexicon.lexicon().leaves().iter().copied() {
            let lemma = lexicon
                .lexicon()
                .leaf_to_lemma(leaf)
                .expect("Invalid lexicon!");

            let lemma: Pronounciation<&'static str> = unsafe { std::mem::transmute(*lemma) };

            if let Pronounciation::Pronounced(word) = lemma.as_ref() {
                word_id.add_word(word);
            }
            lemma_to_id.entry(lemma).or_insert(vec![]).push(leaf);
        }

        Ok(PyLexicon {
            lexicon,
            word_id,
            lexeme_to_id,
            lemma_to_id,
        })
    }
}

impl PyLexicon {
    #[allow(clippy::too_many_arguments)]
    fn inner_parse(
        slf: &Bound<'_, Self>,
        s: &[PhonContent<&str>],
        category: String,
        min_log_prob: Option<f64>,
        move_prob: f64,
        max_steps: Option<usize>,
        n_beams: Option<usize>,
        max_parses: Option<usize>,
    ) -> PyResult<Vec<PySyntacticStructure>> {
        let lex = slf.borrow();
        let config = get_config(min_log_prob, move_prob, max_steps, n_beams)?;
        let parser = lex
            .lexicon
            .lexicon()
            .parse(s, category.as_str(), &config)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        //     let self_ref: Py<Self> = slf.clone().into_pyobject(py).unwrap().into();

        if let Some(max_parses) = max_parses {
            Ok(parser
                .take(max_parses)
                .map(|(prob, string, rules)| {
                    PySyntacticStructure::into_syntax_structure(slf, prob, string, rules)
                })
                .collect())
        } else {
            Ok(parser
                .map(|(prob, string, rules)| {
                    PySyntacticStructure::into_syntax_structure(slf, prob, string, rules)
                })
                .collect())
        }
    }
}

#[pymethods]
impl PyLexicon {
    ///Check if this lexicon has semantics
    fn is_semantic(&self) -> bool {
        matches!(self.lexicon.lexicon, PossiblySemanticLexicon::Semantic(_))
    }

    fn __getnewargs__(&self) -> (String,) {
        (self.lexicon.to_string(),)
    }

    ///Gets the model description length of this lexicon. The precise calculation is described in `Deconstructing syntactic generalizations with minimalist grammars <https://aclanthology.org/2021.conll-1.34/>`_ (Ermolaeva, CoNLL 2021)
    ///
    ///Parameters
    ///----------
    ///n_phonemes : int
    ///    The number of phonemes that are possible in the phonology of the grammar (e.g. how many
    ///    letters)
    ///
    ///Returns
    ///-------
    ///float
    ///    the MDL of the lexicon.
    fn mdl(&self, n_phonemes: u16) -> PyResult<f64> {
        self.lexicon
            .lexicon()
            .mdl_score(n_phonemes)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[pyo3(signature = (prefix, category, min_log_prob=None, move_prob=0.5, max_steps=64, n_beams=None))]
    ///Compute valid next string for a prefix string.
    ///
    ///Parameters
    ///----------
    ///prefix : str
    ///    A prefix string to be continued
    ///category : str
    ///    The syntactic category of the parsed string
    ///min_log_prob : float or None, optional
    ///    Minimum log probability threshold for the parser to consider
    ///    Default is None.
    ///move_prob : float, optional
    ///    Probability of preferring a move over a merge when parsing.
    ///    Default is 0.5
    ///max_steps : int or None, optional
    ///    Maximum number of derivation steps. If None, will not be limited.
    ///    Default is 64.
    ///n_beams : int or None, optional
    ///    Number of beams to maintain while parsing. If none, will not be limited.
    ///    Default is None.
    ///Returns
    ///-------
    ///set of Continuation
    ///    Set indicating the next possible word, affixed word or whether the
    ///    sentence can be ended.
    fn continuations(
        &self,
        prefix: &str,
        category: String,
        min_log_prob: Option<f64>,
        move_prob: f64,
        max_steps: Option<usize>,
        n_beams: Option<usize>,
    ) -> PyResult<HashSet<PyContinuation>> {
        let config = get_config(min_log_prob, move_prob, max_steps, n_beams)?;
        let prefix = map_string(prefix);

        Ok(self
            .lexicon
            .lexicon()
            .valid_continuations(&category.as_str(), &prefix, &config)
            .map_err(|e| PyValueError::new_err(e.to_string()))?
            .into_iter()
            .map(|x| {
                PyContinuation(match x {
                    Continuation::Word(x) => Continuation::Word(x.to_string()),
                    Continuation::AffixedWord(items) => Continuation::AffixedWord(
                        items.into_iter().map(|x| x.to_string()).collect(),
                    ),
                    Continuation::EndOfSentence => Continuation::EndOfSentence,
                })
            })
            .collect())
    }

    #[staticmethod]
    ///Generates a random lexicon with random categories.
    ///
    ///Returns
    ///-------
    ///:meth:`python_mg.Lexicon`
    ///    a random Lexicon
    fn random_lexicon(lemmas: Vec<String>) -> PyResult<Self> {
        let mut rng = rand::rng();
        let lexicon: Lexicon<_, u16> = Lexicon::random(&0, &lemmas, None, &mut rng);
        let lexicon = lexicon.remap_lexicon(Clone::clone, ToString::to_string);
        let lex_s = lexicon.to_string();
        PyLexicon::from_lexicon(SelfOwningLexicon::new(lex_s)?)
    }

    #[pyo3(signature = (category, min_log_prob=None, move_prob=0.5, max_steps=64, n_beams=None, max_strings=None))]
    ///Generates all strings for the lexicon, without paying attention to their SyntacticStructure.
    ///This differs from :meth:`python_mg.Lexicon.generate_grammar` as different parses will be
    ///collapsed, and only strings will be returned.
    ///
    ///Parameters
    ///----------
    ///category : str
    ///    The syntactic category to be generated.
    ///min_log_prob : float or None, optional
    ///    Minimum log probability threshold to be generated.
    ///    If none, there is no limit on log probability.
    ///    Default is None.
    ///move_prob : float, optional
    ///    Probability of preferring a move over a merge when parsing.
    ///    Default is 0.5
    ///max_steps : int or None, optional
    ///    Maximum number of derivation steps. If None, will not be limited.
    ///    Default is 64.
    ///n_beams : int or None, optional
    ///    Number of beams to maintain while parsing. If None, will not be limited.
    ///    Default is None.
    ///max_strings : int or None, optional
    ///    Number of strings to generate before stopping.
    ///    Default is None.
    ///Returns
    ///-------
    ///list[tuple[list[str], float]]
    ///    The list of all strings along with their log probability
    fn generate_unique_strings(
        &self,
        category: &str,
        min_log_prob: Option<f64>,
        move_prob: f64,
        max_steps: Option<usize>,
        n_beams: Option<usize>,
        max_strings: Option<usize>,
    ) -> PyResult<Vec<(Vec<String>, f64)>> {
        let config = get_config(min_log_prob, move_prob, max_steps, n_beams)?;
        let mut hashmap = HashMap::new();
        for (prob, string, _) in self
            .lexicon
            .lexicon()
            .generate(category, &config)
            .map_err(|e| PyValueError::new_err(e.to_string()))?
        {
            hashmap
                .entry(string)
                .and_modify(|old_log_prob: &mut LogProb<f64>| {
                    *old_log_prob = old_log_prob.add_log_prob_clamped(prob);
                })
                .or_insert(prob);

            if let Some(max_strings) = max_strings
                && hashmap.len() > max_strings
            {
                break;
            }
        }

        let mut values = hashmap.into_iter().collect::<Vec<_>>();
        values.sort_by_key(|x| x.1);
        Ok(values
            .into_iter()
            .map(|(s, p)| {
                (
                    s.into_iter()
                        .map(|x| match x {
                            PhonContent::Normal(s) => s.to_string(),
                            PhonContent::Affixed(items) => items.join("-"),
                        })
                        .collect(),
                    p.into_inner(),
                )
            })
            .collect())
    }

    #[pyo3(signature = (category, min_log_prob=None, move_prob=0.5, max_steps=64, n_beams=None, max_strings=None))]
    ///Generates all syntactic structures for the lexicon.
    ///
    ///Parameters
    ///----------
    ///category : str
    ///    The syntactic category to be generated.
    ///min_log_prob : float or None, optional
    ///    Minimum log probability threshold to be generated.
    ///    If none, there is no limit on log probability.
    ///    Default is None.
    ///move_prob : float, optional
    ///    Probability of preferring a move over a merge when parsing.
    ///    Default is 0.5
    ///max_steps : int or None, optional
    ///    Maximum number of derivation steps. If None, will not be limited.
    ///    Default is 64.
    ///n_beams : int or None, optional
    ///    Number of beams to maintain while parsing. If None, will not be limited.
    ///    Default is None.
    ///max_strings : int or None, optional
    ///    Number of strings to generate before stopping.
    ///    Default is None.
    ///
    ///Returns
    ///-------
    ///an iterator which yields all parses as they are found
    fn generate_grammar(
        slf: PyRef<'_, Self>,
        category: String,
        min_log_prob: Option<f64>,
        move_prob: f64,
        max_steps: Option<usize>,
        n_beams: Option<usize>,
        max_strings: Option<usize>,
    ) -> PyResult<GrammarIterator> {
        let config = get_config(min_log_prob, move_prob, max_steps, n_beams)?;

        let py = slf.py();
        Ok(GrammarIterator {
            generator: slf
                .lexicon
                .lexicon()
                .clone()
                .remap_lexicon(|x| x.to_string(), |y| y.to_string())
                .into_generate(category, &config)
                .map_err(|e| anyhow!(e))?,
            max_strings,
            lexicon: slf.into_pyobject(py).unwrap().into(),
            n_strings: 0,
        })
    }

    #[expect(clippy::too_many_arguments)]
    #[pyo3(signature = (s, category, min_log_prob=None, move_prob=0.5, max_steps=64, n_beams=None, max_parses=None))]
    ///Parses a string and returns all found parses in a list
    ///The string, s, should be delimited by spaces for words and hyphens for multi-word expressions from head-movement
    ///
    ///Parameters
    ///----------
    ///s: str
    ///    A string to be parsed
    ///category : str
    ///    The syntactic category of the parsed string
    ///min_log_prob : float or None, optional
    ///    Minimum log probability threshold for the parser to consider
    ///    If none, there is no limit on log probability.
    ///    Default is None.
    ///move_prob : float, optional
    ///    Probability of preferring a move over a merge when parsing.
    ///    Default is 0.5
    ///max_steps : int or None, optional
    ///    Maximum number of derivation steps. If None, will not be limited.
    ///    Default is 64.
    ///n_beams : int or None, optional
    ///    Number of beams to maintain while parsing. If None, will not be limited.
    ///    Default is None.
    ///Returns
    ///-------
    ///list of SyntacticStructure
    ///    All found parses of the string.
    fn parse(
        slf: &Bound<'_, Self>,
        s: &str,
        category: String,
        min_log_prob: Option<f64>,
        move_prob: f64,
        max_steps: Option<usize>,
        n_beams: Option<usize>,
        max_parses: Option<usize>,
    ) -> PyResult<Vec<PySyntacticStructure>> {
        let s = map_string(s);
        PyLexicon::inner_parse(
            slf,
            &s,
            category,
            min_log_prob,
            move_prob,
            max_steps,
            n_beams,
            max_parses,
        )
    }

    #[new]
    fn new(grammar: String) -> PyResult<PyLexicon> {
        PyLexicon::from_lexicon(SelfOwningLexicon::new(grammar)?)
    }
}

/// A Python module implemented in Rust.
#[pymodule]
#[pyo3(name = "_lib_name")]
fn python_mg(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLexicon>()?;
    m.add_class::<PyContinuation>()?;
    m.add_class::<PySyntacticStructure>()?;
    m.add_class::<PyMgNode>()?;
    m.add_class::<PyMgEdge>()?;
    m.add_class::<PyScenario>()?;
    m.add_class::<PyScenarioGenerator>()?;
    m.add_class::<PyActor>()?;
    m.add_class::<PyEvent>()?;
    m.add_class::<PyPossibleEvent>()?;
    m.add_class::<PyMeaning>()?;
    Ok(())
}
