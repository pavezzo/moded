pub mod renderer;
pub mod shader;
pub mod font;
pub mod editor;
pub mod gap_buffer;
pub mod vim_commands;

use std::path::Path;

use editor::{Editor, EditorMode};
use font::CharacterCache;
use gap_buffer::LinePos;
use glfw::{self};
use glfw::Context;
use gl::{self};

use ab_glyph::{self, Font, ScaleFont};

use nalgebra::*;
use renderer::{highlight_line, DrawLine, DrawRect, RectRenderer, TextRenderer};
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
    }
}


pub struct CursorPos {
    pub x: usize,
    pub y: usize,
    pub wanted_x: usize,
}

impl CursorPos {
    pub fn to_screen_position(&self, state: &State, start_line: usize) -> (f32, f32) {
        // xpos, ypos
        let xpos = (self.x - 1) as f32 * state.char_width;
        let ypos = state.height as f32 - ((self.y - start_line) as f32 * state.char_height);
        
        return (xpos, ypos);
    }

    pub fn to_linepos(&self) -> LinePos {
        LinePos { line: self.y - 1, col: self.x - 1 }
    }
    
    pub fn to_line_col(&self) -> (usize, usize) {
        (self.y - 1, self.x - 1)
    }
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
}

impl State {
    pub fn max_rows(&self) -> usize {
        (self.height as f32 / self.char_height).floor() as usize
    }

    pub fn max_cols(&self) -> usize {
        (self.width as f32 / self.char_width) as usize
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
            state.io.modifiers |= modifiers;
        },
        glfw::WindowEvent::Char(c) => {
            state.io.chars.push(c);
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

//static mut WIDTH: u32 = 1280 * 2;
//static mut HEIGHT: u32 = 720 * 2;

fn main() {
    let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
    let (screen_width, screen_height) =
    unsafe {
        let vid_mode = glfw::ffi::glfwGetVideoMode(glfw::ffi::glfwGetPrimaryMonitor());
        ((*vid_mode).width as u32, (*vid_mode).height as u32)
    };

    let (mut window, events) = glfw.create_window(screen_width / 2, screen_height / 2, "moded", glfw::WindowMode::Windowed).unwrap();
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

    let mut state = State { width: screen_width as i32 / 2, height: screen_height as i32 / 2, window_changed_size: true, char_scale: 50.0, char_width: 0.0, char_height: 0.0, cursor: CursorPos {x: 1, y: 1, wanted_x: 1}, io: Io { chars: String::new(), special_keys: Vec::new(), modifiers: glfw::Modifiers::empty() } };

    let char_cache = CharacterCache::from_font_bytes(&state, include_bytes!("../fonts/JetBrainsMono-Medium.ttf"));
    state.char_width = char_cache.get('W').unwrap().width;
    state.char_height = char_cache.get(' ').unwrap().height;

    let (font_ascent, _font_descent, font_height) = {
        let font = ab_glyph::FontRef::try_from_slice(include_bytes!("../fonts/JetBrainsMono-Medium.ttf")).unwrap();
        (font.as_scaled(state.char_scale).ascent(), font.as_scaled(state.char_scale).descent(), font.as_scaled(state.char_scale).height())
    };

    let mut text_renderer = TextRenderer::new(text_shader, char_cache, font_height, font_ascent);
    let rect_renderer = RectRenderer::new(rect_shader);

    println!("font_height: {font_height}");
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
            window.set_should_close(true);
        }

        editor.handle_input(&mut state);

        start_line = if state.cursor.y > start_line && state.cursor.y - start_line > state.max_rows() {
            state.cursor.y - state.max_rows()
        } else if state.cursor.y <= start_line {
            start_line - 1
        } else {
            start_line
        };

        if editor.mode == EditorMode::Visual {
            let start = editor.visual_range_anchor.min(editor.visual_range_moving);
            let end = editor.visual_range_anchor.max(editor.visual_range_moving);

            if start.line == end.line {
                let rect = highlight_line(&state, start.col, end.col, start.line, start_line);
                rect_renderer.draw_rect(&state, rect);
            } else {
                let line_len = editor.buffer.line_len(start.line).max(1);
                let first = highlight_line(&state, start.col, line_len - 1, start.line, start_line);
                rect_renderer.draw_rect(&state, first);

                for line in (start.line + 1)..end.line {
                    let line_len = editor.buffer.line_len(line).max(1);
                    let rect = highlight_line(&state, 0, line_len - 1, line, start_line);
                    rect_renderer.draw_rect(&state, rect);
                }

                let last = highlight_line(&state, 0, end.col, end.line, start_line);
                rect_renderer.draw_rect(&state, last);
            }
        }

        let end_line = start_line + state.max_rows() + 1;
        for i in (start_line as usize)..(editor.buffer.total_lines().min(end_line as usize)) {
            let line = editor.buffer.line(i);
            let draw_line = DrawLine::new(&line, i + 1 - start_line, (1.0, 1.0, 1.0));
            text_renderer.draw_line(&state, draw_line);
        }

        let (xpos, ypos) = state.cursor.to_screen_position(&state, start_line);
        let rect = DrawRect::from_screen_points(&state, xpos, ypos, (1.0, 1.0, 1.0));
        rect_renderer.draw_rect(&state, rect);

        //println!();
        //for line in 0..editor.buffer.total_lines() {
        //    println!("{line}: {:?}", editor.buffer.raw_line(line).as_bytes());
        //}


        unsafe {
            gl::BindVertexArray(0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        state.io.reset();
        window.swap_buffers();
    }
}
