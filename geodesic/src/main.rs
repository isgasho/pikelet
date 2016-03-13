extern crate cgmath;
#[macro_use] extern crate glium;
extern crate rusttype;
extern crate time;

use cgmath::{Angle, PerspectiveFov, Rad};
use cgmath::{Matrix4, SquareMatrix};
use cgmath::{Point2, Point3, Point};
use cgmath::{Vector2, Vector3, Vector};
use glium::{DisplayBuild, Frame, IndexBuffer, Program, VertexBuffer};
use glium::{DrawParameters, PolygonMode, Surface};
use glium::backend::Context;
use glium::index::{PrimitiveType, NoIndices};
use rusttype::Font;
use std::mem;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use camera::Camera;
use geom::Geometry;
use text::TextTexture;

mod macros;

pub mod camera;
pub mod color;
pub mod geom;
pub mod index;
pub mod input;
pub mod math;
pub mod text;
pub mod times;

const WINDOW_TITLE: &'static str = "Geodesic Test";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 500;

const CAMERA_XZ_RADIUS: f32 = 2.0;
const CAMERA_Y_HEIGHT: f32 = 1.0;
const CAMERA_NEAR: f32 = 0.1;
const CAMERA_FAR: f32 = 1000.0;
const CAMERA_ZOOM_FACTOR: f32 = 10.0;
const CAMERA_DRAG_FACTOR: f32 = 10.0;

const POLYHEDRON_SUBDIVS: usize = 1;

const LIGHT_DIR: Vector3<f32> = Vector3 { x: 0.0, y: 1.0, z: 0.2 };

macro_rules! include_resource {
    (shader: $path:expr) => { include_str!(concat!("../resources/shaders/", $path)) };
    (font: $path:expr) => { include_bytes!(concat!("../resources/fonts/", $path)) };
}

const FLAT_SHADED_VERT: &'static str = include_resource!(shader: "flat_shaded.v.glsl");
const FLAT_SHADED_FRAG: &'static str = include_resource!(shader: "flat_shaded.f.glsl");
const TEXT_VERT: &'static str = include_resource!(shader: "text.v.glsl");
const TEXT_FRAG: &'static str = include_resource!(shader: "text.f.glsl");
const UNSHADED_VERT: &'static str = include_resource!(shader: "unshaded.v.glsl");
const UNSHADED_FRAG: &'static str = include_resource!(shader: "unshaded.f.glsl");

const BLOGGER_SANS_FONT: &'static [u8] = include_resource!(font: "blogger/Blogger Sans.ttf");

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 3],
}

implement_vertex!(Vertex, position);

pub fn create_delaunay_vertices(geometry: &Geometry) -> Vec<Vertex> {
    const VERTICES_PER_FACE: usize = 3;

    let mut vertices = Vec::with_capacity(geometry.faces.len() * VERTICES_PER_FACE);

    for face in &geometry.faces {
        let n0 = index::get(&geometry.nodes, face.nodes[0]).position;
        let n1 = index::get(&geometry.nodes, face.nodes[1]).position;
        let n2 = index::get(&geometry.nodes, face.nodes[2]).position;

        vertices.push(Vertex { position: n0.into() });
        vertices.push(Vertex { position: n1.into() });
        vertices.push(Vertex { position: n2.into() });
    }

    vertices
}

pub fn create_voronoi_vertices(geometry: &Geometry) -> Vec<Vertex> {
    // const MAX_FACES_PER_NODE: usize = 6;
    // const VERTICES_PER_FACE: usize = 3;

    let mut vertices = Vec::with_capacity(geometry.faces.len());

    for face in geometry.faces.iter() {
        let n0 = index::get(&geometry.nodes, face.nodes[0]).position;
        let n1 = index::get(&geometry.nodes, face.nodes[1]).position;
        let n2 = index::get(&geometry.nodes, face.nodes[2]).position;
        let mut points = Vec::with_capacity(3);
        points.push(n0);
        points.push(n1);
        points.push(n2);
        let centroid = math::centroid(&points);
        vertices.push(Vertex { position: centroid.into() });
    }

    // for (i, node) in geometry.nodes.iter().enumerate() {
    //     let midpoints: Vec<_> =
    //         geometry.adjacent_nodes(geom::NodeIndex(i)).iter()
    //             .map(|n| math::midpoint(node.position, n.position))
    //             .collect();

    //     let centroid = math::centroid(&midpoints);
    //     vertices.push(Vertex { position: centroid.into() });

    //     let first = midpoints[0];
    //     let mut prev = first;

    //     for &curr in midpoints[1..].iter().chain(Some(&first)) {
    //         vertices.push(Vertex { position: centroid.into() });
    //         vertices.push(Vertex { position: curr.into() });
    //         vertices.push(Vertex { position: prev.into() });

    //         prev = curr;
    //     }
    // }

    vertices
}

enum Loop {
    Continue,
    Break,
}

struct State {
    frames_per_second: f32,

    is_wireframe: bool,
    is_showing_mesh: bool,
    is_dragging: bool,
    is_zooming: bool,

    mouse_position: Point2<i32>,
    new_mouse_position: Option<Point2<i32>>,
    window_dimensions: (u32, u32),

    light_dir: Vector3<f32>,

    camera_rotation: Rad<f32>,
    camera_rotation_delta: Rad<f32>,
    camera_distance: f32,
}

impl State {
    fn update<Events>(&mut self, actions: Events, delta_time: f32) -> Loop where
        Events: IntoIterator,
        Events::Item: Into<input::Event>,
    {
        for action in actions {
            use input::Event::*;

            match action.into() {
                CloseApp => return Loop::Break,
                ToggleMesh => self.is_wireframe = !self.is_wireframe,
                ToggleWireframe => self.is_showing_mesh = !self.is_showing_mesh,
                DragStart => self.is_dragging = true,
                DragEnd => self.is_dragging = false,
                ZoomStart => self.is_zooming = true,
                ZoomEnd => self.is_zooming = false,
                MousePosition(position) => self.new_mouse_position = Some(position),
                Resize(width, height) => self.window_dimensions = (width, height),
                NoOp => {},
            }
        }

        self.frames_per_second = 1.0 / delta_time;

        let mouse_position_delta = match self.new_mouse_position.take() {
            Some(new_position) => {
                let old_position = mem::replace(&mut self.mouse_position, new_position);
                new_position - old_position
            },
            None => Vector2::zero(),
        };

        if self.is_dragging {
            let (window_width, _) = self.window_dimensions;
            let rotations_per_second = (mouse_position_delta.x as f32 / window_width as f32) * CAMERA_DRAG_FACTOR;
            self.camera_rotation_delta = Rad::full_turn() * rotations_per_second * delta_time;
        }

        self.camera_rotation = self.camera_rotation - self.camera_rotation_delta;

        if self.is_zooming {
            let zoom_delta = mouse_position_delta.x as f32 * delta_time;
            self.camera_distance = self.camera_distance - (zoom_delta * CAMERA_ZOOM_FACTOR);
        }

        Loop::Continue
    }

    fn create_camera(&self, (target_width, target_height): (u32, u32)) -> Camera {
        Camera {
            position: Point3 {
                x: Rad::sin(self.camera_rotation) * self.camera_distance,
                y: CAMERA_Y_HEIGHT,
                z: Rad::cos(self.camera_rotation) * self.camera_distance,
            },
            target: Point3::origin(),
            projection: PerspectiveFov {
                aspect: target_width as f32 / target_height as f32,
                fovy: Rad::full_turn() / 6.0,
                near: CAMERA_NEAR,
                far: CAMERA_FAR,
            },
        }
    }
}

fn draw_params<'a>(polygon_mode: PolygonMode) -> DrawParameters<'a> {
    use glium::{BackfaceCullingMode, Depth, DepthTest};
    use glium::draw_parameters::{Smooth};

    DrawParameters {
        backface_culling: BackfaceCullingMode::CullClockwise,
        depth: Depth {
            test: DepthTest::IfLess,
            write: true,
            ..Depth::default()
        },
        polygon_mode: polygon_mode,
        line_width: Some(0.5),
        point_size: Some(5.0),
        smooth: Some(Smooth::Nicest),
        ..DrawParameters::default()
    }
}

struct Resources {
    context: Rc<Context>,

    hidpi_factor: f32,

    delaunay_vertex_buffer: VertexBuffer<Vertex>,
    voronoi_vertex_buffer: VertexBuffer<Vertex>,
    index_buffer: NoIndices,

    text_vertex_buffer: VertexBuffer<text::Vertex>,
    text_index_buffer: IndexBuffer<u8>,

    flat_shaded_program: Program,
    text_program: Program,
    unshaded_program: Program,

    blogger_sans_font: Font<'static>,
}

impl Resources {
    pub fn render_text(&self, target: &mut Frame, text_texture: &TextTexture, color: (f32, f32, f32, f32)) {
        use glium::texture::Texture2d;
        use glium::uniforms::MagnifySamplerFilter;

        let proj = {
            let (target_width, target_height) = target.get_dimensions();
            cgmath::ortho(0.0, target_width as f32, target_height as f32, 0.0, -1.0, 1.0)
        };

        let model = {
            let scale_x = text_texture.width as f32 / text::TEXTURE_WIDTH;
            let scale_y = text_texture.height as f32 / text::TEXTURE_HEIGHT;

            Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0)
        };

        let text = Texture2d::new(&self.context, text_texture).unwrap();

        let params = {
            use glium::Blend;
            use glium::BlendingFunction::Addition;
            use glium::LinearBlendingFactor::*;

            let blending_function = Addition {
                source: SourceAlpha,
                destination: OneMinusSourceAlpha
            };

            DrawParameters {
                blend: Blend {
                    color: blending_function,
                    alpha: blending_function,
                    constant_value: (1.0, 1.0, 1.0, 1.0),
                },
                ..DrawParameters::default()
            }
        };

        target.draw(
            &self.text_vertex_buffer,
            &self.text_index_buffer,
            &self.text_program,
            &uniform! {
                color:    color,
                text:     text.sampled().magnify_filter(MagnifySamplerFilter::Nearest),
                proj:     math::array_m4(proj),
                model:    math::array_m4(model),
            },
            &params,
        ).unwrap();
    }
}

fn render(resources: &Resources, mut target: Frame, state: &State) {
    let camera = state.create_camera(target.get_dimensions());
    let view_matrix = camera.view_matrix();
    let proj_matrix = camera.projection_matrix();
    let eye_position = camera.position;

    let draw_unshaded = |target: &mut Frame, vertex_buffer, color, polygon_mode| {
        target.draw(
            vertex_buffer,
            &resources.index_buffer,
            &resources.unshaded_program,
            &uniform! {
                color:      color,
                model:      math::array_m4(Matrix4::from_scale(1.025)),
                view:       math::array_m4(view_matrix),
                proj:       math::array_m4(proj_matrix),
            },
            &draw_params(polygon_mode),
        )
    };

    let draw_flat_shaded = |target: &mut Frame, vertex_buffer, color, polygon_mode| {
        target.draw(
            vertex_buffer,
            &resources.index_buffer,
            &resources.flat_shaded_program,
            &uniform! {
                color:      color,
                light_dir:  math::array_v3(state.light_dir),
                model:      math::array_m4(Matrix4::identity()),
                view:       math::array_m4(view_matrix),
                proj:       math::array_m4(proj_matrix),
                eye:        math::array_p3(eye_position),
            },
            &draw_params(polygon_mode),
        )
    };

    target.clear_color_and_depth(color::BLUE, 1.0);

    if state.is_showing_mesh {
        draw_unshaded(&mut target, &resources.delaunay_vertex_buffer,
                      color::RED, PolygonMode::Point).unwrap();
        draw_unshaded(&mut target, &resources.voronoi_vertex_buffer,
                      color::YELLOW, PolygonMode::Point).unwrap();
        draw_unshaded(&mut target, &resources.voronoi_vertex_buffer,
                      color::WHITE, PolygonMode::Line).unwrap();
    }

    let polygon_mode = if state.is_wireframe { PolygonMode::Line } else { PolygonMode::Fill };
    draw_flat_shaded(&mut target, &resources.delaunay_vertex_buffer,
                     color::GREEN, polygon_mode).unwrap();

    let fps_text = TextTexture::new(
        &resources.blogger_sans_font,
        &format!("FPS: {}", state.frames_per_second),
        12.0 * resources.hidpi_factor,
    );

    resources.render_text(&mut target, &fps_text, color::BLACK);

    target.finish().unwrap();
}

fn main() {
    use glium::backend::Facade;
    use glium::glutin::WindowBuilder;

    let display = WindowBuilder::new()
        .with_title(WINDOW_TITLE.to_string())
        .with_dimensions(WINDOW_WIDTH, WINDOW_HEIGHT)
        .with_depth_buffer(24)
        .build_glium()
        .unwrap();

    let mut state = State {
        frames_per_second: 0.0,

        is_wireframe: false,
        is_showing_mesh: true,
        is_dragging: false,
        is_zooming: false,

        light_dir: LIGHT_DIR,

        mouse_position: Point2::origin(),
        new_mouse_position: None,
        window_dimensions: (WINDOW_WIDTH, WINDOW_HEIGHT),

        camera_rotation: Rad::new(0.0),
        camera_rotation_delta: Rad::new(0.0),
        camera_distance: CAMERA_XZ_RADIUS,
    };

    let resources = {
        use rusttype::FontCollection;

        let hidpi_factor = display.get_window()
            .map(|window| window.hidpi_factor())
            .unwrap_or(1.0);
        let geometry = geom::icosahedron().subdivide(POLYHEDRON_SUBDIVS);
        let font_collection = FontCollection::from_bytes(BLOGGER_SANS_FONT);

        Resources {
            context: display.get_context().clone(),

            hidpi_factor: hidpi_factor,

            delaunay_vertex_buffer: VertexBuffer::new(&display, &create_delaunay_vertices(&geometry)).unwrap(),
            voronoi_vertex_buffer: VertexBuffer::new(&display, &create_voronoi_vertices(&geometry)).unwrap(),
            index_buffer: NoIndices(PrimitiveType::TrianglesList),

            text_vertex_buffer: VertexBuffer::new(&display, &text::TEXTURE_VERTICES).unwrap(),
            text_index_buffer: IndexBuffer::new(&display, PrimitiveType::TrianglesList, &text::TEXTURE_INDICES).unwrap(),

            flat_shaded_program: Program::from_source(&display, FLAT_SHADED_VERT, FLAT_SHADED_FRAG, None).unwrap(),
            text_program: Program::from_source(&display, TEXT_VERT, TEXT_FRAG, None).unwrap(),
            unshaded_program: Program::from_source(&display, UNSHADED_VERT, UNSHADED_FRAG, None).unwrap(),

            blogger_sans_font: font_collection.into_font().unwrap(),
        }
    };

    for time in times::in_seconds() {
        let events = display.poll_events();
        let delta_time = time.delta() as f32;

        match state.update(events, delta_time) {
            Loop::Break => break,
            Loop::Continue => render(&resources, display.draw(), &state),
        }

        thread::sleep(Duration::from_millis(10)); // battery saver ;)
    }
}
