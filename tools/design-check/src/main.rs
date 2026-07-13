//! Validates machine-readable design artifacts that require structured parsing.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table};

const REQUIRED_MACHINES: &[&str] = &[
    "binding-route",
    "directory-process",
    "expose",
    "in-flight",
    "subscription",
];

#[derive(Debug)]
struct Transition {
    from: String,
    event: String,
    to: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("design structure check: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let command = env::args().nth(1).unwrap_or_else(|| "check".to_owned());
    if command != "check" && command != "check-state" {
        return Err(format!(
            "unknown command {command:?}; expected check or check-state"
        ));
    }

    let root = repository_root()?;
    check_state_machines(&root)?;
    println!("design structure check: state machines valid");
    Ok(())
}

fn repository_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "cannot resolve repository root".to_owned())
}

fn check_state_machines(root: &Path) -> Result<(), String> {
    let path = root.join("docs/state-machines.toml");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;

    require_integer(document.get("schema_version"), "schema_version", 1)?;
    require_string(document.get("design_revision"), "design_revision", "4.6")?;
    let machines = document
        .get("machine")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| "state artifact has no [[machine]] entries".to_owned())?;
    let design = fs::read_to_string(root.join("docs/design.md"))
        .map_err(|error| format!("cannot read docs/design.md: {error}"))?;

    let mut ids = BTreeSet::new();
    for machine in machines {
        check_machine(machine, &design, &mut ids)?;
    }

    let expected: BTreeSet<String> = REQUIRED_MACHINES
        .iter()
        .map(|value| (*value).to_owned())
        .collect();
    if ids != expected {
        return Err(format!(
            "state machine set mismatch; expected {expected:?}, found {ids:?}"
        ));
    }
    Ok(())
}

fn check_machine(machine: &Table, design: &str, ids: &mut BTreeSet<String>) -> Result<(), String> {
    let id = string_field(machine, "id", "machine")?;
    if !ids.insert(id.clone()) {
        return Err(format!("duplicate state machine {id:?}"));
    }

    let states = string_set(array_field(machine, "states", &id)?, &id, "states")?;
    let terminals = string_set(array_field(machine, "terminal", &id)?, &id, "terminal")?;
    if terminals.is_empty() || !terminals.is_subset(&states) {
        return Err(format!("machine {id:?} has invalid terminal states"));
    }
    let initial = string_field(machine, "initial", &id)?;
    if !states.contains(&initial) {
        return Err(format!(
            "machine {id:?} initial state {initial:?} is unknown"
        ));
    }

    for requirement in string_set(
        array_field(machine, "requirements", &id)?,
        &id,
        "requirements",
    )? {
        if !design.contains(&format!("`{requirement}`:")) {
            return Err(format!(
                "machine {id:?} references unknown requirement {requirement:?}"
            ));
        }
    }

    let transition_tables = machine
        .get("transition")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| format!("machine {id:?} has no transitions"))?;
    let transitions = parse_transitions(&id, transition_tables, &states, &terminals)?;
    check_required_events(machine, &id, &transitions)?;
    check_reachability(&id, &initial, &states, &terminals, &transitions)?;
    Ok(())
}

fn parse_transitions(
    machine: &str,
    tables: &ArrayOfTables,
    states: &BTreeSet<String>,
    terminals: &BTreeSet<String>,
) -> Result<Vec<Transition>, String> {
    let mut transitions = Vec::new();
    let mut identities = BTreeSet::new();
    for table in tables {
        let from = string_field(table, "from", machine)?;
        let event = string_field(table, "event", machine)?;
        let to = string_field(table, "to", machine)?;
        if !states.contains(&from) || !states.contains(&to) {
            return Err(format!(
                "machine {machine:?} transition {from:?}:{event:?}->{to:?} uses an unknown state"
            ));
        }
        if terminals.contains(&from) {
            return Err(format!(
                "machine {machine:?} terminal state {from:?} has transition {event:?}"
            ));
        }
        if !identities.insert((from.clone(), event.clone())) {
            return Err(format!(
                "machine {machine:?} duplicates transition {from:?}:{event:?}"
            ));
        }
        for field in ["owner", "linearization", "outcome", "retryability"] {
            string_field(table, field, machine)?;
        }
        transitions.push(Transition { from, event, to });
    }
    Ok(transitions)
}

fn check_required_events(
    machine: &Table,
    id: &str,
    transitions: &[Transition],
) -> Result<(), String> {
    let required = string_set(
        array_field(machine, "required_events", id)?,
        id,
        "required_events",
    )?;
    let actual: BTreeSet<String> = transitions
        .iter()
        .map(|transition| format!("{}:{}", transition.from, transition.event))
        .collect();
    let missing: Vec<&String> = required.difference(&actual).collect();
    if !missing.is_empty() {
        return Err(format!(
            "machine {id:?} is missing required events {missing:?}"
        ));
    }
    Ok(())
}

fn check_reachability(
    machine: &str,
    initial: &str,
    states: &BTreeSet<String>,
    terminals: &BTreeSet<String>,
    transitions: &[Transition],
) -> Result<(), String> {
    let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut incoming: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for transition in transitions {
        outgoing
            .entry(&transition.from)
            .or_default()
            .push(&transition.to);
        incoming
            .entry(&transition.to)
            .or_default()
            .push(&transition.from);
    }

    let reachable = traverse(initial, &outgoing);
    let unreachable: Vec<&String> = states
        .iter()
        .filter(|state| !reachable.contains(state.as_str()))
        .collect();
    if !unreachable.is_empty() {
        return Err(format!(
            "machine {machine:?} has unreachable states {unreachable:?}"
        ));
    }

    let mut terminal_reachable = BTreeSet::new();
    let mut queue: VecDeque<&str> = terminals.iter().map(String::as_str).collect();
    while let Some(state) = queue.pop_front() {
        if !terminal_reachable.insert(state) {
            continue;
        }
        if let Some(previous) = incoming.get(state) {
            queue.extend(previous.iter().copied());
        }
    }
    let trapped: Vec<&String> = states
        .iter()
        .filter(|state| !terminals.contains(*state))
        .filter(|state| !terminal_reachable.contains(state.as_str()))
        .collect();
    if !trapped.is_empty() {
        return Err(format!(
            "machine {machine:?} has nonterminal states with no terminal path {trapped:?}"
        ));
    }
    Ok(())
}

fn traverse<'a>(start: &'a str, edges: &BTreeMap<&'a str, Vec<&'a str>>) -> BTreeSet<&'a str> {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([start]);
    while let Some(state) = queue.pop_front() {
        if !visited.insert(state) {
            continue;
        }
        if let Some(next) = edges.get(state) {
            queue.extend(next.iter().copied());
        }
    }
    visited
}

fn array_field<'a>(table: &'a Table, field: &str, context: &str) -> Result<&'a Array, String> {
    table
        .get(field)
        .and_then(Item::as_array)
        .ok_or_else(|| format!("{context:?} has no {field:?} array"))
}

fn string_field(table: &Table, field: &str, context: &str) -> Result<String, String> {
    let value = table
        .get(field)
        .and_then(Item::as_str)
        .ok_or_else(|| format!("{context:?} has no string field {field:?}"))?;
    if value.is_empty() {
        return Err(format!("{context:?} has an empty {field:?}"));
    }
    Ok(value.to_owned())
}

fn string_set(array: &Array, context: &str, field: &str) -> Result<BTreeSet<String>, String> {
    let mut values = BTreeSet::new();
    for value in array {
        let value = value
            .as_str()
            .ok_or_else(|| format!("{context:?} {field:?} contains a non-string value"))?;
        if value.is_empty() || !values.insert(value.to_owned()) {
            return Err(format!(
                "{context:?} {field:?} contains an empty or duplicate value {value:?}"
            ));
        }
    }
    Ok(values)
}

fn require_integer(item: Option<&Item>, field: &str, expected: i64) -> Result<(), String> {
    let actual = item.and_then(Item::as_integer);
    if actual != Some(expected) {
        return Err(format!(
            "{field} mismatch; expected {expected}, found {actual:?}"
        ));
    }
    Ok(())
}

fn require_string(item: Option<&Item>, field: &str, expected: &str) -> Result<(), String> {
    let actual = item.and_then(Item::as_str);
    if actual != Some(expected) {
        return Err(format!(
            "{field} mismatch; expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}
