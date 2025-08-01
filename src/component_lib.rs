use crate::{
    grid_db::{
        Component, DFFParams, Port, PrimitiveComponent, PrimitiveType, Rotation, TextField, Unit,
        grid_pos,
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
    vec![
        ComponentLibEntry {
            name: "Empty unit",
            component: Component::Unit(Unit {
                pos: grid_pos(1, 1), // Default preview pos
                width: 5,
                height: 5,
                ports: vec![],
            }),
        },
        ComponentLibEntry {
            name: "Example unit",
            component: Component::Unit(Unit {
                pos: grid_pos(1, 1), // Default preview pos
                width: 5,
                height: 6,
                ports: vec![
                    Port {
                        offset: 3,
                        align: Rotation::ROT0,
                        name: "vld".to_owned(),
                    },
                    Port {
                        offset: 4,
                        align: Rotation::ROT0,
                        name: "data1".to_owned(),
                    },
                    Port {
                        offset: 5,
                        align: Rotation::ROT0,
                        name: "data2".to_owned(),
                    },
                    Port {
                        offset: 1,
                        align: Rotation::ROT180,
                        name: "vld".to_owned(),
                    },
                    Port {
                        offset: 2,
                        align: Rotation::ROT180,
                        name: "data1".to_owned(),
                    },
                    Port {
                        offset: 3,
                        align: Rotation::ROT180,
                        name: "data2".to_owned(),
                    },
                    Port {
                        offset: 2,
                        align: Rotation::ROT90,
                        name: "error".to_owned(),
                    },
                    Port {
                        offset: 2,
                        align: Rotation::ROT270,
                        name: "clk".to_owned(),
                    },
                ],
            }),
        },
    ]
}

fn get_flip_flops() -> Vec<ComponentLibEntry> {
    let mut result = Vec::with_capacity(8);
    for i in 0..8 {
        let has_enable = (i & 1) == 1;
        let has_sync_reset = ((i >> 1) & 1) == 1;
        let has_async_reset = ((i >> 2) & 1) == 1;
        for async_reset_inverted in if has_async_reset {
            [false, true].iter()
        } else {
            [false].iter()
        } {
            for sync_reset_inverted in if has_sync_reset {
                [false, true].iter()
            } else {
                [false].iter()
            } {
                let params = DFFParams {
                    has_enable: has_enable,
                    has_sync_reset: has_sync_reset,
                    has_async_reset: has_async_reset,
                    async_reset_inverted: *async_reset_inverted,
                    sync_reset_inverted: *sync_reset_inverted,
                };
                let name: &'static str = Box::leak(Box::new(format!(
                    "DFF{}{}{}",
                    if params.has_enable { "E" } else { "" },
                    if params.has_sync_reset {
                        format!("_RST{}", if params.sync_reset_inverted { "N" } else { "" })
                    } else {
                        String::new()
                    },
                    if params.has_async_reset {
                        format!(
                            "_ARST{}",
                            if params.async_reset_inverted { "N" } else { "" }
                        )
                    } else {
                        String::new()
                    }
                )));
                result.push(ComponentLibEntry {
                    name: &name,
                    component: Component::Primitive(PrimitiveComponent {
                        typ: PrimitiveType::DFF(params),
                        pos: grid_pos(1, 1), // Default preview pos
                        rotation: crate::grid_db::Rotation::ROT0,
                    }),
                });
            }
        }
    }
    result
}

fn get_text_labels() -> Vec<ComponentLibEntry> {
    vec![ComponentLibEntry {
        name: "Text field",
        component: Component::TextField(TextField {
            pos: grid_pos(1, 1), // Default preview pos
            size: (4, 1),
            text: "Some text".into(),
        }),
    }]
}

pub fn get_component_lib() -> Vec<Vec<ComponentLibEntry>> {
    vec![
        get_gates(),
        get_muxes(),
        get_io(),
        get_units_examples(),
        get_flip_flops(),
        get_text_labels(),
    ]
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
        4 => locale.flip_flops,
        5 => locale.text_labels,
        _ => "",
    }
}
