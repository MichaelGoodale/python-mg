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

#[pyclass(name = "Scenario", str, eq, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PyScenario {
    #[pyo3(get, set)]
    actors: Vec<PyActor>,
    #[pyo3(get, set)]
    events: Vec<PyEvent>,
}

impl From<Scenario<'_>> for PyScenario {
    fn from(value: Scenario) -> Self {
        let actors = value
            .actors()
            .iter()
            .map(|x| PyActor {
                name: x.to_string(),
                properties: value
                    .properties()
                    .iter()
                    .filter_map(|(k, v)| {
                        if v.contains(&Entity::Actor(x)) {
                            Some(k.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
            })
            .collect();

        let events = value
            .thematic_relations()
            .iter()
            .enumerate()
            .map(|(i, x)| PyEvent {
                agent: x.agent.map(|x| x.to_string()),
                patient: x.patient.map(|x| x.to_string()),
                properties: value
                    .properties()
                    .iter()
                    .filter_map(|(k, v)| {
                        if v.contains(&Entity::Event(u8::try_from(i).expect("Too many events!"))) {
                            Some(k.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
            })
            .collect();

        PyScenario { actors, events }
    }
}

impl Display for PyScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_scenario())
    }
}

impl PyScenario {
    fn as_scenario<'a>(&'a self) -> Scenario<'a> {
        let actors = self.actors.iter().map(|x| x.name.as_str()).collect();
        let thematic_relations = self.events.iter().map(|x| x.into_theta_roles()).collect();
        let mut properties: BTreeMap<_, Vec<_>> = BTreeMap::new();

        for a in &self.actors {
            for p in &a.properties {
                properties
                    .entry(p.as_str())
                    .or_default()
                    .push(Entity::Actor(a.name.as_str()));
            }
        }
        for (i, e) in self.events.iter().enumerate() {
            for p in &e.properties {
                properties
                    .entry(p.as_str())
                    .or_default()
                    .push(Entity::Event(u8::try_from(i).expect("Too many events!")));
            }
        }

        Scenario::new(actors, thematic_relations, properties)
    }
}

struct LanguageResultWrapper<'a>(LanguageResult<'a>, Scenario<'a>);

fn convert_to_py_actor(name: &str, scenario: &Scenario<'_>) -> PyActor {
    PyActor {
        name: name.to_string(),
        properties: scenario
            .properties()
            .iter()
            .filter_map(|(prop, entries)| {
                if entries.contains(&Entity::Actor(name)) {
                    Some(prop.to_string())
                } else {
                    None
                }
            })
            .collect(),
    }
}

fn convert_to_py_event(e_i: u8, scenario: &Scenario<'_>) -> Result<PyEvent, PyErr> {
    let e = scenario
        .thematic_relations()
        .get(e_i as usize)
        .ok_or_else(|| {
            PyValueError::new_err(format!(
                "Result is event {e_i}, but no such event exists in the scenario!"
            ))
        })?;

    Ok(PyEvent {
        agent: e.agent.map(|x| x.to_string()),
        patient: e.patient.map(|x| x.to_string()),
        properties: scenario
            .properties()
            .iter()
            .filter_map(|(prop, entries)| {
                if entries.contains(&Entity::Event(e_i)) {
                    Some(prop.to_string())
                } else {
                    None
                }
            })
            .collect(),
    })
}

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
    ) -> PyScenarioIterator {
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

        PyScenarioIterator {
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

#[pyclass(name = "Actor", eq, str, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PyActor {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub properties: BTreeSet<String>,
}

#[pymethods]
impl PyActor {
    #[new]
    #[pyo3(signature = (name, properties=None))]
    ///Parameters
    ///----------
    ///name : str
    ///    The name of the actor.
    ///properties: set[str], optional
    ///    Any properties that apply to the actor.
    ///Returns
    ///-------
    ///Actor
    fn new(name: String, properties: Option<BTreeSet<String>>) -> Self {
        PyActor {
            name,
            properties: properties.unwrap_or_default(),
        }
    }
}

impl Display for PyActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.name,
            if self.properties.is_empty() { "" } else { " (" },
            self.properties.iter().join(", "),
            if self.properties.is_empty() { "" } else { ")" },
        )
    }
}

#[pyclass(name = "Event", eq, str, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct PyEvent {
    #[pyo3(get, set)]
    pub agent: Option<String>,
    #[pyo3(get, set)]
    pub patient: Option<String>,
    #[pyo3(get, set)]
    pub properties: BTreeSet<String>,
}

#[pymethods]
impl PyEvent {
    #[new]
    #[pyo3(signature = (agent=None, patient=None, properties=None))]
    ///Parameters
    ///----------
    ///agent : str, optional
    ///    The name of the agent (if there is one)
    ///patient : str, optional
    ///    The name of the patient (if there is one)
    ///properties: set[str], optional
    ///    Any properties that apply to the actor.
    ///Returns
    ///-------
    ///Event
    fn new(
        agent: Option<String>,
        patient: Option<String>,
        properties: Option<BTreeSet<String>>,
    ) -> Self {
        PyEvent {
            agent,
            patient,
            properties: properties.unwrap_or_default(),
        }
    }
}

impl Display for PyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{{}{}{}{}{}{}}}",
            self.agent
                .as_deref()
                .map(|x| format!("A = {x}"))
                .unwrap_or("".to_string()),
            if self.patient.is_some() && self.agent.is_some() {
                ", "
            } else {
                ""
            },
            self.patient
                .as_deref()
                .map(|x| format!("P = {x}"))
                .unwrap_or("".to_string()),
            if self.properties.is_empty() { "" } else { " (" },
            self.properties.iter().join(" "),
            if self.properties.is_empty() { "" } else { ")" },
        )
    }
}

impl PyEvent {
    pub fn into_theta_roles<'a>(self: &'a PyEvent) -> ThetaRoles<'a> {
        ThetaRoles {
            agent: self.agent.as_deref(),
            patient: self.patient.as_deref(),
        }
    }
}

#[pyclass(name = "ScenarioGenerator")]
pub struct PyScenarioIterator {
    generator: ScenarioIterator<'static>,
    _parameter_holder: Arc<ParameterHolder>,
}

#[pymethods]
impl PyScenarioIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyScenario> {
        slf.generator.next().map(|s| s.into())
    }
}

struct ParameterHolder {
    actors: Vec<String>,
    event_kinds: Vec<PyPossibleEvent>,
    actor_properties: Vec<String>,
}
