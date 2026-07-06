//! Library API: construct a machine from a native value (no YAML text), SPEC §2.
//! Mirrors the Python "build a machine in code" path.

use std::collections::BTreeMap;

use determa_state::{build_machine, load_machine_from_value, load_machines_from_values, validate, Engine, Value};
use serde_json::json;

fn gate_value() -> serde_json::Value {
    json!({
        "id": "gate",
        "events": {
            "coin": {"payload": {"amount": {"type": "int", "required": true}}},
            "push": {}
        },
        "top": {
            "esvs": {"fare": {"type": "int", "external": true}},
            "initial": {"transition_to": "locked"},
            "states": {
                "locked": {
                    "on_events": {
                        "coin": {"transition_to": "unlocked", "guard": "event.payload.amount >= fare"}
                    }
                },
                "unlocked": {"on_events": {"push": {"transition_to": "locked"}}}
            }
        }
    })
}

#[test]
fn build_machine_from_a_native_value() {
    // no YAML string is ever serialized — load straight from a serde_json::Value.
    let raw = load_machine_from_value(gate_value()).expect("parses");
    assert_eq!(raw.id, "gate");

    let (valid, _errs) = validate(std::slice::from_ref(&raw), &[]);
    assert!(valid);

    let machine = build_machine(&raw).expect("builds");
    let mut engine = Engine::new();
    engine.register(machine);

    let mut external = BTreeMap::new();
    external.insert("fare".to_string(), Value::Int(50));
    engine.create_root("g1", "gate", None, &external).expect("create_root");

    assert_eq!(engine.state_view("g1").unwrap().config, vec!["locked".to_string()]);

    let payload = Value::from_json(&json!({"amount": 100}));
    engine.send("g1", "coin", payload).expect("send");
    assert_eq!(engine.state_view("g1").unwrap().config, vec!["unlocked".to_string()]);
}

#[test]
fn multi_document_machine_from_values() {
    let root = json!({"id": "root", "top": {"initial": {"transition_to": "idle"}, "states": {"idle": {}}}});
    let child = json!({"id": "child", "top": {"initial": {"transition_to": "on"}, "states": {"on": {}}}});
    let docs = load_machines_from_values(vec![root, child]).expect("parses");
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].id, "root");
    assert_eq!(docs[1].id, "child");
}

#[test]
fn value_based_loading_rejects_an_invalid_mapping() {
    // missing required "top" — deserialize fails, same contract as YAML loading.
    let bad = json!({"id": "x"});
    assert!(load_machine_from_value(bad).is_err());
}
