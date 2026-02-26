use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt::Display,
    hash::Hash,
    sync::Arc,
    time::Duration,
};

use itertools::Itertools;
use pyo3::{exceptions::PyValueError, prelude::*};
use simple_semantics::{
    Entity, EventType, LanguageResult, PossibleEvent, Scenario, ScenarioIterator, ThetaRoles,
    lambda::RootedLambdaPool,
    language::{ExecutionConfig, Expr},
};

pub mod lot_types;
use lot_types::{PyActor, PyEvent, convert_to_py_actor, convert_to_py_event};
pub mod scenario;
use scenario::PyScenario;

/// A language of thought expression that has been parsed.
///
/// You can always use a string instead of this class, but
/// this class allows you to save time on parsing the LOT expression if you use it a lot.
///
/// Parameters
/// ----------
/// s : str
///     A Language of Thought Expression
#[pyclass(name = "Meaning", eq, from_py_object, frozen, str)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PyMeaning {
    expr: RootedLambdaPool<'static, Expr<'static>>,
    string: Arc<String>,
}

impl PyMeaning {
    fn expr<'a>(&'a self) -> &'a RootedLambdaPool<'a, Expr<'a>> {
        &self.expr
    }

    pub unsafe fn from_other(expr: RootedLambdaPool<'_, Expr<'_>>, s: &Arc<String>) -> Self {
        let expr: RootedLambdaPool<'static, Expr<'static>> = unsafe { std::mem::transmute(expr) };

        Self {
            expr,
            string: s.clone(),
        }
    }
}

impl Display for PyMeaning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.expr)
    }
}

#[pymethods]
impl PyMeaning {
    #[new]
    fn new(expr: String) -> PyResult<Self> {
        let string = Arc::new(expr);
        let s: &'static str = unsafe { std::mem::transmute(string.as_str()) };
        let expr = RootedLambdaPool::parse(s).map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(Self { expr, string })
    }

    fn __repr__(&self) -> String {
        format!("Meaning({self})")
    }
}

#[derive(FromPyObject)]
enum MeaningOrString {
    #[pyo3(transparent, annotation = "Meaning")]
    Meaning(PyMeaning),
    #[pyo3(transparent, annotation = "str")]
    String(String),
}

impl MeaningOrString {
    fn into_meaning(self) -> PyResult<PyMeaning> {
        match self {
            MeaningOrString::Meaning(meaning) => Ok(meaning),
            MeaningOrString::String(s) => PyMeaning::new(s),
        }
    }
}

#[derive(IntoPyObject)]
enum OwnedLanguageResult {
    Bool(bool),
    Actor(PyActor),
    Event(PyEvent),
    ActorSet(HashSet<PyActor>),
    EventSet(HashSet<PyEvent>),
}

impl OwnedLanguageResult {
    fn new(language_result: LanguageResult, scenario: &Scenario) -> PyResult<Self> {
        Ok(match language_result {
            LanguageResult::Bool(bool) => OwnedLanguageResult::Bool(bool),
            LanguageResult::Actor(name) => {
                OwnedLanguageResult::Actor(convert_to_py_actor(name, scenario))
            }
            LanguageResult::Event(e_i) => {
                OwnedLanguageResult::Event(convert_to_py_event(e_i, scenario)?)
            }
            LanguageResult::ActorSet(items) => OwnedLanguageResult::ActorSet(
                items
                    .into_iter()
                    .map(|name| convert_to_py_actor(name, scenario))
                    .collect(),
            ),
            LanguageResult::EventSet(items) => OwnedLanguageResult::EventSet(
                items
                    .into_iter()
                    .map(|e_i| convert_to_py_event(e_i, scenario))
                    .collect::<Result<HashSet<_>, _>>()?,
            ),
        })
    }
}

impl PyScenario {
    fn execute<'a>(
        &'a self,
        mut expr: RootedLambdaPool<'a, Expr<'a>>,
        config: Option<ExecutionConfig>,
    ) -> PyResult<OwnedLanguageResult> {
        let scenario = self.as_scenario();
        expr.reduce()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        expr.cleanup();

        let pool = expr
            .into_pool()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        let language_result = pool
            .run(&scenario, config)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        OwnedLanguageResult::new(language_result, &scenario)
    }
}

#[pymethods]
impl PyScenario {
    ///Parse a scenario from a string description:
    ///
    ///Parameters
    ///----------
    ///s : str
    ///    The description of the scenario.
    ///
    ///Raises
    ///------
    ///ValueError
    ///    If the expression is not a valid description of a scenario
    #[staticmethod]
    fn from_str(s: String) -> PyResult<Self> {
        let scenario =
            Scenario::parse(s.as_str()).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(scenario.into())
    }

    fn __repr__(&self) -> String {
        format!("Scenario({self})")
    }

    ///Executes an language of thought expression in this scenario. Will potentially throw a PresuppositionException if
    ///something is referenced that isn't in the scenario. It will also reduce any lambda
    ///expressions if possible, and then will only execute the expression if it is fully reducible.
    ///
    ///Parameters
    ///----------
    ///expression : Meaning | str
    ///    The expression in the language of thought to execute.
    ///max_steps : int or None, optional
    ///    The number of steps in the virtual machine to execute before giving up.
    ///    Default is 256.
    ///timeout : datetime.timedelta or None, optional
    ///    The amount of time before the execution gives up.
    ///    Default is None
    ///Returns
    ///-------
    ///bool or Actor or Event or set[Actor] or set[Event]
    ///    the value of the expression
    ///Raises
    ///------
    ///ValueError
    ///    If the expression is a string which is incorrectly formatted or if there is a presupposition error.
    #[pyo3(signature = (expression, max_steps=64, timeout=None))]
    fn evaluate(
        &self,
        expression: MeaningOrString,
        max_steps: Option<usize>,
        timeout: Option<Duration>,
    ) -> PyResult<OwnedLanguageResult> {
        self.execute(
            expression.into_meaning()?.expr().clone(),
            Some(ExecutionConfig::new(max_steps, timeout).allow_empty_quantification()),
        )
    }

    ///Creates a generator that goes over all possible scenarios that can be generated according to
    ///the its parameters. This gets very large very quickly.
    ///
    ///Parameters
    ///----------
    ///actors : list[str]
    ///    The actors who may or may not be present.
    ///event_kinds : list[``PossibleEvent``]
    ///    The possible kinds of events
    ///
    ///Returns
    ///-------
    ///ScenarioGenerator
    #[staticmethod]
    fn all_scenarios(
        actors: Vec<String>,
        event_kinds: Vec<PyPossibleEvent>,
        actor_properties: Vec<String>,
    ) -> PyScenarioGenerator {
        let parameter_holder = Arc::new(ParameterHolder {
            actors,
            event_kinds,
            actor_properties,
        });

        let actors: Vec<&'static str> = parameter_holder
            .actors
            .iter()
            .map(|x| {
                let s: &'static str = unsafe { std::mem::transmute(x.as_str()) };
                s
            })
            .collect::<Vec<_>>();
        let properties: Vec<&'static str> = parameter_holder
            .actor_properties
            .iter()
            .map(|x| {
                let s: &'static str = unsafe { std::mem::transmute(x.as_str()) };
                s
            })
            .collect::<Vec<_>>();

        let event_kinds: Vec<PossibleEvent<'static>> = parameter_holder
            .event_kinds
            .iter()
            .map(|x| {
                let x = x.as_possible_event();
                let x: PossibleEvent<'static> = unsafe { std::mem::transmute(x) };
                x
            })
            .collect::<Vec<_>>();

        PyScenarioGenerator {
            generator: Scenario::all_scenarios(&actors, &event_kinds, &properties),
            _parameter_holder: parameter_holder,
        }
    }
}

/// A possible linguistic event with theta role structure.
///
/// Parameters
/// ----------
/// name : str
///     Identifier for the event.
/// has_agent : bool, optional
///     Whether the event has an agent participant. Default is ``True``.
/// has_patient : bool, optional
///     Whether the event has a patient participant. Default is ``False``.
/// is_reflexive : bool, optional
///     Whether the event allows reflexive construal. Default is ``True``.
#[pyclass(name = "PossibleEvent", eq, get_all, set_all, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct PyPossibleEvent {
    ///Whether the event takes an agent
    pub has_agent: bool,
    ///Whether the event takes a patient
    pub has_patient: bool,
    ///Whether the event can have the same agent and patient
    pub is_reflexive: bool,
    ///The name of this kind of event (e.g. `running` could be a unaccusative event)
    pub name: String,
}

#[pymethods]
impl PyPossibleEvent {
    #[new]
    #[pyo3(signature = (name, has_agent=true, has_patient=false, is_reflexive=true))]
    fn new(name: String, has_agent: bool, has_patient: bool, is_reflexive: bool) -> Self {
        PyPossibleEvent {
            name,
            has_agent,
            has_patient,
            is_reflexive,
        }
    }

    /// Classify the event based on its argument structure.
    ///
    /// Returns
    /// -------
    /// Literal['Transitive', 'TransitiveNonReflexive', 'Unergative', 'Unaccusative', 'Avalent'].
    fn event_type(&self) -> &'static str {
        match (self.has_agent, self.has_patient) {
            (true, true) if self.is_reflexive => "Transitive",
            (true, true) => "TransitiveNonReflexive",
            (true, false) => "Unergative",
            (false, true) => "Unaccusative",
            (false, false) => "Avalent",
        }
    }
}

impl PyPossibleEvent {
    fn as_event_type(&self) -> EventType {
        match (self.has_agent, self.has_patient) {
            (true, true) if self.is_reflexive => EventType::Transitive,
            (true, true) => EventType::TransitiveNonReflexive,
            (true, false) => EventType::Unergative,
            (false, true) => EventType::Unaccusative,
            (false, false) => EventType::Avalent,
        }
    }

    fn as_possible_event<'a>(&'a self) -> PossibleEvent<'a> {
        PossibleEvent {
            label: self.name.as_str(),
            event_type: self.as_event_type(),
        }
    }
}

///Yields
///------
///Scenario
///    Another scenario that can be generated according to the parameters.
///
#[pyclass(name = "ScenarioGenerator", eq, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct PyScenarioGenerator {
    generator: ScenarioIterator<'static>,
    _parameter_holder: Arc<ParameterHolder>,
}

#[pymethods]
impl PyScenarioGenerator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyScenario> {
        slf.generator.next().map(|s| s.into())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
struct ParameterHolder {
    actors: Vec<String>,
    event_kinds: Vec<PyPossibleEvent>,
    actor_properties: Vec<String>,
}
