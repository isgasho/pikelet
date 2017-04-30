extern crate cgmath;
#[cfg(test)]
extern crate expectest;
extern crate find_folder;
#[macro_use]
extern crate glium;
#[macro_use]
extern crate imgui;
extern crate notify;
extern crate num_traits;
#[macro_use]
extern crate quick_error;
extern crate rand;
extern crate rayon;
extern crate time;
extern crate fnv;

extern crate dggs;
extern crate geomath;
extern crate job_queue;

use cgmath::Vector2;
use std::time::Duration;

use math::Size2;
use render::CommandList;

pub mod camera;
pub mod color;
mod game;
pub mod geom;
pub mod math;
pub mod times;
mod render;
pub mod ui;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Loop {
    Continue,
    Break,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrameMetrics {
    pub size_points: Size2<u32>,
    pub size_pixels: Size2<u32>,
    pub delta_time: f32,
}

impl FrameMetrics {
    fn frames_per_second(&self) -> f32 {
        match self.delta_time {
            0.0 => 0.0,
            delta_time => 1.0 / delta_time,
        }
    }

    fn framebuffer_scale(&self) -> Vector2<f32> {
        Vector2::new(match self.size_points.width {
                         0 => 0.0,
                         width => self.size_pixels.width as f32 / width as f32,
                     },
                     match self.size_points.height {
                         0 => 0.0,
                         height => self.size_pixels.height as f32 / height as f32,
                     })
    }

    fn aspect_ratio(&self) -> f32 {
        self.size_pixels.width as f32 / self.size_pixels.height as f32
    }
}

pub struct RenderData<Event> {
    metrics: FrameMetrics,
    is_limiting_fps: bool,
    command_list: CommandList<Event>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum UpdateEvent<InputEvent> {
    FrameRequested(FrameMetrics),
    Input(InputEvent),
}

fn create_frame_metrics(display: &glium::Display, delta_time: f32) -> FrameMetrics {
    let window = display.get_window().unwrap();
    let size_points = window.get_inner_size_points().unwrap();
    let size_pixels = window.get_inner_size_pixels().unwrap();

    FrameMetrics {
        size_points: Size2::new(size_points.0, size_points.1),
        size_pixels: Size2::new(size_pixels.0, size_pixels.1),
        delta_time: delta_time,
    }
}

macro_rules! try_or {
    ($e:expr, $or:expr) => {
        match $e { Ok(x) => x, Err(_) => $or }
    };
}

#[cfg_attr(feature = "cargo-clippy", allow(drop_copy))]
fn main() {
    use glium::DisplayBuild;
    use glium::glutin::WindowBuilder;
    use std::sync::mpsc;
    use std::thread;

    use render::Resources;

    let display = WindowBuilder::new()
        .with_title("Voyager!")
        .with_dimensions(1000, 500)
        .with_depth_buffer(24)
        .build_glium()
        .unwrap();

    let metrics = create_frame_metrics(&display, 0.0);

    let (render_tx, render_rx) = mpsc::sync_channel(1);
    let (update_tx, update_rx) = mpsc::channel();
    let (resource_tx, resource_rx) = mpsc::channel();

    thread::spawn(move || {
        use game::Game;

        let mut game = Game::init(metrics, resource_tx);

        for event in update_rx.iter() {
            let loop_result = match event {
                UpdateEvent::FrameRequested(metrics) => {
                    // We send the data for the last frame so that the renderer
                    // can get started doing it's job in parallel!
                    render_tx
                        .send(game.create_render_data())
                        .expect("Failed to send render data");

                    game.handle_frame_request(metrics)
                },
                UpdateEvent::Input(event) => game.handle_input(event),
            };

            if loop_result == Loop::Break {
                break;
            };
        }
    });

    let mut resources = Resources::new(&display);;

    'main: for time in times::in_seconds() {
        // Swap frames with update thread
        let render_data = {
            let metrics = create_frame_metrics(&display, time.delta() as f32);
            let update_event = UpdateEvent::FrameRequested(metrics);

            try_or!(update_tx.send(update_event), break 'main);
            try_or!(render_rx.recv(), break 'main)
        };

        // Get user input
        for event in display.poll_events() {
            resources.handle_ui_event(event.clone());
            let update_event = UpdateEvent::Input(event.into());
            try_or!(update_tx.send(update_event), break 'main);
        }

        // Update resources
        while let Ok(event) = resource_rx.try_recv() {
            resources.handle_resource_event(event);
        }

        // Render frame
        let mut frame = display.draw();

        resources
            .draw(&mut frame,
                  render_data.metrics,
                  render_data.command_list,
                  |event| drop(update_tx.send(UpdateEvent::Input(event))))
            .unwrap();

        frame.finish().unwrap();

        if render_data.is_limiting_fps {
            thread::sleep(Duration::from_millis(10));
        }
    }
}
