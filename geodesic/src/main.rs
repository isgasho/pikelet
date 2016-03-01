extern crate cgmath;
#[macro_use] extern crate glium;
extern crate time;

pub use glium::glutin;

use cgmath::{Angle, PerspectiveFov, Rad};
use cgmath::{Matrix4, SquareMatrix};
use cgmath::{Point3, Point};
use cgmath::{Vector3, EuclideanVector};
use glium::{BackfaceCullingMode, Depth, DepthTest, DrawParameters};
use glium::{DisplayBuild, PolygonMode, Program, Surface, VertexBuffer};
use glium::index::{PrimitiveType, NoIndices};
use glutin::{ElementState, Event, GlProfile, GlRequest, WindowBuilder};
use glutin::VirtualKeyCode as Key;

use camera::Camera;
use polyhedra::octahedron;

pub mod camera;
pub mod color;
pub mod polyhedra;
pub mod times;

#[derive(Copy, Clone)]
struct Vertex {
    normal: [f32; 3],
    position: [f32; 3],
}

implement_vertex!(Vertex, normal, position);

fn face_normal(p0: Point3<f32>, p1: Point3<f32>, p2: Point3<f32>) -> Vector3<f32> {
    let cross = (p1 - p0).cross(p2 - p0);
    cross / cross.length()
}

fn collect_vertices(points: &[Point3<f32>], faces: &[[u8; 3]]) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(faces.len() * 3);

    for face in faces {
        let p0 = points[face[0] as usize];
        let p1 = points[face[1] as usize];
        let p2 = points[face[2] as usize];
        let normal = face_normal(p0, p1, p2);

        vertices.push(Vertex { normal: normal.into(), position: p0.into() });
        vertices.push(Vertex { normal: normal.into(), position: p1.into() });
        vertices.push(Vertex { normal: normal.into(), position: p2.into() });
    }

    vertices
}

fn create_camera(rotation: Rad<f32>, (width, height): (u32, u32), ) -> Camera {
    Camera {
        position: Point3 {
            x: Rad::sin(rotation) * 2.0,
            y: Rad::cos(rotation) * 2.0,
            z: 1.0,
        },
        target: Point3::origin(),
        projection: PerspectiveFov {
            aspect: width as f32 / height as f32,
            fovy: Rad::full_turn() / 6.0,
            near: 0.1,
            far: 300.0,
        },
    }
}

fn draw_params<'a>(polygon_mode: PolygonMode) -> DrawParameters<'a> {
    DrawParameters {
        backface_culling: BackfaceCullingMode::CullClockwise,
        depth: Depth {
            test: DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        polygon_mode: polygon_mode,
        ..DrawParameters::default()
    }
}

fn main() {
    let display = WindowBuilder::new()
        .with_title("Thing".to_string())
        .with_dimensions(800, 500)
        .with_gl(GlRequest::Latest)
        .with_gl_profile(GlProfile::Core)
        .with_depth_buffer(24)
        .build_glium()
        .unwrap();

    let vertices = collect_vertices(&octahedron::points(), &octahedron::faces());
    let vertex_buffer = VertexBuffer::new(&display, &vertices).unwrap();
    let index_buffer = NoIndices(PrimitiveType::TrianglesList);

    let shaded_program =
        Program::from_source(&display,
                             include_str!("shader/shaded.v.glsl"),
                             include_str!("shader/shaded.f.glsl"),
                             None).unwrap();

    let flat_program =
        Program::from_source(&display,
                             include_str!("shader/flat.v.glsl"),
                             include_str!("shader/flat.f.glsl"),
                             None).unwrap();

    let mut camera_rotation = Rad::new(0.0);

    'main: for time in times::in_seconds() {
        if let Some(window) = display.get_window() {
            window.set_title(&format!("FPS: {:.2}",  1.0 / time.delta()));
        }

        let mut target = display.draw();

        target.clear_color_and_depth(color::DARK_GREY, 1.0);


        let light_dir = Vector3::new(0.0, 0.5, 1.0);
        let rotation_delta = Rad::new(time.delta() as f32) * 0.5;
        camera_rotation = camera_rotation + rotation_delta;
        let camera = create_camera(camera_rotation, target.get_dimensions());
        let view_proj = camera.to_mat();

        let vector3_array: fn(Vector3<f32>) -> [f32; 3] = Vector3::into;
        let matrix4_array: fn(Matrix4<f32>) -> [[f32; 4]; 4] = Matrix4::into;

        target.draw(&vertex_buffer, &index_buffer, &shaded_program,
                    &uniform! {
                        color:      color::WHITE,
                        light_dir:  vector3_array(light_dir),
                        model:      matrix4_array(Matrix4::identity()),
                        view_proj:  matrix4_array(view_proj),
                    },
                    &draw_params(PolygonMode::Fill)).unwrap();

        target.draw(&vertex_buffer, &index_buffer, &flat_program,
                    &uniform! {
                        color:      color::BLACK,
                        // Scaled to prevent depth-fighting
                        model:      matrix4_array(Matrix4::from_scale(1.001)),
                        view_proj:  matrix4_array(view_proj),
                    },
                    &draw_params(PolygonMode::Line)).unwrap();

        target.finish().unwrap();

        for ev in display.poll_events() {
            match ev {
                Event::Closed => break 'main,
                Event::KeyboardInput(ElementState::Pressed, _, Some(Key::Escape)) => break 'main,
                _ => {},
            }
        }
    }
}
