use crate::grid_db::{
    Component, ConnectionAlign, Port, PrimitiveComponent, PrimitiveType, Unit, grid_pos,
};

pub struct ComponentLibEntry {
    pub name: &'static str,
    pub component: Component,
}

pub fn get_component_lib() -> Vec<ComponentLibEntry> {
    vec![
        ComponentLibEntry {
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
        },
        ComponentLibEntry {
            name: "AND2",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::And(2),
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
        ComponentLibEntry {
            name: "AND3",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::And(3),
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
        ComponentLibEntry {
            name: "AND4",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::And(4),
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
            name: "OR4",
            component: Component::Primitive(PrimitiveComponent {
                typ: PrimitiveType::Or(4),
                pos: grid_pos(1, 1), // Default preview pos
                rotation: crate::grid_db::Rotation::ROT0,
            }),
        },
    ]
}
