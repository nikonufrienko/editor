use crate::grid_db::{AndGate, Component, ConnectionAlign, Port, Unit, grid_pos};
use once_cell::sync::Lazy;

pub static EXAMPLE_UNIT: Lazy<Component> = Lazy::new(|| {
    Component::Unit(Unit {
        name: "АМОГУС".to_owned(),
        pos: grid_pos(1, 1), // Default preview pos
        width: 5,
        height: 6,
        ports: vec![
            Port {
                cell: grid_pos(0, 3),
                align: ConnectionAlign::LEFT,
                name: "vld".to_owned(),
            },
            Port {
                cell: grid_pos(0, 4),
                align: ConnectionAlign::LEFT,
                name: "data1".to_owned(),
            },
            Port {
                cell: grid_pos(0, 5),
                align: ConnectionAlign::LEFT,
                name: "data2".to_owned(),
            },
            Port {
                cell: grid_pos(4, 1),
                align: ConnectionAlign::RIGHT,
                name: "vld".to_owned(),
            },
            Port {
                cell: grid_pos(4, 2),
                align: ConnectionAlign::RIGHT,
                name: "data1".to_owned(),
            },
            Port {
                cell: grid_pos(4, 3),
                align: ConnectionAlign::RIGHT,
                name: "data2".to_owned(),
            },
            Port {
                cell: grid_pos(2, 0),
                align: ConnectionAlign::TOP,
                name: "error".to_owned(),
            },
        ],
    })
});

pub static AND2: Lazy<Component> = Lazy::new(|| {
    Component::AndGate(AndGate {
        n_inputs: 2,
        pos: grid_pos(1, 1), // Default preview pos
        rotation: crate::grid_db::Rotation::ROT0,
    })
});

pub static AND3: Lazy<Component> = Lazy::new(|| {
    Component::AndGate(AndGate {
        n_inputs: 3,
        pos: grid_pos(1, 1), // Default preview pos
        rotation: crate::grid_db::Rotation::ROT0,
    })
});

pub static AND9: Lazy<Component> = Lazy::new(|| {
    Component::AndGate(AndGate {
        n_inputs: 9,
        pos: grid_pos(1, 1), // Default preview pos
        rotation: crate::grid_db::Rotation::ROT0,
    })
});
