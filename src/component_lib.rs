use crate::{
    grid_db::{
        Component, ConnectionAlign, Port, PrimitiveComponent, PrimitiveType, Unit, grid_pos,
    },
    locale::Locale,
};

#[derive(Clone)]
pub struct ComponentLibEntry {
    pub name: &'static str,
    pub component: Component,
}

fn get_io() -> Vec<ComponentLibEntry> {
    vec![
        ComponentLibEntry {
            name: "INPUT",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Input,
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
        ComponentLibEntry {
            name: "OUTPUT",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Output,
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
        ComponentLibEntry {
            name: "POINT",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Point,
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
    ]
}

fn get_muxes() -> Vec<ComponentLibEntry> {
    vec![
        ComponentLibEntry {
            name: "MUX2",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Mux(2),
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
        ComponentLibEntry {
            name: "MUX4",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Mux(4),
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
        ComponentLibEntry {
            name: "MUX8",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Mux(8),
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
    ]
}

fn get_gates() -> Vec<ComponentLibEntry> {
    vec![
        ComponentLibEntry {
            name: "AND2",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::And(2),
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
        ComponentLibEntry {
            name: "OR2",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Or(2),
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
        ComponentLibEntry {
            name: "XOR2",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Xor(2),
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
        ComponentLibEntry {
            name: "NAND2",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Nand(2),
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
        ComponentLibEntry {
            name: "NOT",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Not,
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
    ]
}

fn get_units_examples() -> Vec<ComponentLibEntry> {
    vec![ComponentLibEntry {
        name: "Example unit",
        component: Component::Unit(Unit {
            name: "Example".to_owned(),
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
        }),
    }]
}

pub fn get_component_lib() -> Vec<Vec<ComponentLibEntry>> {
    vec![get_gates(), get_muxes(), get_io(), get_units_examples()]
}

pub fn get_component_lib_with_query(query: &String) -> Vec<Vec<ComponentLibEntry>> {
    if query == "" {
        get_component_lib()
    } else {
        get_component_lib()
            .iter()
            .map(|group| {
                group
                    .iter()
                    .filter_map(|entry| {
                        if entry.name.to_lowercase().contains(&query.to_lowercase()) {
                            Some(entry.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

pub fn get_group_name(group_id: usize, locale: &Locale) -> &'static str {
    match group_id {
        0 => locale.logic_gates,
        1 => locale.muxes,
        2 => locale.input_outputs,
        3 => locale.custom_units,
        _ => "",
    }
}
