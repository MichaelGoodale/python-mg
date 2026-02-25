use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt::Display,
    hash::Hash,
    sync::Arc,
    time::Duration,
};

use itertools::Itertools;
use pyo3::{IntoPyObjectExt, exceptions::PyValueError, prelude::*};
use simple_semantics::{
    Entity, EventType, LanguageResult, PossibleEvent, Scenario, ScenarioIterator, ThetaRoles,
    lambda::RootedLambdaPool,
    language::{ExecutionConfig, Expr},
};

pub mod lot_types;
use lot_types::{PyActor, PyEvent, convert_to_py_actor, convert_to_py_event};
pub mod scenario;
use scenario::PyScenario;

struct LanguageResultWrapper<'a>(LanguageResult<'a>, Scenario<'a>);

impl<'py> IntoPyObject<'py> for LanguageResultWrapper<'_> {
    type Target = PyAny;

    type Output = Bound<'py, Self::Target>;

    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self.0 {
            LanguageResult::Bool(bool) => bool.into_bound_py_any(py),
            LanguageResult::Actor(name) => convert_to_py_actor(name, &self.1).into_bound_py_any(py),
            LanguageResult::Event(e_i) => convert_to_py_event(e_i, &self.1)?.into_bound_py_any(py),
            LanguageResult::ActorSet(items) => items
                .into_iter()
                .map(|name| convert_to_py_actor(name, &self.1))
                .collect::<HashSet<_>>()
                .into_bound_py_any(py),
            LanguageResult::EventSet(items) => items
                .into_iter()
                .map(|e_i| convert_to_py_event(e_i, &self.1))
                .collect::<Result<HashSet<_>, _>>()?
                .into_bound_py_any(py),
        }
    }
}

impl PyScenario {
    fn execute<'a>(
        &'a self,
        mut expr: RootedLambdaPool<'a, Expr<'a>>,
        config: Option<ExecutionConfig>,
    ) -> PyResult<LanguageResultWrapper<'a>> {
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
        Ok(LanguageResultWrapper(language_result, scenario))
    }
}

#[pymethods]
impl PyScenario {
    #[new]
    fn new(s: String) -> PyResult<Self> {
        let scenario =
            Scenario::parse(s.as_str()).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(scenario.into())
    }

    fn __repr__(&self) -> String {
        format!("Scenario({self})")
    }

    #[pyo3(signature = (expression, max_steps=64, timeout=None))]
    ///Executes an language of thought expression in this scenario. Will potentially throw a PresuppositionException if
    ///something is referenced that isn't in the scenario. It will also reduce any lambda
    ///expressions if possible, and then will only execute the expression if it is fully reducible.
    ///
    ///Parameters
    ///----------
    ///expression : str
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
    ///    The result of the language evaluation, typed according to the
    ///    expression's return kind:
    ///
    ///    - ``bool`` — a plain boolean value.
    ///    - ``Actor`` — a single actor resolved from the model.
    ///    - ``Event`` — a single event resolved from the model.
    ///    - ``set[Actor]`` — an unordered collection of actors.
    ///    - ``set[Event]`` — an unordered collection of events.
    ///
    ///Raises
    ///------
    ///PyErr
    ///    If conversion of an ``Event`` or ``EventSet`` variant fails.
    fn evaluate<'a>(
        &'a self,
        expression: &'a str,
        max_steps: Option<usize>,
        timeout: Option<Duration>,
    ) -> PyResult<LanguageResultWrapper<'a>> {
        let expr = RootedLambdaPool::parse(expression)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.execute(expr, Some(ExecutionConfig::new(max_steps, timeout)))
    }

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

#[pyclass(name = "PossibleEvent", eq, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct PyPossibleEvent {
    #[pyo3(get, set)]
    pub has_agent: bool,
    #[pyo3(get, set)]
    pub has_patient: bool,
    pub is_reflexive: bool,
    #[pyo3(get, set)]
    pub name: String,
}

impl PyPossibleEvent {
    fn event_type(&self) -> EventType {
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
            event_type: self.event_type(),
        }
    }
}

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
