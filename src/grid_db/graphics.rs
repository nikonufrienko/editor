use egui::{Color32, Mesh, Pos2, Shape, pos2};
use lyon::geom::point;
use lyon::{
    path::{LineCap, LineJoin, Path},
    tessellation::{
        BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
        StrokeVertex, VertexBuffers,
    },
};
use std::{cell::RefCell, sync::Arc};

pub fn get_concave_polygon_shape(
    points: Vec<Pos2>,
    fill_color: Color32,
    stroke_color: Color32,
    stroke_w: f32,
) -> Shape {
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
                &mut BuffersBuilder::new(&mut stroke_geometry, |vertex: StrokeVertex| {
                    egui::epaint::Vertex {
                        pos: pos2(vertex.position().x, vertex.position().y),
                        uv: egui::epaint::WHITE_UV,
                        color: stroke_color,
                    }
                }),
            )
            .expect("Tessellation failed");
    });
    mesh.append(Mesh {
        vertices: stroke_geometry.vertices,
        indices: stroke_geometry.indices,
        texture_id: egui::TextureId::default(),
    });
    egui::Shape::Mesh(Arc::new(mesh))
}
