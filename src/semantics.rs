use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    hash::Hash,
    sync::Arc,
    time::Duration,
};

use itertools::Itertools;
use pyo3::{
    exceptions::{PyDeprecationWarning, PyTypeError, PyValueError},
    prelude::*,
};
use simple_semantics::{
    Entity, EventType, PossibleEvent, Scenario, ScenarioIterator, ThetaRoles,
    lambda::{FreeVar, RootedLambdaPool, Value, types::LambdaType},
    language::Expr,
    owned::{OwnedExpr, OwnedLiteral, OwnedRootedLambdaPool, OwnedValue},
};

pub mod lot_types;
use lot_types::{PyActor, PyEvent};
pub mod scenario;
use scenario::PyScenario;

use crate::semantics::lot_types::PyLambdaType;

/// A language of thought expression that has been parsed.
///
/// You can always use a string instead of this class, but
/// this class allows you to save time on parsing the LOT expression if you use it a lot.
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
    expr: OwnedRootedLambdaPool<OwnedExpr>,
}

impl PyMeaning {
    pub(crate) fn new_parsed(expr: RootedLambdaPool<Expr>) -> Self {
        Self {
            expr: expr.into_owned(),
        }
    }
}

impl PartialEq for PyMeaning {
    fn eq(&self, other: &Self) -> bool {
        self.expr == other.expr
    }
}

impl Eq for PyMeaning {}

impl PyMeaning {
    fn expr<'a>(&'a self) -> RootedLambdaPool<'a, Expr<'a>> {
        self.expr.as_borrowed()
    }
}

impl Display for PyMeaning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.expr.as_borrowed())
    }
}

#[derive(FromPyObject, IntoPyObject, PartialEq, Eq, PartialOrd, Ord)]
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
        let string: Arc<str> = expr.into();
        let s: &'static str = unsafe { std::mem::transmute(&*string) };
        let expr: RootedLambdaPool<Expr> =
            RootedLambdaPool::parse(s).map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(Self {
            expr: expr.into_owned(),
        })
    }

    fn __getnewargs__(&self) -> (String,) {
        (self.expr.as_borrowed().to_string(),)
    }

    ///Returns the type of the expression
    ///
    ///Returns
    ///-------
    ///LambdaType
    ///    The type of expression
    fn lambda_type(&self) -> PyLambdaType {
        PyLambdaType(self.expr.as_borrowed().get_type().unwrap())
    }

    ///Returns a dictionary of all free variables in the Meaning.
    ///
    ///Returns
    ///-------
    ///dict of {int or str, LambdaType}
    ///    A dictionary of all free variables and their types.
    fn free_variables(&self) -> BTreeMap<IntOrStr, PyLambdaType> {
        let expr = self.expr.as_borrowed();

        expr.free_variables()
            .map(|(fvar, t)| {
                (
                    match fvar {
                        FreeVar::Named(s) => IntOrStr::Str(s.to_string()),
                        FreeVar::Anonymous(i) => IntOrStr::Int(*i),
                    },
                    PyLambdaType(t.clone()),
                )
            })
            .collect()
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
        let mut phi = self.expr.as_borrowed();

        let psi = value.as_expr()?;

        let fvar = match &free_var {
            IntOrStr::Int(x) => FreeVar::Anonymous(*x),
            IntOrStr::Str(string) => FreeVar::Named(string.as_str()),
        };

        phi.bind_free_variable(fvar, psi)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        if reduce {
            phi.reduce()
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            phi.cleanup();
        }

        Ok(PyMeaning {
            expr: phi.into_owned(),
        })
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
        let psi = psi.as_expr()?;
        let phi = self.expr.as_borrowed();
        if let Some(mut phi) = phi.apply(psi) {
            if reduce {
                phi.reduce()
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                phi.cleanup();
            }
            //strings may grow monotonically but its unlikely to ever actually be an issue!
            Ok(Some(PyMeaning {
                expr: phi.into_owned(),
            }))
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
        let mut phi = self.expr.as_borrowed();
        phi.reduce()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        phi.cleanup();
        Ok(PyMeaning {
            expr: phi.into_owned(),
        })
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
    fn as_expr<'a>(&'a self) -> PyResult<RootedLambdaPool<'a, Expr<'a>>> {
        match self {
            MeaningOrString::Meaning(meaning) => Ok(meaning.expr.as_borrowed()),
            MeaningOrString::String(s) => RootedLambdaPool::parse(s.as_str())
                .map_err(|x| PyValueError::new_err(x.to_string())),
        }
    }
}

#[pyclass(
    name = "LOTValue",
    module = "python_mg.semantics",
    eq,
    str,
    frozen,
    from_py_object
)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PyLotValue(OwnedValue<OwnedExpr>, LiteralExtras);

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
enum LiteralExtras {
    None,
    Actor(PyActor),
    Event(PyEvent),
    ActorSet(Vec<PyActor>),
    EventSet(Vec<PyEvent>),
}

impl PyLotValue {
    fn new(value: Value<'_, Expr>, scenario: &PyScenario) -> Self {
        let v = value.into_owned();

        let extras = match &v {
            OwnedValue::Base(OwnedLiteral::Actor(a)) => LiteralExtras::Actor(
                scenario
                    .actors
                    .iter()
                    .find(|x| &x.name == a)
                    .expect("Value's actor must be contained in scenario! (Rust library error)")
                    .clone(),
            ),

            OwnedValue::Base(OwnedLiteral::Event(e)) => LiteralExtras::Event(
                scenario
                    .events
                    .get(*e as usize)
                    .expect("Value's event must be contained in scenario! (Rust library error)")
                    .clone(),
            ),

            OwnedValue::Base(OwnedLiteral::ActorSet(a)) => LiteralExtras::ActorSet(
                scenario
                    .actors
                    .iter()
                    .filter(|x| a.iter().any(|n| &x.name == n))
                    .cloned()
                    .collect(),
            ),
            OwnedValue::Base(OwnedLiteral::EventSet(e)) => LiteralExtras::EventSet(
                e.iter()
                    .map(|e| {
                        scenario
                            .events
                            .get(*e as usize)
                            .expect("Value's events must be in the scenario! (Rust library error)")
                            .clone()
                    })
                    .collect(),
            ),
            _ => LiteralExtras::None,
        };

        PyLotValue(v, extras)
    }
}

#[pymethods]
impl PyLotValue {
    fn __bool__(&self) -> PyResult<bool> {
        self.as_bool()
    }

    ///Converts to a boolean
    ///
    ///Returns
    ///-------
    ///bool
    ///    The corresponding boolean of this LotValue.
    ///
    ///Raises
    ///------
    ///TypeError
    ///    If the value is not a raw boolean.
    fn as_bool(&self) -> PyResult<bool> {
        if let OwnedValue::Base(OwnedLiteral::Bool(b)) = self.0 {
            Ok(b)
        } else {
            Err(PyTypeError::new_err(format!(
                "{self} is not a raw boolean!"
            )))
        }
    }

    ///Converts to a Meaning
    ///
    ///Returns
    ///-------
    ///PyMeaning
    ///    This value as a Meaning
    fn as_meaning(&self) -> PyMeaning {
        PyMeaning {
            expr: self.0.as_borrowed().into_pool().into_owned(),
        }
    }

    ///Converts to a actor
    ///
    ///Returns
    ///-------
    ///Actor
    ///    The corresponding actor of this LotValue.
    ///
    ///Raises
    ///------
    ///TypeError
    ///    If the value is not a raw actor.
    fn as_actor(&self) -> PyResult<PyActor> {
        if let LiteralExtras::Actor(a) = &self.1 {
            Ok(a.clone())
        } else {
            Err(PyTypeError::new_err(format!("{self} is not a raw actor!")))
        }
    }

    ///Converts to a event
    ///
    ///Returns
    ///-------
    ///Event
    ///    The corresponding event of this LotValue.
    ///
    ///Raises
    ///------
    ///TypeError
    ///    If the value is not a raw event.
    fn as_event(&self) -> PyResult<PyEvent> {
        if let LiteralExtras::Event(a) = &self.1 {
            Ok(a.clone())
        } else {
            Err(PyTypeError::new_err(format!("{self} is not a raw event!")))
        }
    }

    ///Converts to a list[Actor]
    ///
    ///Returns
    ///-------
    ///list[Actor]
    ///    The corresponding list[Actor] of this LotValue.
    ///
    ///Raises
    ///------
    ///TypeError
    ///    If the value is not a raw set of actors.
    fn as_actor_set(&self) -> PyResult<Vec<PyActor>> {
        if let LiteralExtras::ActorSet(a) = &self.1 {
            Ok(a.clone())
        } else {
            Err(PyTypeError::new_err(format!(
                "{self} is not a raw actor set!"
            )))
        }
    }

    ///Converts to a list[Event]
    ///
    ///Returns
    ///-------
    ///list[Event]
    ///    The corresponding list[Event] of this LotValue.
    ///
    ///Raises
    ///------
    ///TypeError
    ///    If the value is not a raw set of events.
    fn as_event_set(&self) -> PyResult<Vec<PyEvent>> {
        if let LiteralExtras::EventSet(a) = &self.1 {
            Ok(a.clone())
        } else {
            Err(PyTypeError::new_err(format!(
                "{self} is not a raw event set!"
            )))
        }
    }
    ///Converts to a TruthTable
    ///
    ///Returns
    ///-------
    ///TruthTable
    ///    The corresponding TruthTable of this LotValue
    ///
    ///Raises
    ///------
    ///TypeError
    ///    If the value is not a raw <t,t>
    fn as_truth_table(&self) -> PyResult<PyTruthToTruth> {
        if let OwnedValue::Base(OwnedLiteral::TruthTable { on_false, on_true }) = self.0 {
            Ok(PyTruthToTruth { on_true, on_false })
        } else {
            Err(PyTypeError::new_err(format!(
                "{self} is not a raw truth to truth!"
            )))
        }
    }
}

impl Display for PyLotValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_borrowed())
    }
}

#[pyclass(
    name = "TruthToTruth",
    module = "python_mg.semantics",
    eq,
    str,
    frozen,
    from_py_object
)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PyTruthToTruth {
    #[pyo3(get)]
    on_true: bool,
    #[pyo3(get)]
    on_false: bool,
}

impl Display for PyTruthToTruth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{False → {}, True → {}}}", self.on_false, self.on_true)
    }
}

#[pymethods]
impl PyTruthToTruth {
    #[new]
    fn new(on_true: bool, on_false: bool) -> Self {
        Self { on_true, on_false }
    }

    fn __call__(&self, x: bool) -> bool {
        if x { self.on_true } else { self.on_false }
    }
}

impl PyScenario {
    fn execute<'a>(&'a self, mut expr: RootedLambdaPool<'a, Expr<'a>>) -> PyResult<PyLotValue> {
        let scenario = self.as_scenario();
        expr.reduce()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        expr.cleanup();

        expr.interp(&scenario)
            .map(|x| PyLotValue::new(x, self))
            .map_err(|e| PyValueError::new_err(e.to_string()))
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
    ) -> PyResult<PyLotValue> {
        if max_steps != Some(64) {
            Python::attach(|py| {
                let category = py.get_type::<PyDeprecationWarning>();
                PyErr::warn(
                    py,
                    &category,
                    c"`max_steps` is currently deprecated and will be ignored",
                    2,
                )
            })?;
        }

        if !timeout.is_none() {
            Python::attach(|py| {
                let category = py.get_type::<PyDeprecationWarning>();
                PyErr::warn(
                    py,
                    &category,
                    c"`timeout` is currently deprecated and will be ignored",
                    2,
                )
            })?;
        }

        self.execute(expression.into_meaning()?.expr().clone())
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
