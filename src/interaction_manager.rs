use std::collections::LinkedList;

use crate::{
    field::{FieldState, blocked_cell, filled_cells},
    grid_db::{
        Component, ComponentAction, ComponentColor, GridBD, GridBDConnectionPoint, GridPos, Id,
        Net, Port, RotationDirection, grid_pos, show_text_edit,
    },
    locale::Locale,
};
use egui::{
    Align2, Color32, CursorIcon, FontId, KeyboardShortcut, Modifiers, Painter, Pos2, Rect,
    Response, Shape, Stroke, StrokeKind, Ui, Vec2, epaint::TextShape, vec2,
};

pub fn draw_component_drag_preview(
    bd: &GridBD,
    state: &FieldState,
    dim: (i32, i32),
    painter: &Painter,
    pos: Pos2,
    component_id: Option<Id>,
    fill_color: Color32,
    only_overlap: bool,
) {
    let p0 = state.screen_to_grid(pos);
    let mut result = vec![];
    for x in 0..dim.0 {
        for y in 0..dim.1 {
            let cell = p0 + grid_pos(x, y);
            let available = if let Some(id) = component_id {
                bd.is_available_cell(cell, id)
            } else {
                bd.is_free_cell(cell, only_overlap)
            };
            if available {
                result.push(filled_cells(state, &cell, 1, 1, fill_color));
            } else {
                result.extend(blocked_cell(state, &cell));
            }
        }
    }
    painter.extend(result);
}

enum InteractionState {
    Idle,
    NetDragged {
        net_id: Id,
        segment_id: Id,
    },
    ComponentSelected(Id),
    ComponentDragged {
        id: Id,
        grab_ofs: Vec2,
    },
    Resizing {
        id: Id,
        direction: ResizeDirection,
    },
    EditingText {
        id: Id,
        text_edit_id: Id,
        text_buffer: String,
    },
    CreatingNet,
    AddingPort(Id),
    RemovingPort(Id),
    EditingPort(Id),
    CustomizeComponent {
        id: Id,
        buffer: Component,
    },
}

pub struct InteractionManager {
    state: InteractionState,
    drag_delta: Vec2,
    applied_transactions: LinkedList<Transaction>,
    reverted_transactions: LinkedList<Transaction>,
    connection_builder: ConnectionBuilder,
}

impl InteractionManager {
    pub fn new() -> Self {
        Self {
            state: InteractionState::Idle,
            drag_delta: vec2(0.0, 0.0),
            applied_transactions: LinkedList::new(),
            reverted_transactions: LinkedList::new(),
            connection_builder: ConnectionBuilder::new(),
        }
    }

    pub fn add_new_component(&mut self, component: Component, bd: &mut GridBD) {
        self.apply_new_transaction(
            Transaction::ChangeComponent {
                comp_id: bd.allocate_component(),
                old_comp: None,
                new_comp: Some(component),
            },
            bd,
        );
    }

    fn apply_new_transaction(&mut self, mut transaction: Transaction, bd: &mut GridBD) {
        transaction.apply(bd);
        self.applied_transactions.push_back(transaction);
        self.reverted_transactions.clear();
    }

    fn move_net_segment(
        &mut self,
        net_id: Id,
        segment_id: Id,
        cursor_grid_pos: &GridPos,
        bd: &mut GridBD,
    ) {
        let GridPos { x, y } = cursor_grid_pos;
        let mut net = bd.get_net(&net_id).unwrap().clone();
        let p1: GridPos = net.points[segment_id];
        let p2 = net.points[segment_id + 1];
        if p1.y == p2.y {
            net.points[segment_id] = grid_pos(p1.x, *y);
            net.points[segment_id + 1] = grid_pos(p2.x, *y);
        } else {
            net.points[segment_id] = grid_pos(*x, p1.y);
            net.points[segment_id + 1] = grid_pos(*x, p2.y);
        }
        if net.points.len() > 0 && (segment_id == net.points.len() - 2) {
            net.points.push(p2);
        }
        if segment_id == 0 {
            net.points.insert(0, p1);
        }
        net.points = simplify_path(net.points);
        self.apply_new_transaction(
            Transaction::ChangeNet {
                net_id: net_id,
                old_net: None,
                new_net: Some(net),
            },
            bd,
        );
    }

    fn get_net_connection_move_transaction(
        net_id: Id,
        bd: &GridBD,
        (delta_x_start, delta_y_start): (i32, i32),
        (delta_x_end, delta_y_end): (i32, i32),
    ) -> Option<Transaction> {
        let mut net = bd.get_net(&net_id).unwrap().clone();
        let pts_len = net.points.len();

        if delta_x_start == 0 && delta_y_start == 0 && delta_x_end == 0 && delta_y_end == 0 {
            return None;
        }
        if pts_len >= 2 {
            if delta_x_start == delta_x_end && delta_y_start == delta_y_end {
                // Just move all points:
                for i in 0..net.points.len() {
                    net.points[i] = net.points[i] + grid_pos(delta_x_start, delta_y_start);
                }
            } else {
                // Rebuild start point:
                let (delta_x, delta_y) = (delta_x_start, delta_y_start);
                if net.points[0].y == net.points[1].y {
                    // horizontal segment
                    if net.points.len() >= 4 {
                        // Has another vertical segment that can be moved
                        net.points[0] += grid_pos(delta_x, delta_y);
                        net.points[1] += grid_pos(delta_x, delta_y);
                        net.points[2] += grid_pos(delta_x, 0);
                    } else {
                        net.points[0].x += delta_x;
                        if delta_y != 0 {
                            net.points.insert(0, net.points[0] + grid_pos(0, delta_y));
                        }
                    }
                } else {
                    // vertical segment
                    if net.points.len() >= 4 {
                        // Has another horizontal segment that can be moved
                        net.points[0] += grid_pos(delta_x, delta_y);
                        net.points[1] += grid_pos(delta_x, delta_y);
                        net.points[2] += grid_pos(0, delta_y);
                    } else {
                        net.points[0].y += delta_y; // Fixed: change Y instead of X
                        if delta_x != 0 {
                            net.points.insert(0, net.points[0] + grid_pos(delta_x, 0));
                        }
                    }
                }
                // Rebuild end point:
                let (delta_x, delta_y) = (delta_x_end, delta_y_end);
                if net.points[pts_len - 1].y == net.points[pts_len - 2].y {
                    // horizontal segment
                    if net.points.len() >= 4 {
                        net.points[pts_len - 1] += grid_pos(delta_x, delta_y);
                        net.points[pts_len - 2] += grid_pos(delta_x, delta_y);
                        net.points[pts_len - 3] += grid_pos(delta_x, 0);
                    } else {
                        net.points[pts_len - 1].x += delta_x;
                        if delta_y != 0 {
                            net.points
                                .push(net.points[pts_len - 1] + grid_pos(0, delta_y));
                        }
                    }
                } else {
                    // vertical segment
                    if net.points.len() >= 4 {
                        net.points[pts_len - 1] += grid_pos(delta_x, delta_y);
                        net.points[pts_len - 2] += grid_pos(delta_x, delta_y);
                        net.points[pts_len - 3] += grid_pos(0, delta_y);
                    } else {
                        net.points[pts_len - 1].y += delta_y;
                        if delta_x != 0 {
                            net.points
                                .push(net.points[pts_len - 1] + grid_pos(delta_x, 0));
                        }
                    }
                }
            }
        }

        net.points = simplify_path(net.points);
        return Some(Transaction::ChangeNet {
            net_id: net_id,
            old_net: None,
            new_net: Some(net),
        });
    }

    fn move_component(&mut self, comp_id: Id, bd: &mut GridBD, new_pos: GridPos) {
        let comp = bd.get_component(&comp_id).unwrap();

        if bd.is_available_location(new_pos, comp.get_dimension(), comp_id) {
            let old_pos = comp.get_position();
            let delta_y = new_pos.y - old_pos.y;
            let delta_x = new_pos.x - old_pos.x;

            let mut new_comp = comp.clone();
            new_comp.set_pos(new_pos);

            let mut transactions = LinkedList::new();
            for net_id in bd.get_connected_nets(&comp_id) {
                let net = bd.get_net(&net_id).unwrap();
                let trans = Self::get_net_connection_move_transaction(
                    net_id,
                    bd,
                    if net.start_point.component_id == comp_id {
                        (delta_x, delta_y)
                    } else {
                        (0, 0)
                    },
                    if net.end_point.component_id == comp_id {
                        (delta_x, delta_y)
                    } else {
                        (0, 0)
                    },
                );
                if let Some(t) = trans {
                    transactions.push_back(t);
                }
            }
            transactions.push_back(Transaction::ChangeComponent {
                comp_id,
                old_comp: None,
                new_comp: Some(new_comp),
            });
            self.apply_new_transaction(Transaction::CombinedTransaction(transactions), bd);
        }
    }

    fn get_net_rotation_transaction(
        net_id: Id,
        bd: &GridBD,
        rot_center: GridPos,
        offset: GridPos,
        rotation_dir: RotationDirection,
    ) -> Transaction {
        let mut new_net = bd.get_net(&net_id).unwrap().clone();
        for p in &mut new_net.points {
            let dx = p.x - rot_center.x;
            let dy = p.y - rot_center.y;
            match rotation_dir {
                RotationDirection::Up => {
                    // -90 degree
                    *p = grid_pos(-dy + rot_center.x, dx + rot_center.y);
                }
                RotationDirection::Down => {
                    // -90 degree
                    *p = grid_pos(dy + rot_center.x, -dx + rot_center.y);
                }
            }
            *p = *p + offset;
        }
        return Transaction::ChangeNet {
            net_id: net_id,
            old_net: None,
            new_net: Some(new_net),
        };
    }

    fn rotate_component(&mut self, comp_id: Id, bd: &mut GridBD, dir: RotationDirection) {
        let comp = bd.get_component(&comp_id).unwrap().clone();
        let mut rotated_comp = comp.clone();
        rotated_comp.rotate(dir);

        if bd.is_available_location(
            rotated_comp.get_position(),
            rotated_comp.get_dimension(),
            comp_id,
        ) {
            let nets_ids: Vec<Id> = bd
                .get_connected_nets(&comp_id)
                .iter()
                .map(|it| *it)
                .collect();

            let mut transactions = LinkedList::new();
            for net_id in nets_ids.iter() {
                let net = bd.get_net(&net_id).unwrap();
                if net.end_point.component_id == comp_id && net.start_point.component_id == comp_id
                {
                    transactions.push_back(Self::get_net_rotation_transaction(
                        *net_id,
                        bd,
                        comp.get_position(),
                        match dir {
                            RotationDirection::Up => grid_pos(comp.get_dimension().1 - 1, 0),
                            RotationDirection::Down => grid_pos(0, comp.get_dimension().0 - 1),
                        },
                        dir,
                    ));
                } else {
                    let trans = Self::get_net_connection_move_transaction(
                        *net_id,
                        bd,
                        if net.start_point.component_id == comp_id {
                            let old_cell = comp
                                .get_connection_dock_cell(net.start_point.connection_id)
                                .unwrap();
                            let new_cell = rotated_comp
                                .get_connection_dock_cell(net.start_point.connection_id)
                                .unwrap();
                            (new_cell.x - old_cell.x, new_cell.y - old_cell.y)
                        } else {
                            (0, 0)
                        },
                        if net.end_point.component_id == comp_id {
                            let old_cell = comp
                                .get_connection_dock_cell(net.end_point.connection_id)
                                .unwrap();
                            let new_cell = rotated_comp
                                .get_connection_dock_cell(net.end_point.connection_id)
                                .unwrap();
                            (new_cell.x - old_cell.x, new_cell.y - old_cell.y)
                        } else {
                            (0, 0)
                        },
                    );
                    if let Some(t) = trans {
                        transactions.push_back(t);
                    }
                }
            }

            transactions.push_back(Transaction::ChangeComponent {
                comp_id,
                old_comp: None,
                new_comp: Some(rotated_comp),
            });
            self.apply_new_transaction(Transaction::CombinedTransaction(transactions), bd);
        }
    }

    fn apply_resize(&mut self, bd: &mut GridBD, comp_id: Id, new_size: (i32, i32)) {
        let comp = bd.get_component(&comp_id).unwrap();

        if bd.is_available_location(comp.get_position(), new_size, comp_id) {
            let mut transactions = LinkedList::new();
            let mut new_comp = comp.clone();
            new_comp.set_size(new_size);

            // Refresh connected nets:
            let nets_ids: Vec<Id> = bd
                .get_connected_nets(&comp_id)
                .iter()
                .map(|it| *it)
                .collect();

            for net_id in &nets_ids {
                let net = bd.get_net(&net_id).unwrap();
                let trans = Self::get_net_connection_move_transaction(
                    *net_id,
                    bd,
                    if net.start_point.component_id == comp_id {
                        let old_cell = comp
                            .get_connection_dock_cell(net.start_point.connection_id)
                            .unwrap();
                        let new_cell = new_comp
                            .get_connection_dock_cell(net.start_point.connection_id)
                            .unwrap();
                        (new_cell.x - old_cell.x, new_cell.y - old_cell.y)
                    } else {
                        (0, 0)
                    },
                    if net.end_point.component_id == comp_id {
                        let old_cell = comp
                            .get_connection_dock_cell(net.end_point.connection_id)
                            .unwrap();
                        let new_cell = new_comp
                            .get_connection_dock_cell(net.end_point.connection_id)
                            .unwrap();
                        (new_cell.x - old_cell.x, new_cell.y - old_cell.y)
                    } else {
                        (0, 0)
                    },
                );
                if let Some(t) = trans {
                    transactions.push_back(t);
                }
            }
            transactions.push_back(Transaction::ChangeComponent {
                comp_id: comp_id,
                old_comp: None,
                new_comp: Some(new_comp),
            });

            self.apply_new_transaction(Transaction::CombinedTransaction(transactions), bd);
        }
    }

    fn remove_component(&mut self, bd: &mut GridBD, comp_id: Id) {
        let mut transactions = LinkedList::new();
        for net_id in bd.get_connected_nets(&comp_id) {
            transactions.push_back(Transaction::ChangeNet {
                net_id: net_id,
                old_net: None,
                new_net: None,
            });
        }
        transactions.push_back(Transaction::ChangeComponent {
            comp_id: comp_id,
            old_comp: None,
            new_comp: None,
        });
        self.apply_new_transaction(Transaction::CombinedTransaction(transactions), bd);
    }

    const UNDO_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, egui::Key::Z);
    const REDO_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, egui::Key::Y);

    fn remove_port(&mut self, bd: &mut GridBD, comp_id: Id, port_id: Id) {
        let mut transactions = LinkedList::new();
        // Refresh connected net:
        for net_id in bd.get_connected_nets(&comp_id) {
            let net = bd.get_net(&net_id).unwrap();
            if (net.end_point.connection_id == port_id && net.end_point.component_id == comp_id)
                || (net.start_point.connection_id == port_id
                    && net.start_point.component_id == comp_id)
            {
                transactions.push_back(Transaction::ChangeNet {
                    net_id: net_id,
                    old_net: None,
                    new_net: None,
                });
            } else {
                let mut new_net = net.clone();
                if net.start_point.connection_id > port_id
                    && net.start_point.component_id == comp_id
                {
                    new_net.start_point.connection_id -= 1;
                }
                if net.end_point.connection_id > port_id && net.end_point.component_id == comp_id {
                    new_net.end_point.connection_id -= 1;
                }
                transactions.push_back(Transaction::ChangeNet {
                    net_id: net_id,
                    old_net: None,
                    new_net: Some(new_net),
                });
            }
        }
        let mut new_comp = bd.get_component(&comp_id).unwrap().clone();
        new_comp.remove_port(port_id);
        transactions.push_back(Transaction::ChangeComponent {
            comp_id: comp_id,
            old_comp: None,
            new_comp: Some(new_comp),
        });
        self.apply_new_transaction(Transaction::CombinedTransaction(transactions), bd);
    }

    fn apply_customization(&mut self, bd: &mut GridBD, comp_id: Id, customized_comp: Component) {
        let old_comp = bd.get_component(&comp_id).unwrap();
        let connections_diff = old_comp.get_connections_diff(&customized_comp);
        let mut transactions = LinkedList::new();

        // Rebuild connected nets:
        for net_id in &bd.get_connected_nets(&comp_id) {
            let net = bd.get_net(&net_id).unwrap();
            let mut new_net = net.clone();
            let mut remove_net = false;

            if net.start_point.component_id == comp_id {
                if let Some(new_id) = connections_diff.get(&net.start_point.connection_id) {
                    if let Some(new_id) = new_id {
                        new_net.start_point.connection_id = *new_id;
                    } else {
                        remove_net = true;
                    }
                }
            }
            if net.end_point.component_id == comp_id {
                if let Some(new_id) = connections_diff.get(&net.end_point.connection_id) {
                    if let Some(new_id) = new_id {
                        new_net.end_point.connection_id = *new_id;
                    } else {
                        remove_net = true;
                    }
                }
            }
            let transaction = if !remove_net {
                // Rebuild net:
                Self::get_net_connection_move_transaction(
                    *net_id,
                    bd,
                    if net.start_point.component_id == comp_id {
                        let p0 = old_comp
                            .get_connection_dock_cell(net.start_point.connection_id)
                            .unwrap();
                        let p1 = customized_comp
                            .get_connection_dock_cell(new_net.start_point.connection_id)
                            .unwrap();
                        (p1.x - p0.x, p1.y - p0.y)
                    } else {
                        (0, 0)
                    },
                    if net.end_point.component_id == comp_id {
                        let p0 = old_comp
                            .get_connection_dock_cell(net.end_point.connection_id)
                            .unwrap();
                        let p1 = customized_comp
                            .get_connection_dock_cell(new_net.end_point.connection_id)
                            .unwrap();
                        (p1.x - p0.x, p1.y - p0.y)
                    } else {
                        (0, 0)
                    },
                )
            } else {
                // Remove net:
                Some(Transaction::ChangeNet {
                    net_id: *net_id,
                    old_net: None,
                    new_net: None,
                })
            };

            if let Some(t) = transaction {
                transactions.push_back(t);
            }
        }
        transactions.push_back(Transaction::ChangeComponent {
            comp_id,
            old_comp: None,
            new_comp: Some(customized_comp),
        });
        self.apply_new_transaction(Transaction::CombinedTransaction(transactions), bd);
    }

    /// Refreshes action state.
    /// Returns false if no action performed.
    pub fn refresh(
        &mut self,
        bd: &mut GridBD,
        state: &FieldState,
        response: &Response,
        ui: &egui::Ui,
        locale: &'static Locale,
    ) -> bool {
        match self.state {
            InteractionState::EditingText {
                id: _,
                text_edit_id: _,
                text_buffer: _,
            } => {}
            _ => {
                if ui.input_mut(|i| i.consume_shortcut(&Self::UNDO_SHORTCUT)) {
                    // Undo:
                    match self.state {
                        InteractionState::Idle => {
                            if let Some(mut trans) = self.applied_transactions.pop_back() {
                                trans.revert(bd);
                                self.reverted_transactions.push_front(trans);
                            }
                        }
                        _ => {
                            self.state = InteractionState::Idle;
                        }
                    }
                } else if ui.input_mut(|i| i.consume_shortcut(&Self::REDO_SHORTCUT)) {
                    // Redo:
                    match self.state {
                        InteractionState::Idle => {
                            if let Some(mut trans) = self.reverted_transactions.pop_front() {
                                trans.apply(bd);
                                self.applied_transactions.push_back(trans);
                            }
                        }
                        _ => {} // ???
                    }
                }
            }
        }

        match &self.state {
            InteractionState::NetDragged { net_id, segment_id } => {
                if let Some(hover_pos) = state.cursor_pos {
                    let segment = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(*segment_id, *net_id)
                        .unwrap();
                    if segment.is_horizontal() {
                        ui.ctx()
                            .output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
                    } else {
                        ui.ctx()
                            .output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                    }
                    if response.is_pointer_button_down_on() {
                        self.drag_delta += response.drag_delta();
                        return true;
                    } else {
                        self.drag_delta = vec2(0.0, 0.0);
                        self.move_net_segment(
                            *net_id,
                            *segment_id,
                            &state.screen_to_grid(hover_pos),
                            bd,
                        );
                        self.state = InteractionState::Idle
                    }
                }
            }
            InteractionState::Idle => {
                if let Some(resp) = self.connection_builder.update(bd, state, &response) {
                    match resp {
                        ConnectionBuilderResponse::Toggled => {
                            self.state = InteractionState::CreatingNet;
                            return true;
                        }
                        ConnectionBuilderResponse::Hovered => {}
                        ConnectionBuilderResponse::Complete(_) => {
                            panic!("Unexpected complete of building connection")
                        }
                    }
                } else if let Some(segment) = bd.get_hovered_segment(state) {
                    if segment.is_horizontal() {
                        ui.ctx()
                            .output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
                    } else {
                        ui.ctx()
                            .output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                    }
                    if response.is_pointer_button_down_on() {
                        // Do no use dragged() or drag_started()
                        self.drag_delta += response.drag_delta();
                        self.state = InteractionState::NetDragged {
                            net_id: segment.net_id,
                            segment_id: segment.inner_id,
                        };
                        return true;
                    }
                } else if let Some(id) = bd.get_hovered_component_id(state) {
                    ui.ctx()
                        .output_mut(|o| o.cursor_icon = CursorIcon::Crosshair);
                    if response.clicked() {
                        self.state = InteractionState::ComponentSelected(*id);
                        return true;
                    }
                }
            }
            InteractionState::ComponentSelected(id) => {
                let comp = bd.get_component(&id).unwrap();
                let resizable = comp.is_resizable();
                let right_border_hovered =
                    Self::is_right_selection_border_hovered(state.cursor_pos, state, comp);
                let bottom_border_hovered =
                    Self::is_bottom_selection_border_hovered(state.cursor_pos, state, comp);

                // Check actions:
                let action = Self::get_action(comp, state);
                if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                    self.remove_component(bd, *id);
                    self.state = InteractionState::Idle;
                    return true;
                }
                if response.clicked() && action != ComponentAction::None {
                    match action {
                        ComponentAction::RotateUp => {
                            self.rotate_component(*id, bd, RotationDirection::Up);
                            self.state = InteractionState::Idle;
                        }
                        ComponentAction::RotateDown => {
                            self.rotate_component(*id, bd, RotationDirection::Down);
                            self.state = InteractionState::Idle;
                        }
                        ComponentAction::Remove => {
                            self.remove_component(bd, *id);
                            self.state = InteractionState::Idle;
                            return true;
                        }
                        ComponentAction::AddPort => {
                            self.state = InteractionState::AddingPort(*id);
                            return true;
                        }
                        ComponentAction::RemovePort => {
                            self.state = InteractionState::RemovingPort(*id);
                            return true;
                        }
                        ComponentAction::EditPort => {
                            self.state = InteractionState::EditingPort(*id);
                            return true;
                        }
                        ComponentAction::EditText => {
                            self.state = InteractionState::EditingText {
                                id: *id,
                                text_edit_id: 0,
                                text_buffer: comp.get_text_edit(0).unwrap().clone(),
                            };
                            return true;
                        }
                        ComponentAction::Customize => {
                            self.state = InteractionState::CustomizeComponent {
                                id: *id,
                                buffer: bd.get_component(id).unwrap().clone(),
                            };
                            return true;
                        }
                        _ => {}
                    }
                    return true;
                } else if comp.is_hovered(state) {
                    ui.ctx().output_mut(|o| o.cursor_icon = CursorIcon::Grab);

                    // Check dragging:
                    if response.dragged() {
                        if let Some(hovepos) = response.hover_pos() {
                            self.state = InteractionState::ComponentDragged {
                                id: *id,
                                grab_ofs: hovepos.to_vec2()
                                    - state.grid_to_screen(&comp.get_position()).to_vec2(),
                            };
                        }
                    }
                    return true;
                } else if resizable && right_border_hovered {
                    ui.ctx()
                        .output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                    if response.is_pointer_button_down_on() {
                        self.state = InteractionState::Resizing {
                            id: *id,
                            direction: ResizeDirection::Right,
                        };
                        return true;
                    }
                } else if resizable && bottom_border_hovered {
                    ui.ctx()
                        .output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
                    if response.is_pointer_button_down_on() {
                        self.state = InteractionState::Resizing {
                            id: *id,
                            direction: ResizeDirection::Down,
                        };
                        return true;
                    }
                } else if response.clicked() {
                    self.state = InteractionState::Idle;
                }
            }
            InteractionState::ComponentDragged { id, grab_ofs } => {
                if response.dragged() {
                    ui.ctx()
                        .output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
                } else {
                    if let Some(pos) = state.cursor_pos {
                        self.move_component(*id, bd, state.screen_to_grid(pos - *grab_ofs));
                    }
                    self.state = InteractionState::Idle;
                }
                return true;
            }
            InteractionState::Resizing { id, direction } => {
                if response.is_pointer_button_down_on() {
                    ui.ctx().output_mut(|o| {
                        o.cursor_icon = match direction {
                            ResizeDirection::Down => CursorIcon::ResizeVertical,
                            ResizeDirection::Right => CursorIcon::ResizeHorizontal,
                        }
                    });
                } else {
                    let comp = bd.get_component(&id).unwrap();
                    if let Some(new_size) = Self::get_new_size(comp, state, *direction) {
                        self.apply_resize(bd, *id, new_size);
                    }
                    self.state = InteractionState::Idle;
                }
                return true;
            }
            InteractionState::EditingText {
                id,
                text_edit_id,
                text_buffer,
            } => {
                let comp = bd.get_component(&id).unwrap();
                let text_edit_rect = comp.get_text_edit_rect(*text_edit_id, state).unwrap();

                if response.clicked() {
                    // Save changes and exit:
                    if let Some(cursor_pos) = state.cursor_pos {
                        if !text_edit_rect.contains(cursor_pos) {
                            let mut new_comp = comp.clone();
                            *(new_comp.get_text_edit_mut(*text_edit_id).unwrap()) =
                                text_buffer.clone();
                            self.apply_new_transaction(
                                Transaction::ChangeComponent {
                                    comp_id: *id,
                                    old_comp: None,
                                    new_comp: Some(new_comp),
                                },
                                bd,
                            );
                            self.state = InteractionState::Idle;
                            return true;
                        }
                    }
                }
            }
            InteractionState::CreatingNet => {
                if let Some(resp) = self.connection_builder.update(bd, state, response) {
                    match resp {
                        ConnectionBuilderResponse::Complete(t) => {
                            self.apply_new_transaction(t, bd);
                            debug_assert!(!self.connection_builder.is_active());
                            self.state = InteractionState::Idle;
                            return true;
                        }
                        ConnectionBuilderResponse::Toggled => panic!(),
                        _ => {}
                    }
                }
            }
            InteractionState::AddingPort(id) => {
                let comp = bd.get_component(id).unwrap();
                if response.clicked() && !comp.is_hovered(state) {
                    self.state = InteractionState::Idle;
                    return true;
                } else if response.clicked() {
                    if let Some((rotation, offset, _)) = comp.get_nearest_port_pos(state, false) {
                        let mut new_comp = comp.clone();
                        new_comp.add_port(Port {
                            offset: offset,
                            align: rotation,
                            name: "...".into(),
                        });
                        self.apply_new_transaction(
                            Transaction::ChangeComponent {
                                comp_id: *id,
                                old_comp: None,
                                new_comp: Some(new_comp),
                            },
                            bd,
                        );
                    }
                }
            }
            InteractionState::RemovingPort(id) => {
                // TODO
                let comp = bd.get_component(id).unwrap();
                if response.clicked() && !comp.is_hovered(state) {
                    self.state = InteractionState::Idle;
                    return true;
                } else if response.clicked() {
                    if let Some((_, _, port_id)) = comp.get_nearest_port_pos(state, true) {
                        self.remove_port(bd, *id, port_id.unwrap());
                    }
                }
            }
            InteractionState::EditingPort(id) => {
                let comp = bd.get_component(id).unwrap();
                if response.clicked() && !comp.is_hovered(state) {
                    self.state = InteractionState::Idle;
                    return true;
                } else if response.clicked() {
                    if let Some((_, _, port_id)) = comp.get_nearest_port_pos(state, true) {
                        let port_id = port_id.unwrap();
                        self.state = InteractionState::EditingText {
                            id: *id,
                            text_edit_id: port_id,
                            text_buffer: comp.get_text_edit(port_id).unwrap().clone(),
                        };
                        return true;
                    }
                }
            }
            InteractionState::CustomizeComponent { id: _, buffer: _ } => {
                let done = if let InteractionState::CustomizeComponent { id: _, buffer } =
                    &mut self.state
                {
                    egui::modal::Modal::new("customizing".into())
                        .show(ui.ctx(), |ui| {
                            buffer.show_customization_panel(ui, locale);
                            ui.button("Ok").clicked()
                        })
                        .inner
                } else {
                    panic!()
                };

                if done {
                    if let InteractionState::CustomizeComponent { id, buffer } =
                        std::mem::replace(&mut self.state, InteractionState::Idle)
                    {
                        self.apply_customization(bd, id, buffer);
                        return true;
                    } else {
                        panic!();
                    }
                }
            }
        }
        false
    }

    pub fn draw(&mut self, bd: &mut GridBD, state: &FieldState, painter: &Painter, ui: &mut Ui) {
        match &mut self.state {
            InteractionState::NetDragged { net_id, segment_id } => {
                let ofs = vec2(0.5, 0.5) * state.grid_size;
                if let Some(pos) = state.cursor_pos {
                    let GridPos { x, y } = state.screen_to_grid(pos);
                    let segment = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(*segment_id, *net_id)
                        .unwrap();
                    let (p1, p2) = if segment.is_horizontal() {
                        (
                            state.grid_to_screen(&grid_pos(segment.pos1.x, y)),
                            state.grid_to_screen(&grid_pos(segment.pos2.x, y)),
                        )
                    } else {
                        (
                            state.grid_to_screen(&grid_pos(x, segment.pos1.y)),
                            state.grid_to_screen(&grid_pos(x, segment.pos2.y)),
                        )
                    };
                    let mut pts = vec![p1 + ofs, p2 + ofs];
                    if let Some(next_segment) = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(*segment_id + 1, *net_id)
                    {
                        pts.push(state.grid_to_screen(&next_segment.pos2) + ofs);
                    } else {
                        pts.push(state.grid_to_screen(&segment.pos2) + ofs);
                    }
                    if let Some(prev_segment) = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(segment_id.wrapping_sub(1), *net_id)
                    {
                        pts.insert(0, state.grid_to_screen(&prev_segment.pos1) + ofs);
                    } else {
                        pts.insert(0, state.grid_to_screen(&segment.pos1) + ofs);
                    }
                    painter.line(
                        pts,
                        Stroke::new(
                            state.grid_size * 0.1,
                            Color32::from_rgba_unmultiplied(100, 100, 0, 100),
                        ),
                    );
                }
            }
            InteractionState::Idle => {
                if !self.connection_builder.draw(bd, state, painter) {
                    if let Some(seg) = bd.get_hovered_segment(state) {
                        seg.highlight(state, &painter);
                    }
                }
            }
            InteractionState::ComponentSelected(id) => {
                if let Some(comp) = bd.get_component(&id) {
                    let rect = Self::get_selection_rect(comp, state);
                    painter.rect_stroke(
                        rect,
                        state.grid_size * 0.1,
                        Stroke::new(
                            state.grid_size * 0.15,
                            Color32::from_rgba_unmultiplied(100, 100, 0, 100),
                        ),
                        StrokeKind::Outside,
                    );
                    Self::draw_actions_panel(comp, state, ui, painter);
                }
            }
            InteractionState::ComponentDragged { id, grab_ofs } => {
                if let Some(pos) = state.cursor_pos {
                    let comp = bd.get_component(&id).unwrap().is_overlap_only();
                    draw_component_drag_preview(
                        bd,
                        state,
                        bd.get_component(&id).unwrap().get_dimension(),
                        painter,
                        pos - *grab_ofs,
                        Some(*id),
                        ui.visuals().strong_text_color().gamma_multiply(0.08),
                        comp,
                    );
                }
            }
            InteractionState::Resizing { id, direction } => {
                if let Some(comp) = bd.get_component(&id) {
                    if let Some(resize_rect) =
                        Self::get_resize_selection_rect(comp, state, *direction)
                    {
                        painter.rect_stroke(
                            resize_rect,
                            state.grid_size * 0.1,
                            Stroke::new(
                                state.grid_size * 0.15,
                                Color32::from_rgba_unmultiplied(100, 100, 0, 100),
                            ),
                            StrokeKind::Outside,
                        );
                    }
                }
            }
            InteractionState::EditingText {
                id,
                text_edit_id,
                text_buffer,
            } => {
                let comp = bd.get_component_mut(&id).unwrap();
                let text_edit_rect = comp.get_text_edit_rect(*text_edit_id, state).unwrap();
                painter.rect_filled(
                    text_edit_rect,
                    state.grid_size * 0.1,
                    ui.ctx().theme().get_stroke_color().gamma_multiply_u8(127),
                );
                show_text_edit(
                    text_edit_rect,
                    comp.is_single_line_text_edit(),
                    text_buffer,
                    state,
                    ui,
                );
            }
            InteractionState::AddingPort(id) => {
                let comp = bd.get_component(id).unwrap();
                let rect = Self::get_selection_rect(comp, state);
                painter.rect_stroke(
                    rect,
                    state.grid_size * 0.1,
                    Stroke::new(state.grid_size * 0.15, Color32::BLUE.gamma_multiply(0.25)),
                    StrokeKind::Outside,
                );
                if let Some((rotation, offset, _)) = comp.get_nearest_port_pos(state, false) {
                    // TODO: refactor it
                    let center = Port {
                        align: rotation,
                        offset: offset,
                        name: "".into(),
                    }
                    .center(&comp.get_position(), comp.get_dimension(), state);
                    painter.text(
                        center,
                        Align2::CENTER_CENTER,
                        "+",
                        FontId::monospace(state.grid_size),
                        Color32::GREEN,
                    );
                }
            }
            InteractionState::EditingPort(id) => {
                let comp = bd.get_component(id).unwrap();

                let rect = Self::get_selection_rect(comp, state);
                painter.rect_stroke(
                    rect,
                    state.grid_size * 0.1,
                    Stroke::new(state.grid_size * 0.15, Color32::GREEN.gamma_multiply(0.25)),
                    StrokeKind::Outside,
                );

                if let Some((rotation, offset, _)) = comp.get_nearest_port_pos(state, true) {
                    // TODO: refactor it
                    let center = Port {
                        align: rotation,
                        offset: offset,
                        name: "".into(),
                    }
                    .center(&comp.get_position(), comp.get_dimension(), state);
                    painter.circle_filled(
                        center,
                        state.grid_size * 0.3,
                        Color32::BLUE.gamma_multiply(0.5),
                    );
                    let theme = painter.ctx().theme();
                    let galley = painter.fonts(|fonts| {
                        fonts.layout_no_wrap(
                            "📝".into(),
                            FontId::monospace(state.grid_size),
                            theme.get_text_color(),
                        )
                    });
                    let shape = Shape::Text(TextShape::new(center, galley, theme.get_text_color()));
                    let visuals = ui.visuals();
                    let bg_rect = shape.visual_bounding_rect().scale_from_center(1.1);
                    let r = state.grid_size * 0.2;
                    painter.add(visuals.popup_shadow.as_shape(bg_rect, r));
                    painter.rect(
                        bg_rect,
                        r,
                        visuals.panel_fill,
                        visuals.window_stroke(),
                        StrokeKind::Outside,
                    );
                    painter.add(shape);
                }
            }

            InteractionState::RemovingPort(id) => {
                let comp = bd.get_component(id).unwrap();

                let rect = Self::get_selection_rect(comp, state);
                painter.rect_stroke(
                    rect,
                    state.grid_size * 0.1,
                    Stroke::new(state.grid_size * 0.15, Color32::RED.gamma_multiply(0.25)),
                    StrokeKind::Outside,
                );
                if let Some((rotation, offset, _)) = comp.get_nearest_port_pos(state, true) {
                    // TODO: refactor it
                    let center = Port {
                        align: rotation,
                        offset: offset,
                        name: "".into(),
                    }
                    .center(&comp.get_position(), comp.get_dimension(), state);
                    painter.text(
                        center,
                        Align2::CENTER_CENTER,
                        "×",
                        FontId::monospace(state.grid_size),
                        Color32::RED,
                    );
                }
            }
            InteractionState::CreatingNet => {
                self.connection_builder.draw(bd, state, painter);
            }
            _ => {}
        }
    }

    fn get_action(comp: &Component, state: &FieldState) -> ComponentAction {
        if let Some(cursor_pos) = state.cursor_pos {
            let actions = comp.get_available_actions();
            for (i, rect) in ComponentAction::actions_grid(comp, state, actions.len())
                .iter()
                .enumerate()
            {
                if rect.contains(cursor_pos) {
                    return actions[i];
                }
            }
        }
        ComponentAction::None
    }

    fn draw_actions_panel(comp: &Component, state: &FieldState, ui: &egui::Ui, painter: &Painter) {
        let actions = comp.get_available_actions();
        if !actions.is_empty() {
            let visuals = &ui.style().visuals;
            let rect = ComponentAction::actions_rect(comp, state, actions.len());
            let r = rect.height() * 0.1;
            painter.add(visuals.popup_shadow.as_shape(rect, r));
            painter.rect(
                rect,
                r,
                visuals.panel_fill,
                visuals.window_stroke(),
                StrokeKind::Outside,
            );
            let grid = ComponentAction::actions_grid(comp, state, actions.len());
            actions.iter().enumerate().for_each(|(i, act)| {
                let rect = grid[i];
                let selected = if let Some(cursor_pos) = state.cursor_pos {
                    rect.contains(cursor_pos)
                } else {
                    false
                };
                act.draw(&rect, painter, selected, visuals);
            });
        }
    }

    fn get_selection_rect(comp: &Component, state: &FieldState) -> Rect {
        let (w, h) = comp.get_dimension();
        Rect::from_min_size(
            state.grid_to_screen(&comp.get_position()),
            vec2(w as f32 * state.grid_size, h as f32 * state.grid_size),
        )
    }

    fn get_new_size(
        comp: &Component,
        state: &FieldState,
        direction: ResizeDirection,
    ) -> Option<(i32, i32)> {
        let cursor_pos = state.screen_to_grid(state.cursor_pos?);
        let component_pos = comp.get_position();
        let (w, h) = comp.get_dimension();
        match direction {
            ResizeDirection::Down => Some((w, (cursor_pos.y - component_pos.y).max(1))),
            ResizeDirection::Right => Some(((cursor_pos.x - component_pos.x).max(1), h)),
        }
    }

    fn get_resize_selection_rect(
        comp: &Component,
        state: &FieldState,
        direction: ResizeDirection,
    ) -> Option<Rect> {
        let (w, h) = Self::get_new_size(comp, state, direction)?;
        Some(Rect::from_min_size(
            state.grid_to_screen(&comp.get_position()),
            vec2(w as f32 * state.grid_size, h as f32 * state.grid_size),
        ))
    }

    fn is_right_selection_border_hovered(
        cursor_pos: Option<Pos2>,
        state: &FieldState,
        comp: &Component,
    ) -> bool {
        if let Some(cursor_pos) = cursor_pos {
            let selection_rect = Self::get_selection_rect(comp, state);
            return cursor_pos.y >= selection_rect.top()
                && cursor_pos.y < selection_rect.bottom()
                && (selection_rect.right() - cursor_pos.x).abs() < state.grid_size * 0.5;
        } else {
            false
        }
    }

    fn is_bottom_selection_border_hovered(
        cursor_pos: Option<Pos2>,
        state: &FieldState,
        comp: &Component,
    ) -> bool {
        if let Some(cursor_pos) = cursor_pos {
            let selection_rect = Self::get_selection_rect(comp, state);
            return cursor_pos.x >= selection_rect.left()
                && cursor_pos.x < selection_rect.right()
                && (selection_rect.bottom() - cursor_pos.y).abs() < state.grid_size * 0.5;
        } else {
            false
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum ResizeDirection {
    Right,
    Down,
}

enum ConnectionBuilderState {
    IDLE,
    ACTIVE {
        point: GridBDConnectionPoint,
        anchors: Vec<GridPos>,
    },
}

/// Response from connection builder
enum ConnectionBuilderResponse {
    Hovered,
    Toggled,
    /// Connection building is complete
    Complete(Transaction),
}

pub struct ConnectionBuilder {
    state: ConnectionBuilderState,
}

fn simplify_path(mut path: Vec<GridPos>) -> Vec<GridPos> {
    loop {
        let prev_size = path.len();
        let mut i = 1;
        while i < (path.len() - 1) {
            let prev = path[i - 1];
            let curr = path[i];
            let next = path[i + 1];

            let same_x = prev.x == curr.x && curr.x == next.x;
            let same_y = prev.y == curr.y && curr.y == next.y;

            if same_x || same_y {
                path.remove(i);
            } else {
                i += 1;
            }
        }
        if prev_size == path.len() {
            break;
        }
    }
    path
}

impl ConnectionBuilder {
    fn generate_full_path_by_anchors(
        &self,
        bd: &GridBD,
        target: &GridBDConnectionPoint,
    ) -> Option<Vec<GridPos>> {
        match &self.state {
            ConnectionBuilderState::ACTIVE { point, anchors } => {
                let comp1 = bd.get_component(&point.component_id)?;
                let mut result = vec![comp1.get_connection_dock_cell(point.connection_id).unwrap()];
                anchors.iter().for_each(|a| {
                    result.extend(bd.find_net_path(result.last().unwrap().clone(), a.clone())); // !!!
                    result.push(a.clone());
                });
                let target_comp = bd.get_component(&target.component_id).unwrap();
                let target_pos = target_comp
                    .get_connection_dock_cell(target.connection_id)
                    .unwrap();
                result.extend(bd.find_net_path(result.last().unwrap().clone(), target_pos.clone())); // !!!
                result.push(target_pos);
                Some(simplify_path(result))
            }
            _ => None,
        }
    }

    fn new() -> Self {
        Self {
            state: ConnectionBuilderState::IDLE,
        }
    }

    fn update(
        &mut self,
        bd: &mut GridBD,
        state: &FieldState,
        response: &Response,
    ) -> Option<ConnectionBuilderResponse> {
        if let Some(con) = bd.get_hovered_connection(&state) {
            if response.clicked() {
                if let Some(t) = self.toggle(bd, con) {
                    return Some(ConnectionBuilderResponse::Complete(t));
                } else {
                    return Some(ConnectionBuilderResponse::Toggled);
                }
            }
            return Some(ConnectionBuilderResponse::Hovered);
        } else if response.clicked() {
            if let Some(pos) = state.cursor_pos {
                self.add_anchor(state.screen_to_grid(pos));
            }
        }
        return None;
    }

    fn toggle(
        &mut self,
        bd: &mut GridBD,
        target_point: GridBDConnectionPoint,
    ) -> Option<Transaction> {
        match self.state {
            ConnectionBuilderState::IDLE => {
                self.state = ConnectionBuilderState::ACTIVE {
                    point: target_point,
                    anchors: vec![],
                };
                None
            }
            ConnectionBuilderState::ACTIVE { point, anchors: _ } => {
                let result =
                    if let Some(points) = self.generate_full_path_by_anchors(bd, &target_point) {
                        Some(Transaction::ChangeNet {
                            net_id: bd.allocate_net(),
                            old_net: None,
                            new_net: Some(Net {
                                start_point: point,
                                end_point: target_point,
                                points: points,
                            }),
                        })
                    } else {
                        None
                    };
                self.state = ConnectionBuilderState::IDLE;
                return result;
            }
        }
    }

    fn add_anchor(&mut self, cell: GridPos) {
        match &mut self.state {
            ConnectionBuilderState::ACTIVE { point: _, anchors } => anchors.push(cell),
            ConnectionBuilderState::IDLE => {}
        }
    }

    fn draw_anchors(&self, state: &FieldState, painter: &egui::Painter) {
        match &self.state {
            ConnectionBuilderState::ACTIVE { point: _, anchors } => {
                anchors.iter().for_each(|a| {
                    let r1 = Rect::from_min_size(
                        state.grid_to_screen(a),
                        vec2(state.grid_size, state.grid_size),
                    )
                    .scale_from_center(0.8);

                    let r2 = r1.scale_from_center(0.5);
                    let stroke = Stroke::new(
                        state.grid_size * 0.1,
                        painter.ctx().theme().get_anchor_color(),
                    );
                    painter.line_segment([r1.left_top(), r2.left_top()], stroke);
                    painter.line_segment([r1.left_bottom(), r2.left_bottom()], stroke);
                    painter.line_segment([r1.right_top(), r2.right_top()], stroke);
                    painter.line_segment([r1.right_bottom(), r2.right_bottom()], stroke);
                });
            }
            _ => {}
        }
    }

    // Returns true, if connection point is hovered
    pub fn draw(&self, bd: &GridBD, state: &FieldState, painter: &egui::Painter) -> bool {
        let result = if let Some(con) = bd.get_hovered_connection(&state) {
            bd.get_component(&con.component_id)
                .unwrap()
                .highlight_connection(con.connection_id, state, painter);
            true
        } else {
            false
        };
        match &self.state {
            ConnectionBuilderState::ACTIVE { point, anchors } => {
                if let Some(comp) = bd.get_component(&point.component_id) {
                    self.draw_anchors(state, painter);
                    let p1 = comp
                        .get_connection_position(point.connection_id, state)
                        .unwrap();
                    let p1_1_grid = comp.get_connection_dock_cell(point.connection_id).unwrap();
                    let mut points = vec![
                        p1,
                        state.grid_to_screen(&p1_1_grid)
                            + vec2(0.5 * state.grid_size, 0.5 * state.grid_size),
                    ];
                    let mut last_grid_p = p1_1_grid;
                    anchors.iter().for_each(|a| {
                        let path = bd.find_net_path(last_grid_p.clone(), a.clone());
                        points.extend(path.iter().map(|t| {
                            state.grid_to_screen(t)
                                + vec2(0.5 * state.grid_size, 0.5 * state.grid_size)
                        }));
                        points.push(
                            state.grid_to_screen(a)
                                + vec2(0.5 * state.grid_size, 0.5 * state.grid_size),
                        );
                        last_grid_p = a.clone();
                    });
                    if let Some(p2) = state.cursor_pos {
                        points.extend(
                            bd.find_net_path(
                                state.screen_to_grid(points.last().unwrap().clone()),
                                state.screen_to_grid(p2),
                            )
                            .iter()
                            .map(|g| {
                                state.grid_to_screen(&g)
                                    + vec2(state.grid_size * 0.5, state.grid_size * 0.5)
                            }),
                        );
                        points.push(p2);
                    } else {
                    }
                    for i in 1..points.len() {
                        if points[i - 1] != points[i] {
                            // fixme
                            painter.circle_filled(
                                points[i],
                                state.grid_size * 0.15,
                                painter.ctx().theme().get_stroke_color(),
                            );
                            painter.line_segment(
                                [points[i - 1], points[i]],
                                Stroke::new(
                                    state.grid_size * 0.3,
                                    painter.ctx().theme().get_stroke_color(),
                                ),
                            );
                        }
                    }
                }
            }
            _ => {}
        }
        result
    }

    fn is_active(&self) -> bool {
        match self.state {
            ConnectionBuilderState::IDLE => false,
            _ => true,
        }
    }
}

#[derive(Clone)]
enum Transaction {
    ChangeComponent {
        comp_id: Id,
        old_comp: Option<Component>,
        new_comp: Option<Component>,
    },
    ChangeNet {
        net_id: Id,
        old_net: Option<Net>,
        new_net: Option<Net>,
    },
    CombinedTransaction(LinkedList<Transaction>),
}

impl Transaction {
    fn apply(&mut self, bd: &mut GridBD) {
        match self {
            Transaction::CombinedTransaction(sequence) => {
                for t in sequence {
                    t.apply(bd);
                }
            }
            Transaction::ChangeComponent {
                comp_id: id,
                old_comp,
                new_comp,
            } => {
                *old_comp = bd.remove_component(&id);
                if let Some(inserting_comp) = std::mem::replace(new_comp, None) {
                    bd.insert_component(*id, inserting_comp);
                }
            }

            Transaction::ChangeNet {
                net_id,
                old_net,
                new_net,
            } => {
                *old_net = bd.remove_net(&net_id);
                if let Some(inserting_net) = std::mem::replace(new_net, None) {
                    bd.insert_net(*net_id, inserting_net);
                }
            }
        }
    }

    fn revert(&mut self, bd: &mut GridBD) {
        match self {
            Transaction::CombinedTransaction(sequence) => {
                for t in sequence.iter_mut().rev() {
                    t.revert(bd);
                }
            }
            Transaction::ChangeComponent {
                comp_id: id,
                old_comp,
                new_comp,
            } => {
                *new_comp = bd.remove_component(&id);
                if let Some(inserting_comp) = std::mem::replace(old_comp, None) {
                    bd.insert_component(*id, inserting_comp);
                }
            }
            Transaction::ChangeNet {
                net_id,
                old_net,
                new_net,
            } => {
                *new_net = bd.remove_net(&net_id);
                if let Some(inserting_net) = std::mem::replace(old_net, None) {
                    bd.insert_net(*net_id, inserting_net);
                }
            }
        }
    }
}
