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
    lambda::{FreeVar, RootedLambdaPool},
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
#[pyclass(
    name = "Meaning",
    module = "python_mg.semantics",
    eq,
    from_py_object,
    str,
    frozen
)]
#[derive(Debug, Clone)]
pub struct PyMeaning {
    expr: RootedLambdaPool<'static, Expr<'static>>,
    strings: Vec<Arc<String>>,
}

impl PartialEq for PyMeaning {
    fn eq(&self, other: &Self) -> bool {
        self.expr == other.expr
    }
}

impl Eq for PyMeaning {}

impl PyMeaning {
    fn expr<'a>(&'a self) -> &'a RootedLambdaPool<'a, Expr<'a>> {
        &self.expr
    }

    pub unsafe fn from_other(expr: RootedLambdaPool<'_, Expr<'_>>, s: Vec<Arc<String>>) -> Self {
        let expr: RootedLambdaPool<'static, Expr<'static>> = unsafe { std::mem::transmute(expr) };

        Self {
            expr,
            strings: s.clone(),
        }
    }
}

impl Display for PyMeaning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.expr)
    }
}

#[derive(FromPyObject)]
enum IntOrStr {
    #[pyo3(transparent, annotation = "int")]
    Int(usize),
    #[pyo3(transparent, annotation = "str")]
    Str(String),
}

#[pymethods]
impl PyMeaning {
    #[new]
    fn new(expr: String) -> PyResult<Self> {
        let string = Arc::new(expr);
        let s: &'static str = unsafe { std::mem::transmute(string.as_str()) };
        let expr = RootedLambdaPool::parse(s).map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(Self {
            expr,
            strings: vec![string],
        })
    }

    fn __getnewargs__(&self) -> (String,) {
        (self.expr.to_string(),)
    }

    ///Binds a free variable
    ///
    ///
    ///Examples
    ///--------
    ///
    ///Binding a free variable with a string.
    ///
    ///.. code-block:: python
    ///
    ///    psi = Meaning("pa_nice(Johnny#a) & pa_friendly(Johnny#a)") # "Johnny#a" is a free variable.
    ///    x = psi.bind_free_variable("Johnny", "a_John")
    ///    assert x == Meaning("pa_nice(a_John) & pa_friendly(a_John)")
    ///
    ///Binding a free variable with an integer.
    ///
    ///.. code-block:: python
    ///
    ///    psi = Meaning("pa_nice(343#a) & pa_friendly(343#a)") # "343#a" is an integer free variable.
    ///    x = psi.bind_free_variable(343, "a_John")
    ///    assert x == Meaning("pa_nice(a_John) & pa_friendly(a_John)")
    ///
    ///Parameters
    ///----------
    ///free_var : str | int
    ///    The name (or int) of the free variables
    ///value : Meaning | str
    ///    The value of the free variable.
    ///reduce : bool
    ///    Whether to reduce immediately after application or not (true by default)
    ///
    ///Returns
    ///-------
    ///Meaning
    ///    The resulting meaning after binding the free variable.
    ///
    ///Raises
    ///------
    ///ValueError
    ///    If the free variable's expression is of the wrong type if the meaning is an
    ///    unparseable string.
    #[pyo3(signature = (free_var, value, reduce=true))]
    fn bind_free_variable(
        &self,
        free_var: IntOrStr,
        value: MeaningOrString,
        reduce: bool,
    ) -> PyResult<PyMeaning> {
        let mut phi = self.clone();
        let PyMeaning { expr: psi, strings } = value.into_meaning()?;
        let fvar = match free_var {
            IntOrStr::Int(x) => FreeVar::Anonymous(x),
            IntOrStr::Str(string) => {
                let string = Arc::new(string);
                let s: &'static str = unsafe { std::mem::transmute(string.as_str()) };
                phi.strings.push(string);
                FreeVar::Named(s)
            }
        };

        phi.strings.extend(strings);
        phi.expr
            .bind_free_variable(fvar, psi)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        if reduce {
            phi.expr
                .reduce()
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            phi.expr.cleanup();
        }

        Ok(phi)
    }

    ///Applies psi to self.
    ///
    ///
    ///Examples
    ///--------
    ///
    ///Applying an argument to a function.
    ///
    ///.. code-block:: python
    ///
    ///    alpha = Meaning("lambda a x pa_nice(x) & pa_friendly(x)")
    ///    beta = Meaning("a_John")
    ///    assert Meaning("pa_nice(a_John) & pa_friendly(a_John)") == alpha.apply(beta)
    ///
    ///Parameters
    ///----------
    ///psi : Meaning | str
    ///    The argument that is to be applied.
    ///reduce : bool
    ///    Whether to reduce immediately after application or not (true by default)
    ///
    ///Returns
    ///-------
    ///Meaning
    ///    The resulting meaning after applying the argument
    ///
    ///Raises
    ///------
    ///ValueError
    ///    If the expression is of the wrong type or if the meaning is an
    ///    unparseable string.
    #[pyo3(signature = (psi, reduce=true))]
    fn apply(&self, psi: MeaningOrString, reduce: bool) -> PyResult<Option<PyMeaning>> {
        let PyMeaning {
            expr: psi,
            strings: psi_strings,
        } = psi.into_meaning()?;
        let PyMeaning {
            expr: phi,
            mut strings,
        } = self.clone();
        strings.extend(psi_strings);
        if let Some(mut phi) = phi.apply(psi) {
            if reduce {
                phi.reduce()
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                phi.cleanup();
            }
            //strings may grow monotonically but its unlikely to ever actually be an issue!
            Ok(Some(PyMeaning { expr: phi, strings }))
        } else {
            Ok(None)
        }
    }

    ///Reduces an expression.
    ///
    ///Returns
    ///-------
    ///Meaning
    ///    The resulting meaning after reduction.
    ///
    ///Raises
    ///------
    ///ValueError
    ///    If there is an error in how the meaning is constructed leading the reduction to fail.
    fn reduce(&self) -> PyResult<Self> {
        let mut phi = self.clone();
        phi.expr
            .reduce()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        phi.expr.cleanup();
        Ok(phi)
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
    ///Returns
    ///-------
    ///Scenario
    ///    The scenario described by the string.
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
    ///    Default is 64.
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
    ///    If the expression is a string which is incorrectly formatted.
    ///    If the expression's lambda terms cannot be fully reduced.
    ///    If there is a presupposition error.
    ///
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
    /// actor_properties : list[str]
    ///    The possible predicates that can apply to actors
    ///max_number_of_events : int | None
    ///    The maximum number of events in a given scenario (default is None, so unbounded)
    ///max_number_of_actors : int | None
    ///    The maximum number of actors in a given scenario (default is None, so unbounded)
    ///max_number_of_actor_properties : int | None
    ///    The maximum number of properties an actor can have in a given scenario (default is None, so unbounded)
    ///
    ///Returns
    ///-------
    ///ScenarioGenerator
    #[staticmethod]
    #[pyo3(signature = (actors, event_kinds, actor_properties, max_number_of_events=None, max_number_of_actors=None, max_number_of_actor_properties=None))]
    fn all_scenarios(
        actors: Vec<String>,
        event_kinds: Vec<PyPossibleEvent>,
        actor_properties: Vec<String>,
        max_number_of_events: Option<usize>,
        max_number_of_actors: Option<usize>,
        max_number_of_actor_properties: Option<usize>,
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
            generator: Scenario::all_scenarios(
                &actors,
                &event_kinds,
                &properties,
                max_number_of_events,
                max_number_of_actors,
                max_number_of_actor_properties,
            ),
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
#[pyclass(
    name = "PossibleEvent",
    module = "python_mg.semantics",
    eq,
    get_all,
    set_all,
    from_py_object
)]
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

    fn __getnewargs__(&self) -> (&str, bool, bool, bool) {
        (
            &self.name,
            self.has_agent,
            self.has_patient,
            self.is_reflexive,
        )
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
#[pyclass(name = "ScenarioGenerator", from_py_object)]
#[derive(Debug, Clone)]
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
