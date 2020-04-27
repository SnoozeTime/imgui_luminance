use imgui::{Context, FontSource, FontConfig, FontGlyphRanges};
use luminance_windowing::{WindowOpt, WindowDim};
use luminance::pipeline::PipelineState;
use luminance_glfw::{WindowEvent, Key, Action, GlfwSurface, Surface};
use luminance::context::GraphicsContext;

fn main() {
    // First thing first: we create a new surface to render to and get events from.
    let mut surface = GlfwSurface::new(
        WindowDim::Windowed(1800, 800),
        "Hello, World",
        WindowOpt::default(),
    ).unwrap();

    //// The back buffer, which we will make our render into (we make it mutable so that we can change
    //// it whenever the window dimensions change).
    let mut back_buffer = surface.back_buffer().unwrap();
    let mut resize = false;

    let mut imgui = Context::create();
    let font_size = 13.0;

    imgui.fonts().add_font(&[
        FontSource::TtfData {
            data: include_bytes!("../resources/mplus-1p-regular.ttf"),
            size_pixels: font_size,
            config: Some(FontConfig {
                rasterizer_multiply: 1.75,
                glyph_ranges: FontGlyphRanges::default(),
                ..FontConfig::default()
            }),
        },
    ]);

    let mut renderer = imgui_luminance::Renderer::new(&mut surface, &mut imgui);
    imgui.set_ini_filename(None);

    // ============================================================================================
    'app: loop {
        // For all the events on the surface.
        for event in surface.poll_events() {
            match event {
                // If we close the window or press escape, quit the main loop (i.e. quit the application).
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => break 'app,
                // Handle window resizing.
                WindowEvent::FramebufferSize(..) => {
                    resize = true;
                }

                event => renderer.handle_event(imgui.io_mut(), &event),
            }
        }

        if resize {
            // Simply ask another backbuffer at the right dimension (no allocation / reallocation).
            back_buffer = surface.back_buffer().unwrap();
            resize = false;
        }

        let ui = imgui.frame();

        //ui.show_demo_window(&mut true);
        // draw something.
        ui.show_demo_window(&mut true);
        let draw_data = ui.render();

        // prepare the buffers with draw data
        renderer.prepare(&mut surface, draw_data);

        // Create a new dynamic pipeline that will render to the back buffer and must clear it with
        // pitch black prior to do any render to it.
        surface.pipeline_builder().pipeline(&back_buffer, &PipelineState::default(), |pipeline, mut shd_gate| {
            renderer.render( &pipeline, &mut shd_gate, draw_data);
        });
        surface.swap_buffers();
    }
}
