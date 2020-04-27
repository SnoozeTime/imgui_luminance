use imgui::{DrawData, FontAtlasTexture, DrawVert, DrawCmd, DrawCmdParams, Textures, TextureId, Io};
use luminance_glfw::{Key, WindowEvent, Action, MouseButton, GlfwSurface};
use luminance_windowing::Surface;
use luminance::texture::{Sampler, Texture, Dim2, GenMipmaps, MagFilter, MinFilter};
use luminance::pixel::{NormRGBA8UI, NormUnsigned};
use luminance_derive::{Semantics, Vertex, UniformInterface};
use luminance::shader::program::{Uniform, Program};
use luminance::linear::M44;
use luminance::pipeline::{BoundTexture, ShadingGate, Pipeline};
use luminance::tess::{TessBuilder, Mode, TessSliceIndex, Tess};
use luminance::context::GraphicsContext;
use luminance::render_state::RenderState;
use luminance::blending::{Equation, Factor};

/**
in vec2 pos;
in vec2 uv;
in vec4 col;
*/
#[derive(Clone, Copy, Debug, Eq, PartialEq, Semantics)]
pub enum Semantics {
    #[sem(name = "pos", repr = "[f32; 2]", wrapper = "VertexPosition")]
    Position,
    #[sem(name = "uv", repr = "[f32; 2]", wrapper = "VertexTexCoord")]
    TexCoord,
    #[sem(name = "col", repr = "[u8; 4]", wrapper = "VertexColor")]
    Color,
}

const VERT_SHADER: &str = include_str!("shaders/vert.glsl");
const FRAG_SHADER: &str = include_str!("shaders/frag.glsl");

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Vertex)]
#[vertex(sem = "Semantics")]
struct Vertex {
    position: VertexPosition,
    #[vertex(normalized = "true")]
    color: VertexColor,
    tex_coord: VertexTexCoord,
}

impl From<&DrawVert> for Vertex {
    fn from(v: &DrawVert) -> Self {
        Self {
            position: VertexPosition::new(v.pos),
            color: VertexColor::new(v.col),
            tex_coord: VertexTexCoord::new(v.uv),
        }
    }
}

#[derive(UniformInterface)]
struct ShaderInterface {
    pub matrix: Uniform<M44>,

    #[uniform(unbound)]
    pub tex: Uniform<&'static BoundTexture<'static, Dim2, NormUnsigned>>,
}


pub struct Renderer {
    font_texture: Texture<Dim2, NormRGBA8UI>,
    program: Program<Semantics, (), ShaderInterface>,
    textures: Textures<Texture<Dim2, NormRGBA8UI>>,

    // to Render
    tesses: Vec<Tess>,
}

impl Renderer {
    pub fn new(surface: &mut GlfwSurface, imgui: &mut imgui::Context) -> Self {
        Renderer::setup_iomut(surface, imgui);
        let mut font_atlas = imgui.fonts();
        font_atlas.tex_id = std::usize::MAX.into();
        let font = font_atlas.build_rgba32_texture();
        let program = Program::from_strings(None, VERT_SHADER, None, FRAG_SHADER)
            .unwrap()
            .ignore_warnings();

        let textures = Textures::new();
        Renderer {
            font_texture: Renderer::upload_texture(surface, font),
            program,
            textures,
            tesses: vec![],
        }
    }

    fn lookup_texture(&self, texture_id: TextureId) -> Result<&Texture<Dim2, NormRGBA8UI>, String> {
        if texture_id.id() == std::usize::MAX {
            Ok(&self.font_texture)
        } else if let Some(texture) = self.textures.get(texture_id) {
            Ok(texture)
        } else {
            Err(format!("BAD TEXTURE {:?}", texture_id))
        }
    }

    /// prepare the buffers for rendering.
    pub fn prepare<C>(&mut self, surface: &mut C,  draw_data: &DrawData) where C: GraphicsContext {
        self.tesses.clear();
        for draw_list in draw_data.draw_lists() {
            let vtx_buffer = draw_list.vtx_buffer().iter().map(Vertex::from).collect::<Vec<_>>();
            let indices = draw_list.idx_buffer();
            let tess = TessBuilder::new(surface).set_indices(indices).add_vertices(vtx_buffer).set_mode(Mode::Triangle).build().unwrap();
            self.tesses.push(tess);
        }
    }

    /// Render the prepared buffer to the screen.
    pub fn render<C>(&self, pipeline: &Pipeline, shd_gate: &mut ShadingGate<C>,  draw_data: &DrawData) where C: GraphicsContext {
        let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        if !(fb_width > 0.0 && fb_height > 0.0) {
            return;
        }
        let left = draw_data.display_pos[0];
        let right = draw_data.display_pos[0] + draw_data.display_size[0];
        let top = draw_data.display_pos[1];
        let bottom = draw_data.display_pos[1] + draw_data.display_size[1];
        let matrix = [
            [(2.0 / (right - left)), 0.0, 0.0, 0.0],
            [0.0, (2.0 / (top - bottom)), 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [
                (right + left) / (left - right),
                (top + bottom) / (bottom - top),
                0.0,
                1.0,
            ],
        ];
        let clip_off = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;


        shd_gate.shade(&self.program, |iface, mut rdr_gate| {

            for (i, draw_list) in draw_data.draw_lists().enumerate() {

                let tess = &self.tesses[i];

                let mut idx_start = 0;

                // Projection and texture are common.
                iface.matrix.update(matrix);

                rdr_gate.render(&RenderState::default()
                    .set_blending((
                        Equation::Additive,
                        Factor::SrcAlpha,
                        Factor::SrcAlphaComplement,
                    )).set_depth_test(None), |mut tess_gate| {
                    for cmd in draw_list.commands() {
                        match cmd {
                            DrawCmd::Elements {
                                count,
                                cmd_params:
                                DrawCmdParams {
                                    clip_rect,
                                    texture_id,
                                    ..
                                },
                            } => {
                                // TODO
                                let idx_end = idx_start + count;
                                let clip_rect = [
                                    (clip_rect[0] - clip_off[0]) * clip_scale[0],
                                    (clip_rect[1] - clip_off[1]) * clip_scale[1],
                                    (clip_rect[2] - clip_off[0]) * clip_scale[0],
                                    (clip_rect[3] - clip_off[1]) * clip_scale[1],
                                ];
                                if clip_rect[0] < fb_width
                                    && clip_rect[1] < fb_height
                                    && clip_rect[2] >= 0.0
                                    && clip_rect[3] >= 0.0
                                {
                                    let texture = pipeline.bind_texture(&self.lookup_texture(texture_id).unwrap_or(&self.font_texture));

                                    iface.tex.update(&texture);

                                    tess_gate.render(tess.slice(idx_start..idx_end));
                                }
                                idx_start = idx_end;
                            }
                            _ => ()
                        }
                    }
                })
            }
            });
    }

    /// Handle the events from the user (keyboard, mouse click...). The event type is Glfw currently
    /// so it only supports this backend.
    pub fn handle_event(&self, io: &mut Io, event: &WindowEvent) {

        match event {
            WindowEvent::Key(_, code, action, _) => {
                io.keys_down[*code as usize] = *action == Action::Press;
            },
            WindowEvent::Char(ch) => {
                // Exclude the backspace key ('\u{7f}'). Otherwise we will insert this char and then
                // delete it.
                if *ch != '\u{7f}' {
                    io.add_input_character(*ch)
                }
            }
            WindowEvent::CursorPos(x, y) => {
                io.mouse_pos = [*x as f32, *y as f32]
            }
            WindowEvent::Scroll(x, y) => {
                io.mouse_wheel = *y as f32;
                io.mouse_wheel_h = *x as f32;
            }
            WindowEvent::MouseButton(button, action, _) => {
                let pressed = *action == Action::Press;
                match button {
                    MouseButton::Button1 => io.mouse_down[0] = pressed,
                    MouseButton::Button2 => io.mouse_down[1] = pressed,
                    MouseButton::Button3 => io.mouse_down[2] = pressed,
                    _ => (),
                }
            }
            _ => ()
        }
    }

    fn setup_iomut(surface: &mut GlfwSurface, imgui: &mut imgui::Context) {
        let io = imgui.io_mut();
        io.font_global_scale = 1.0;
        io.display_size = [surface.size()[0] as f32, surface.size()[1] as f32];
        println!("IO DISPLAY SIZE = {:?}", io.display_size);
        io[imgui::Key::Tab] = Key::Tab.get_scancode().unwrap() as _;
        io[imgui::Key::LeftArrow] = Key::Left.get_scancode().unwrap() as _;
        io[imgui::Key::RightArrow] = Key::Right.get_scancode().unwrap() as _;
        io[imgui::Key::UpArrow] = Key::Up.get_scancode().unwrap() as _;
        io[imgui::Key::DownArrow] = Key::Down.get_scancode().unwrap() as _;
        io[imgui::Key::PageUp] = Key::PageUp.get_scancode().unwrap() as _;
        io[imgui::Key::PageDown] = Key::PageDown.get_scancode().unwrap() as _;
        io[imgui::Key::Home] = Key::Home.get_scancode().unwrap() as _;
        io[imgui::Key::End] = Key::End.get_scancode().unwrap() as _;
        io[imgui::Key::Insert] = Key::Insert.get_scancode().unwrap() as _;
        io[imgui::Key::Delete] = Key::Delete.get_scancode().unwrap() as _;
        io[imgui::Key::Backspace] = Key::Backspace.get_scancode().unwrap() as _;
        io[imgui::Key::Space] = Key::Space.get_scancode().unwrap() as _;
        io[imgui::Key::Enter] = Key::Enter.get_scancode().unwrap() as _;
        io[imgui::Key::Escape] = Key::Escape.get_scancode().unwrap() as _;
        io[imgui::Key::KeyPadEnter] = Key::KpEnter.get_scancode().unwrap() as _;
        io[imgui::Key::A] = Key::A.get_scancode().unwrap() as _;
        io[imgui::Key::C] = Key::C.get_scancode().unwrap() as _;
        io[imgui::Key::V] = Key::V.get_scancode().unwrap() as _;
        io[imgui::Key::X] = Key::X.get_scancode().unwrap() as _;
        io[imgui::Key::Y] = Key::Y.get_scancode().unwrap() as _;
        io[imgui::Key::Z] = Key::Z.get_scancode().unwrap() as _;
    }

    fn upload_texture(surface:&mut GlfwSurface, font: FontAtlasTexture) -> Texture<Dim2, NormRGBA8UI> {
        // create the luminance texture; the third argument is the number of mipmaps we want (leave it
        // to 0 for now) and the latest is the sampler to use when sampling the texels in the
        // shader (we’ll just use the default one)
        let mut sampler = Sampler::default();
        sampler.mag_filter = MagFilter::Linear;
        sampler.min_filter = MinFilter::Linear;
        let tex = Texture::new(surface, [font.width, font.height], 0, sampler)
            .expect("luminance texture creation");

        tex.upload_raw(GenMipmaps::No, font.data).unwrap();

        tex
    }
}