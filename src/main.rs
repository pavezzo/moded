pub mod renderer;
pub mod shader;
pub mod font;
pub mod editor;
pub mod gap_buffer;
pub mod vim_commands;

use std::path::Path;

use editor::Editor;
use font::CharacterCache;
use glfw::{self};
use glfw::Context;
//use glfw::{Action, Key};
//use gl::{self, QUERY_TARGET};
use gl::{self};

use ab_glyph::{self, Font, ScaleFont};

use nalgebra::*;
use renderer::{DrawLine, DrawRect, RectRenderer, TextRenderer};
use shader::{RectShader, TextShader};


const TEXT_VERTEX_SHADER_SOURCE: &str = "#version 330 core
layout (location = 0) in vec4 vertex; // <vec2 pos, vec2 tex>
out vec2 TexCoords;

uniform mat4 projection;

void main()
{
    gl_Position = projection * vec4(vertex.xy, 0.0, 1.0);
    TexCoords = vertex.zw;
}";

const TEXT_FRAGMENT_SHADER_SOURCE: &str = "#version 330 core
in vec2 TexCoords;
out vec4 color;

uniform sampler2D text;
uniform vec3 textColor;

void main()
{    
    vec4 sampled = vec4(1.0, 1.0, 1.0, texture(text, TexCoords).r);
    color = vec4(textColor, 1.0) * sampled;
}";

const RECT_VERTEX_SHADER_SOURCE: &str ="#version 330 core
layout (location = 0) in vec3 position; // vec3 pos


void main()
{
    gl_Position = vec4(position, 1.0);
}";

const RECT_FRAGMENT_SHADER_SOURCE: &str = "#version 330 core
out vec4 color;

uniform vec3 rectColor;

void main()
{    
    color = vec4(rectColor, 1.0);
}";



//
//fn frame_buffer_size_callback(_window: &mut glfw::Window, width: i32, height: i32) {
//    unsafe { gl::Viewport(0, 0, width, height) };
//}

#[derive(PartialEq, Eq, Debug)]
pub enum SpecialKey {
    Backspace,
    Enter,
    Escape,
    Control,
    Tab,
}

#[derive(Debug)]
pub struct Io {
    pub chars: String,
    pub special_keys: Vec<SpecialKey>,
    pub modifiers: glfw::Modifiers,
}

impl Io {
    pub fn pressed_char(&self, wanted: char) -> bool {
        self.chars.contains(wanted)
    }

    pub fn pressed_special(&self, wanted: SpecialKey) -> bool {
        for item in &self.special_keys {
            if *item == wanted {
                return true
            }
        }

        false
    }

    pub fn pressed_char_and_special(&self, c: char, s: SpecialKey) -> bool {
        self.pressed_char(c) && self.pressed_special(s)
    }

    pub fn pressed_char_with_modifiers(&self, wanted: char, modifiers: glfw::Modifiers) -> bool {
        self.chars.contains(wanted) && self.modifiers.contains(modifiers)
    }

    pub fn pressed_special_with_modifiers(&self, wanted: SpecialKey, modifiers: glfw::Modifiers) -> bool {
        for item in &self.special_keys {
            if *item == wanted && self.modifiers.contains(modifiers) {
                return true
            }
        }

        false
    }

    pub fn reset(&mut self) {
        self.chars.clear();
        self.special_keys.clear();
        //self.modifiers &= glfw::Modifiers::empty();
    }
}


pub struct CursorPos {
    pub x: u32,
    pub y: u32,
    pub wanted_x: u32,
}

impl CursorPos {
    pub fn to_screen_position(&self, state: &State, start_line: u32) -> (f32, f32) {
        // xpos, ypos
        let xpos = (self.x - 1) as f32 * state.char_width;
        let ypos = state.height as f32 - ((self.y - start_line) as f32 * state.char_height);
        
        return (xpos, ypos);
    }
}

#[derive(PartialEq, Eq)]
pub enum KeyListeningMode {
    Keys,
    KeysAndIgnoreOneChar,
    KeysAndChars,
}

pub struct State {
    pub width: i32,
    pub height: i32,
    pub window_changed_size: bool,
    pub io: Io,
    pub char_scale: f32,
    pub char_width: f32,
    pub char_height: f32,
    pub cursor: CursorPos,
    pub key_listening_mode: KeyListeningMode,
}

impl State {
    pub fn max_rows(&self) -> u32 {
        (self.height as f32 / self.char_height).floor() as u32
    }

    pub fn max_cols(&self) -> u32 {
        (self.width as f32 / self.char_width) as u32
    }
}


fn process_event(state: &mut State, _window: &mut glfw::Window, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(key, _scancode, glfw::Action::Press | glfw::Action::Repeat, modifiers) => {
            match key {
                glfw::Key::Backspace => state.io.special_keys.push(SpecialKey::Backspace),
                glfw::Key::Enter => state.io.special_keys.push(SpecialKey::Enter),
                glfw::Key::Tab => state.io.special_keys.push(SpecialKey::Tab),
                glfw::Key::Escape => state.io.special_keys.push(SpecialKey::Escape),
                glfw::Key::LeftControl | glfw::Key::RightControl => state.io.special_keys.push(SpecialKey::Control),
                // dumb glfw doesn't report ctrl + char in charmods polling
                key if key as i32 >= glfw::Key::A as i32 && key as i32 <= glfw::Key::Z as i32 => {
                    if modifiers.contains(glfw::Modifiers::Control) {
                        state.io.chars.push((b'a' + (key as i32 - glfw::Key::A as i32) as u8) as char);
                        state.io.special_keys.push(SpecialKey::Control);
                    }
                }
                _ => {},
            }
            ////state.io.keys.push(key);
            //println!("key: {key:?}, modifiers: {:?}", modifiers);
            state.io.modifiers |= modifiers;
        },
        glfw::WindowEvent::Char(c) => {
            if state.key_listening_mode == KeyListeningMode::KeysAndChars {
                //let mut buf = [0; 4];
                //let res = c.encode_utf8(&mut buf);
                //let len = res.len();
                //state.io.chars.extend_from_slice(&buf[0..len]);
                state.io.chars.push(c);
            } else if state.key_listening_mode == KeyListeningMode::KeysAndIgnoreOneChar {
                state.key_listening_mode = KeyListeningMode::KeysAndChars;
            }
        },
        glfw::WindowEvent::FramebufferSize(w, h) => {
            state.width = w;
            state.height = h;
            state.window_changed_size = true;
            unsafe { gl::Viewport(0, 0, w, h) };
        },
        _ => {},
    }
}

const WIDTH: u32 = 1280 * 2;
const HEIGHT: u32 = 720 * 2;

fn main() {
    let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));

    let (mut window, events) = glfw.create_window(WIDTH, HEIGHT, "moded", glfw::WindowMode::Windowed).unwrap();
    window.make_current();
    window.set_key_polling(true);
    window.set_char_polling(true);
    //window.set_char_mods_polling(true);
    window.set_framebuffer_size_polling(true);
    glfw.set_swap_interval(glfw::SwapInterval::None);
    //window.set_framebuffer_size_callback(frame_buffer_size_callback);

    gl::load_with(|ptr| window.get_proc_address(ptr) as *const _);

    unsafe {
        gl::Enable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    }

    let text_shader = TextShader::new(TEXT_VERTEX_SHADER_SOURCE, TEXT_FRAGMENT_SHADER_SOURCE).unwrap();

    let rect_shader = RectShader::new(RECT_VERTEX_SHADER_SOURCE, RECT_FRAGMENT_SHADER_SOURCE).unwrap();
    //let projection = Matrix4::new_orthographic(0.0f32, WIDTH as f32, 0.0, HEIGHT as f32, -1.0, 1.0);
    //rect_shader.use_program();
    //unsafe { 
    //    gl::UniformMatrix4fv(gl::GetUniformLocation(rect_shader.id, "projection".as_ptr().cast()), 1, gl::FALSE, projection.as_ptr()) 
    //}

    //let mut state = State { width: WIDTH as i32, height: HEIGHT as i32, window_changed_size: true, io: Default::default(), char_scale: 70.0, char_width: char_cache.get(' ').unwrap().width, char_height: char_cache.get(' ').unwrap().height, cursor: CursorPos {x: 1, y: 1} };
    let mut state = State { width: WIDTH as i32, height: HEIGHT as i32, window_changed_size: true, char_scale: 50.0, char_width: 0.0, char_height: 0.0, cursor: CursorPos {x: 1, y: 1, wanted_x: 1}, key_listening_mode: KeyListeningMode::KeysAndChars, io: Io { chars: String::new(), special_keys: Vec::new(), modifiers: glfw::Modifiers::empty() } };
    let char_cache = CharacterCache::from_font_bytes(&state, include_bytes!("../JetBrainsMono-Medium.ttf"));
    state.char_width = char_cache.get('W').unwrap().width;
    state.char_height = char_cache.get(' ').unwrap().height;

    let (font_ascent, _font_descent, font_height) = {
        let font = ab_glyph::FontRef::try_from_slice(include_bytes!("../JetBrainsMono-Medium.ttf")).unwrap();
        (font.as_scaled(state.char_scale).ascent(), font.as_scaled(state.char_scale).descent(), font.as_scaled(state.char_scale).height())
    };


    let mut text_renderer = TextRenderer::new(text_shader, char_cache, font_height, font_ascent);
    let rect_renderer = RectRenderer::new(rect_shader);

    println!("font_height: {font_height}");
    //let mut editor = Editor::from_path(Path::new(&"./src/main.rs"));
    //let mut editor = Editor::from_path(Path::new(&"./test.txt"));
    let mut editor = if let Some(arg) = std::env::args().skip(1).next() {
        let p = Path::new(&arg);
        Editor::from_path(&p)
    } else {
        Editor::from_path(Path::new(&"./Cargo.toml"))
    };
    //let mut editor = Editor::from_path(Path::new(&"./Cargo.toml"));
    let mut start_line = 0;

    while !window.should_close() {
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            process_event(&mut state, &mut window, event);
        }

        unsafe { 
            gl::ClearColor(0.16, 0.16, 0.16, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        if state.window_changed_size {
            let projection = Matrix4::new_orthographic(0.0f32, state.width as f32, 0.0, state.height as f32, -1.0, 1.0);
            text_renderer.shader.use_program();
            unsafe { 
                gl::UniformMatrix4fv(gl::GetUniformLocation(text_renderer.shader.id, c"projection".as_ptr().cast()), 1, gl::FALSE, projection.as_ptr()) 
            }
            state.window_changed_size = false;
        }

        if state.io.pressed_char_and_special('q', SpecialKey::Control) {
        //if state.io.pressed_char_with_modifiers(b'q', glfw::Modifiers::Control) {
            window.set_should_close(true);
        }

        //println!("io: {:?}", state.io);

        editor.handle_input(&mut state);

        start_line = if state.cursor.y > start_line && state.cursor.y - start_line > state.max_rows() {
            state.cursor.y - state.max_rows()
        } else if state.cursor.y <= start_line {
            start_line - 1
        } else {
            start_line
        };

        let end_line = start_line + state.max_rows() + 1;
        for i in (start_line as usize)..(editor.buffer.total_lines().min(end_line as usize)) {
            let line = editor.buffer.line(i);
            let draw_line = DrawLine::new(&line, i as u32 + 1 - start_line, (1.0, 1.0, 1.0));
            text_renderer.draw_line(&state, draw_line);
        }

        let (xpos, ypos) = state.cursor.to_screen_position(&state, start_line);
        let rect = DrawRect::from_screen_points(&state, state.char_height, state.char_width, xpos, ypos, (1.0, 1.0, 1.0));
        rect_renderer.draw_rect(&state, rect);

        unsafe {
            gl::BindVertexArray(0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        state.io.reset();
        window.swap_buffers();
    }

    //unsafe { glfw::ffi::glfwTerminate() };
}
