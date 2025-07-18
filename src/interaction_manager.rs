use crate::{
    field::{blocked_cell, filled_cells, FieldState},
    grid_db::{
        grid_pos, Component, ComponentAction, ComponentColor, GridBD, GridBDConnectionPoint, GridPos, Id, Net, RotationDirection
    },
};
use egui::{
    vec2, Color32, CursorIcon, Painter, Pos2, Rect, Response, Stroke, StrokeKind, TextEdit, Ui, UiBuilder, Vec2
};

#[derive(PartialEq)]
enum InteractionState {
    Idle,
    NetDragged { net_id: Id, segment_id: Id },
    ComponentSelected(Id),
    ComponentDragged { id: Id, grab_ofs: Vec2 },
    Resizing { id: Id, direction: ResizeDirection },
    EditingText { id: Id, text_edit_id: Id },
}

pub struct InteractionManager {
    state: InteractionState,
    drag_delta: Vec2,
}

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

impl InteractionManager {
    pub fn new() -> Self {
        Self {
            state: InteractionState::Idle,
            drag_delta: vec2(0.0, 0.0),
        }
    }

    fn move_net_segment(&self, cursor_grid_pos: &GridPos, bd: &mut GridBD) {
        match self.state {
            InteractionState::NetDragged { net_id, segment_id } => {
                let GridPos { x, y } = cursor_grid_pos;
                let mut net = bd.remove_net(&net_id).unwrap();
                let p1 = net.points[segment_id];
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
                bd.add_net(net);
            }
            _ => {}
        }
    }

    fn move_net_connection_point(
        comp_id: Id,
        net_id: Id,
        bd: &mut GridBD,
        delta_x: i32,
        delta_y: i32,
    ) {
        let mut net = bd.remove_net(&net_id).unwrap();
        let pts_len = net.points.len();

        if pts_len >= 2 {
            if comp_id == net.start_point.component_id && comp_id == net.end_point.component_id {
                // Move all points if component is connected to both ends
                for i in 0..net.points.len() {
                    net.points[i] = net.points[i] + grid_pos(delta_x, delta_y);
                }
            } else if comp_id == net.start_point.component_id {
                // Handle component connected to start of net
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
            } else if comp_id == net.end_point.component_id {
                // Handle component connected to end of net
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
        bd.add_net(net);
    }

    fn move_component(&self, comp_id: Id, bd: &mut GridBD, new_pos: GridPos) {
        let comp = bd.get_component(&comp_id).unwrap();

        if bd.is_available_location(new_pos, comp.get_dimension(), comp_id) {
            let old_pos = comp.get_position();
            let delta_y = new_pos.y - old_pos.y;
            let delta_x = new_pos.x - old_pos.x;

            for net_id in bd.get_connected_nets(&comp_id) {
                Self::move_net_connection_point(comp_id, net_id, bd, delta_x, delta_y);
            }

            let mut comp = bd.remove_component(&comp_id).unwrap();
            comp.set_pos(new_pos);
            bd.insert_component(comp_id, comp);
        }
    }

    fn rotate_component(&self, comp_id: Id, bd: &mut GridBD, dir: RotationDirection) {
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
            let connections_ids: Vec<Id> = nets_ids
                .iter()
                .map(|it| {
                    let net = bd.nets.get(it).unwrap();
                    if net.end_point.component_id == comp_id {
                        net.end_point.connection_id
                    } else {
                        net.start_point.connection_id
                    }
                })
                .collect();

            for (i, net_id) in nets_ids.iter().enumerate() {
                let old_pos = comp.get_connection_dock_cell(connections_ids[i]).unwrap();
                let new_pos = rotated_comp
                    .get_connection_dock_cell(connections_ids[i])
                    .unwrap();
                let delta_y = new_pos.y - old_pos.y;
                let delta_x = new_pos.x - old_pos.x;
                Self::move_net_connection_point(comp_id, *net_id, bd, delta_x, delta_y);
            }

            bd.remove_component(&comp_id).unwrap();
            bd.insert_component(comp_id, rotated_comp);
        }
    }

    fn apply_resize(bd: &mut GridBD, comp_id: Id, new_size: (i32, i32)) {
        let comp = bd.get_component(&comp_id).unwrap();

        if bd.is_available_location(comp.get_position(), new_size, comp_id) {
            let mut comp = bd.remove_component(&comp_id).unwrap();
            comp.set_size(new_size);
            bd.insert_component(comp_id, comp);
        }
    }

    /// Refreshes action state.
    /// Returns false if no action performed.
    pub fn refresh(
        &mut self,
        bd: &mut GridBD,
        state: &FieldState,
        response: &Response,
        ui: &egui::Ui,
    ) -> bool {
        match self.state {
            InteractionState::NetDragged { net_id, segment_id } => {
                if let Some(hover_pos) = state.cursor_pos {
                    let segment = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(segment_id, net_id)
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
                        self.move_net_segment(&state.screen_to_grid(hover_pos), bd);
                        self.state = InteractionState::Idle
                    }
                }
            }
            InteractionState::Idle => {
                if let Some(segment) = bd.get_hovered_segment(state) {
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
                let mut action = Self::get_action(comp, state);
                if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                    action = ComponentAction::Remove;
                }
                if response.clicked() && action != ComponentAction::None {
                    match action {
                        ComponentAction::RotateUp => {
                            self.rotate_component(id, bd, RotationDirection::Up);
                            self.state = InteractionState::Idle;
                        }
                        ComponentAction::RotateDown => {
                            self.rotate_component(id, bd, RotationDirection::Down);
                            self.state = InteractionState::Idle;
                        }
                        ComponentAction::Remove => {
                            bd.remove_component_with_connected_nets(&id);
                            self.state = InteractionState::Idle;
                            return true;
                        }
                        _ => {}
                    }
                    return true;
                } else if comp.is_hovered(state) {
                    if let Some(text_edit_id) = comp.get_hovered_text_edit_id() {
                        ui.ctx().output_mut(|o| o.cursor_icon = CursorIcon::Text);
                        if response.clicked() {
                            self.state = InteractionState::EditingText {
                                id: id,
                                text_edit_id,
                            };
                            return true;
                        }
                    } else {
                        ui.ctx().output_mut(|o| o.cursor_icon = CursorIcon::Grab);
                    }

                    // Check dragging:
                    if response.dragged() {
                        if let Some(hovepos) = response.hover_pos() {
                            self.state = InteractionState::ComponentDragged {
                                id,
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
                            id: id,
                            direction: ResizeDirection::Right,
                        };
                        return true;
                    }
                } else if resizable && bottom_border_hovered {
                    ui.ctx()
                        .output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
                    if response.is_pointer_button_down_on() {
                        self.state = InteractionState::Resizing {
                            id: id,
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
                        self.move_component(id, bd, state.screen_to_grid(pos - grab_ofs));
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
                    if let Some(new_size) = Self::get_new_size(comp, state, direction) {
                        Self::apply_resize(bd, id, new_size);
                    }
                    self.state = InteractionState::Idle;
                }
                return true;
            }
            InteractionState::EditingText { id, text_edit_id } => {
                let comp = bd.get_component(&id).unwrap();
                let text_edit_rect = comp.get_text_edit_rect(text_edit_id, state).unwrap();

                if response.clicked() {
                    if let Some(cursor_pos) = state.cursor_pos {
                        if !text_edit_rect.contains(cursor_pos) {
                            self.state = InteractionState::Idle;
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn draw(&mut self, bd: &mut GridBD, state: &FieldState, painter: &Painter, ui: &mut Ui) {
        match self.state {
            InteractionState::NetDragged { net_id, segment_id } => {
                let ofs = vec2(0.5, 0.5) * state.grid_size;
                if let Some(pos) = state.cursor_pos {
                    let GridPos { x, y } = state.screen_to_grid(pos);
                    let segment = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(segment_id, net_id)
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
                        .get_segment(segment_id + 1, net_id)
                    {
                        pts.push(state.grid_to_screen(&next_segment.pos2) + ofs);
                    } else {
                        pts.push(state.grid_to_screen(&segment.pos2) + ofs);
                    }
                    if let Some(prev_segment) = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(segment_id.wrapping_sub(1), net_id)
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
                if let Some(seg) = bd.get_hovered_segment(state) {
                    seg.highlight(state, &painter);
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
                        pos - grab_ofs,
                        Some(id),
                        ui.visuals().strong_text_color().gamma_multiply(0.08),
                        comp,
                    );
                }
            }
            InteractionState::Resizing { id, direction } => {
                if let Some(comp) = bd.get_component(&id) {
                    if let Some(resize_rect) =
                        Self::get_resize_selection_rect(comp, state, direction)
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
            InteractionState::EditingText { id, text_edit_id } => {
                let comp = bd.get_component_mut(&id).unwrap();
                let text_edit_rect = comp.get_text_edit_rect(text_edit_id, state).unwrap();
                ui.scope_builder(UiBuilder::new().max_rect(text_edit_rect), |ui| {
                    ui.add_sized(text_edit_rect.size(),
                    TextEdit::multiline(comp.get_text_edit_mut(text_edit_id).unwrap())
                        .background_color(if state.debounce {Color32::TRANSPARENT} else {ui.ctx().theme().get_bg_color()})
                        .text_color(if state.debounce {Color32::TRANSPARENT} else {ui.ctx().theme().get_text_color()})
                        .lock_focus(true)
                        .font(egui::FontId::monospace(state.grid_size * 0.5 )));
                });
                painter.ctx().request_repaint();
            }
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
    ACTIVE,
}

pub struct ConnectionBuilder {
    state: ConnectionBuilderState,
    point: Option<GridBDConnectionPoint>,
    anchors: Vec<GridPos>,
}

fn simplify_path(path: Vec<GridPos>) -> Vec<GridPos> {
    let mut cleaned = Vec::with_capacity(path.len());
    cleaned.push(path[0]);
    for (i, &point) in path.iter().enumerate().skip(1) {
        if point != *cleaned.last().unwrap() || i == path.len() - 1 {
            cleaned.push(point);
        }
    }

    if cleaned.len() <= 2 {
        return cleaned;
    }

    let mut i = 1;
    while i < cleaned.len().saturating_sub(1) {
        let prev = cleaned[i - 1];
        let curr = cleaned[i];
        let next = cleaned[i + 1];

        let same_x = prev.x == curr.x && curr.x == next.x;
        let same_y = prev.y == curr.y && curr.y == next.y;

        if same_x || same_y {
            cleaned.remove(i);
        } else {
            i += 1;
        }
    }

    cleaned
}

impl ConnectionBuilder {
    fn generate_full_path_by_anchors(
        &self,
        bd: &GridBD,
        target: &GridBDConnectionPoint,
    ) -> Option<Vec<GridPos>> {
        let cp = &self.point?;
        let comp1 = bd.get_component(&cp.component_id)?;
        let mut result = vec![comp1.get_connection_dock_cell(cp.connection_id).unwrap()];
        self.anchors.iter().for_each(|a| {
            result.extend(bd.find_net_path(result.last().unwrap().clone(), a.clone())); // !!!
            result.push(a.clone());
        });
        let target_comp = bd.get_component(&target.component_id).unwrap();
        let target_pos = target_comp
            .get_connection_dock_cell(target.connection_id)
            .unwrap();
        result.extend(bd.find_net_path(result.last().unwrap().clone(), target_pos.clone())); // !!!
        result.push(target_pos);
        return Some(simplify_path(result));
    }

    pub fn new() -> Self {
        Self {
            state: ConnectionBuilderState::IDLE,
            point: None,
            anchors: vec![],
        }
    }

    pub fn update(
        &mut self,
        bd: &mut GridBD,
        state: &FieldState,
        response: &Response,
        painter: &egui::Painter,
    ) {
        if let Some(con) = bd.get_hovered_connection(&state) {
            let comp = bd.get_component(&con.component_id).unwrap();
            comp.highlight_connection(con.connection_id, state, painter);
            if response.clicked() {
                self.toggle(bd, con);
            }
        } else if response.clicked() {
            if let Some(pos) = state.cursor_pos {
                self.add_anchor(state.screen_to_grid(pos));
            }
        }
    }

    pub fn toggle(&mut self, bd: &mut GridBD, point: GridBDConnectionPoint) {
        match self.state {
            ConnectionBuilderState::IDLE => {
                self.point = Some(point);
                self.state = ConnectionBuilderState::ACTIVE;
            }
            ConnectionBuilderState::ACTIVE => {
                let old_point = self.point.clone().unwrap();
                self.state = ConnectionBuilderState::IDLE;
                if let Some(points) = self.generate_full_path_by_anchors(bd, &point) {
                    bd.add_net(Net {
                        start_point: old_point,
                        end_point: point,
                        points: points,
                    });
                }
                self.point = None;
                self.anchors.clear();
            }
        }
    }

    fn add_anchor(&mut self, cell: GridPos) {
        match self.state {
            ConnectionBuilderState::ACTIVE => self.anchors.push(cell),
            ConnectionBuilderState::IDLE => {}
        }
    }

    fn draw_anchors(&self, state: &FieldState, painter: &egui::Painter) {
        self.anchors.iter().for_each(|a| {
            let r1 = Rect::from_min_size(
                state.grid_to_screen(a),
                vec2(state.grid_size, state.grid_size),
            )
            .scale_from_center(0.8);

            let r2 = r1.scale_from_center(0.5);
            let stroke = Stroke::new(state.grid_size * 0.1, Color32::GRAY);
            painter.line_segment([r1.left_top(), r2.left_top()], stroke);
            painter.line_segment([r1.left_bottom(), r2.left_bottom()], stroke);
            painter.line_segment([r1.right_top(), r2.right_top()], stroke);
            painter.line_segment([r1.right_bottom(), r2.right_bottom()], stroke);
        });
    }

    pub fn draw(&self, bd: &GridBD, state: &FieldState, painter: &egui::Painter) {
        if let Some(point) = &self.point {
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
                self.anchors.iter().for_each(|a| {
                    let path = bd.find_net_path(last_grid_p.clone(), a.clone());
                    points.extend(path.iter().map(|t| {
                        state.grid_to_screen(t) + vec2(0.5 * state.grid_size, 0.5 * state.grid_size)
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
                            Color32::DARK_GRAY,
                        );
                        painter.line_segment(
                            [points[i - 1], points[i]],
                            Stroke::new(state.grid_size * 0.3, Color32::DARK_GRAY),
                        );
                    }
                }
            }
        }
    }
}
