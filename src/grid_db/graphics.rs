use egui::epaint::Vertex;
use egui::{Align, Align2, Color32, Mesh, Painter, Pos2, Rect, Stroke, Theme, Vec2, pos2};
use lyon::geom::point;
use lyon::{
    path::{LineCap, LineJoin, Path},
    tessellation::{
        BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
        StrokeVertex, VertexBuffers,
    },
};

use std::cell::RefCell;

use crate::field::FieldState;
use crate::grid_db::Rotation;

pub fn tesselate_polygon(
    points: &Vec<Pos2>,
    fill_color: Color32,
    stroked: bool,
    stroke_color: Color32,
    stroke_w: f32,
) -> Mesh {
    let mut builder = Path::builder();
    if let Some(first) = points.first() {
        builder.begin(point(first.x, first.y));
        for p in &points[1..] {
            builder.line_to(point(p.x, p.y));
        }
        builder.close();
    }
    let path = builder.build();

    let mut geometry: VertexBuffers<egui::epaint::Vertex, u32> = VertexBuffers::new();

    thread_local! {
        static TESSELLATOR: RefCell<FillTessellator> = RefCell::new(FillTessellator::new());
        static STROKE_TESSELLATOR: RefCell<StrokeTessellator> = RefCell::new(StrokeTessellator::new());
    }

    TESSELLATOR.with(|tessellator| {
        let mut tessellator = tessellator.borrow_mut();
        tessellator
            .tessellate_path(
                &path,
                &FillOptions::default(),
                &mut lyon::tessellation::BuffersBuilder::new(
                    &mut geometry,
                    |vertex: FillVertex| egui::epaint::Vertex {
                        pos: pos2(vertex.position().x, vertex.position().y),
                        uv: egui::epaint::WHITE_UV,
                        color: fill_color,
                    },
                ),
            )
            .expect("Tessellation failed");
    });
    let mut mesh = Mesh {
        vertices: geometry.vertices,
        indices: geometry.indices,
        texture_id: egui::TextureId::default(),
    };
    if stroked {
        let mut stroke_geometry: VertexBuffers<egui::epaint::Vertex, u32> = VertexBuffers::new();
        STROKE_TESSELLATOR.with(|tessellator| {
            let mut tessellator = tessellator.borrow_mut();
            let stroke_options = StrokeOptions::default()
                .with_line_width(stroke_w)
                .with_tolerance(0.05)
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round);
            tessellator
                .tessellate_path(
                    &path,
                    &stroke_options,
                    &mut BuffersBuilder::new(&mut stroke_geometry, |vertex: StrokeVertex| Vertex {
                        pos: pos2(vertex.position().x, vertex.position().y),
                        uv: egui::epaint::WHITE_UV,
                        color: stroke_color,
                    }),
                )
                .expect("Tessellation failed");
        });
        mesh.append(Mesh {
            vertices: stroke_geometry.vertices,
            indices: stroke_geometry.indices,
            texture_id: egui::TextureId::default(),
        });
    }
    mesh
}

pub fn mesh_line(pts: Vec<Pos2>, width: f32, color: Color32) -> Mesh {
    let half_w = width * 0.5;
    let mut mesh = Mesh::default();
    for i in 0..pts.len() - 1 {
        let start = pts[i];
        let end = pts[i + 1];

        let delta = end - start;
        let length = delta.length();
        if length == 0.0 {
            continue;
        }
        let dir = delta / length;
        let perp = Vec2::new(-dir.y, dir.x);
        let half = perp * half_w;

        let p1 = start + half - dir * half_w;
        let p2 = start - half - dir * half_w;
        let p3 = end + half + dir * half_w;
        let p4 = end - half + dir * half_w;

        let idx_base = mesh.vertices.len() as u32;

        mesh.vertices.push(Vertex {
            pos: p1,
            uv: Pos2::ZERO,
            color,
        });
        mesh.vertices.push(Vertex {
            pos: p2,
            uv: Pos2::ZERO,
            color,
        });
        mesh.vertices.push(Vertex {
            pos: p3,
            uv: Pos2::ZERO,
            color,
        });
        mesh.vertices.push(Vertex {
            pos: p4,
            uv: Pos2::ZERO,
            color,
        });

        mesh.indices.extend_from_slice(&[
            idx_base,
            idx_base + 1,
            idx_base + 2,
            idx_base + 2,
            idx_base + 1,
            idx_base + 3,
        ]);
    }
    mesh
}

pub fn svg_polygon(
    points: &Vec<Pos2>,
    fill_color: Color32,
    stroke_color: Color32,
    stroke_w: f32,
) -> String {
    fill_color.to_hex();
    let points_str = points
        .iter()
        .map(|p| format!("{} {}", p.x, p.y))
        .collect::<Vec<String>>()
        .join(" ");
    format!(
        "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\" />",
        points_str,
        fill_color.to_hex(),
        stroke_color.to_hex(),
        stroke_w
    )
}

pub fn svg_line(points: &Vec<Pos2>, color: Color32, width: f32) -> String {
    let mut path = String::new();
    path.push_str(&format!("M {} {}", points[0].x, points[0].y));

    for i in 1..points.len() - 1 {
        path.push_str(&format!(" L {} {}", points[i].x, points[i].y));
    }
    path.push_str(&format!(
        " L {} {}",
        points[points.len() - 1].x,
        points[points.len() - 1].y
    ));

    format!(
        r#"<path d="{}" stroke="{}" stroke-width="{}" fill="none"/>"#,
        path,
        color.to_hex(),
        width
    )
}

pub fn svg_circle_filled(center: Pos2, radius: f32, fill_color: Color32) -> String {
    format!(
        r#"<circle cx="{}" cy="{}" r="{}" fill="{}"/>"#,
        center.x,
        center.y,
        radius,
        fill_color.to_hex()
    )
}

#[allow(unused)]
pub fn svg_circle(
    center: Pos2,
    radius: f32,
    fill_color: Color32,
    stroke_color: Color32,
    stroke_width: f32,
) -> String {
    format!(
        r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
        center.x,
        center.y,
        radius,
        fill_color.to_hex(),
        stroke_color.to_hex(),
        stroke_width
    )
}

#[allow(unused)]
pub trait ComponentColor {
    fn get_fill_color(&self) -> Color32;
    fn get_stroke_color(&self) -> Color32;
    fn get_text_color(&self) -> Color32;
    fn get_bg_color(&self) -> Color32;
    fn get_stroke(&self, state: &FieldState) -> Stroke;
    fn get_anchor_color(&self) -> Color32;
}

pub const STROKE_SCALE: f32 = 0.1;

impl ComponentColor for Theme {
    fn get_fill_color(&self) -> Color32 {
        match self {
            Self::Dark => Color32::GRAY,
            Self::Light => Color32::WHITE,
        }
    }
    fn get_stroke_color(&self) -> Color32 {
        match self {
            Self::Dark => Color32::DARK_GRAY,
            Self::Light => Color32::BLACK,
        }
    }

    fn get_text_color(&self) -> Color32 {
        match self {
            Self::Dark => Color32::WHITE,
            Self::Light => Color32::DARK_GRAY,
        }
    }

    fn get_anchor_color(&self) -> Color32 {
        match self {
            Self::Dark => Color32::GRAY,
            Self::Light => Color32::BLACK,
        }
    }

    /// Used for SVG
    fn get_bg_color(&self) -> Color32 {
        match self {
            Self::Light => Color32::WHITE,
            Self::Dark => Color32::from_rgb(30, 30, 30),
        }
    }

    fn get_stroke(&self, state: &FieldState) -> Stroke {
        return Stroke::new(state.grid_size * STROKE_SCALE, self.get_stroke_color());
    }
}

pub fn svg_single_line_text(
    text: String,
    pos: Pos2,
    font_size: f32,
    rotation: Rotation,
    theme: Theme,
    anchor: Align2,
) -> String {
    let color = theme.get_text_color().to_hex();
    let Pos2 { x, y } = pos;
    let deg_angle = match rotation {
        Rotation::ROT0 => "0",
        Rotation::ROT90 => "90",
        Rotation::ROT180 => "180",
        Rotation::ROT270 => "270",
    };

    let text_anchor = match anchor.x() {
        Align::LEFT => "start",
        Align::Center => "middle",
        Align::RIGHT => "end",
    };

    let dominant_baseline = match anchor.y() {
        Align::TOP => "hanging",
        Align::Center => "middle",
        Align::BOTTOM => "baseline",
    };

    format!(
        r#"<text x="{x}" y="{y}" font-family="monospace" font-size="{font_size}" fill="{color}" text-anchor="{text_anchor}" dominant-baseline="{dominant_baseline}" transform="rotate({deg_angle}, {x}, {y})">{text}</text>"#
    )
}

pub fn svg_rect(pos: Pos2, (width, height): (f32, f32), stroke_w: f32, theme: Theme) -> String {
    let fill_color = theme.get_fill_color().to_hex();
    let stroke_color = theme.get_stroke_color().to_hex();
    format!(
        r#"
    <rect
        x="{}"
        y="{}"
        width="{width}"
        height="{height}"
        fill="{fill_color}"
        stroke="{stroke_color}"
        stroke-width="{stroke_w}"
    />"#,
        pos.x, pos.y,
    )
}

#[allow(unused)]
pub fn draw_dashed_line(
    painter: &Painter,
    start: Pos2,
    end: Pos2,
    color: Color32,
    stroke_width: f32,
    dash_length: f32,
    gap_length: f32,
) {
    let dir = end - start;
    let total_length = dir.length();
    let dir_normalized = dir.normalized();

    let mut current_pos = 0.0;
    while current_pos < total_length {
        let segment_start = start + dir_normalized * current_pos;
        let segment_end = start + dir_normalized * (current_pos + dash_length).min(total_length);

        painter.line_segment(
            [segment_start, segment_end],
            Stroke::new(stroke_width, color),
        );

        current_pos += dash_length + gap_length;
    }
}

#[allow(unused)]
pub fn draw_dashed_rect(
    painter: &Painter,
    rect: Rect,
    color: Color32,
    stroke_width: f32,
    dash_length: f32,
    gap_length: f32,
) {
    let top_left = rect.min;
    let top_right = Pos2::new(rect.max.x, rect.min.y);
    let bottom_right = rect.max;
    let bottom_left = Pos2::new(rect.min.x, rect.max.y);
    draw_dashed_line(
        painter,
        top_left,
        top_right,
        color,
        stroke_width,
        dash_length,
        gap_length,
    );
    draw_dashed_line(
        painter,
        top_right,
        bottom_right,
        color,
        stroke_width,
        dash_length,
        gap_length,
    );
    draw_dashed_line(
        painter,
        bottom_right,
        bottom_left,
        color,
        stroke_width,
        dash_length,
        gap_length,
    );
    draw_dashed_line(
        painter,
        bottom_left,
        top_left,
        color,
        stroke_width,
        dash_length,
        gap_length,
    );
}
