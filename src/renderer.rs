use crate::{shader::{RectShader, TextShader}, CharacterCache, State};

pub struct DrawLine<'a> {
    pub text: &'a str,
    pub linenr: usize,
    pub color: (f32, f32, f32),
}

impl<'a> DrawLine<'a> {
    pub fn new(text: &'a str, linenr: usize, color: (f32, f32, f32)) -> Self {
        Self { text, linenr, color }
    }
}


pub struct TextRenderer {
    pub shader: TextShader,
    pub char_cache: CharacterCache,
    pub vao: u32,
    pub vbo: u32,
    pub font_height: f32,
    pub font_ascent: f32,
}

impl TextRenderer {
    pub fn new(shader: TextShader, char_cache: CharacterCache, font_height: f32, font_ascent: f32) -> Self {
        // vao / vbo for texture quads
        let mut vertex_array_object = 0;
        let mut vertex_buffer_object = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut vertex_array_object);
            gl::GenBuffers(1, &mut vertex_buffer_object);
            gl::BindVertexArray(vertex_array_object);
            gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer_object);
            gl::BufferData(gl::ARRAY_BUFFER, std::mem::size_of::<f32>() as isize * 6 * 4, 0 as *const _, gl::DYNAMIC_DRAW);
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 4, gl::FLOAT, gl::FALSE, 4 * std::mem::size_of::<f32>() as i32, 0 as *const _);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        Self { shader, char_cache, vao: vertex_array_object, vbo: vertex_buffer_object, font_height, font_ascent }
    }

    pub fn draw_line(&mut self, state: &State, line: DrawLine) {
        self.shader.use_program();

        let mut x = 0f32;
        for ch in line.text.chars() {
            // colors
            unsafe {
                let uniform_location = gl::GetUniformLocation(self.shader.id, c"textColor".as_ptr().cast());
                assert!(uniform_location != -1);

                gl::Uniform3f(uniform_location, line.color.0, line.color.1, line.color.2);
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindVertexArray(self.vao);
            }

            //let (xpos, ypos) = (0f32, 100f32);
            let c = if let Some(c) = self.char_cache.get(ch) {
                c
            } else {
                self.char_cache.try_insert(ch);
                let Some(c) = self.char_cache.get(ch) else {continue;};
                c
            };
            //let c = self.char_cache.get(ch).unwrap();

            let xadvance = ((state.char_width - c.width) / 2.0).max(0.0);
            let (h, w) = (c.height, c.width);
            let (xpos, ypos) = (x + xadvance, state.height as f32 - self.font_ascent - c.position_max_y - (self.font_height * (line.linenr - 1) as f32));

            let vertices: [[f32; 4]; 6] = [
                [xpos,     ypos + h, 0.0, 0.0],
                [xpos,     ypos,     0.0, 1.0],
                [xpos + w, ypos,     1.0, 1.0],

                [xpos,     ypos + h, 0.0, 0.0],
                [xpos + w, ypos,     1.0, 1.0],
                [xpos + w, ypos + h, 1.0, 0.0],
            ];

            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, c.texture_id);
                gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
                // std::mem::size_of_val(&vertices) as isize
                //gl::BufferSubData(gl::ARRAY_BUFFER, 0, 4 * 6 * std::mem::size_of::<f32>() as isize, vertices.as_ptr().cast());
                gl::BufferSubData(gl::ARRAY_BUFFER, 0, std::mem::size_of_val(&vertices) as isize, vertices.as_ptr().cast());

                gl::BindBuffer(gl::ARRAY_BUFFER, 0);
                gl::DrawArrays(gl::TRIANGLES, 0, 6);
            }

            //const CHAR_SPACE: f32 = 5.0;
            x += state.char_width;
            //x += c.width + char_space;
            //x += c.advance_horizontal - c.bearing_horizontal;
        }
    }
}


pub struct DrawRect {
    pub height: f32,
    pub width: f32,
    pub xpos: f32,
    pub ypos: f32,
    pub color: (f32, f32, f32),
}

impl DrawRect {
    pub fn new(height: f32, width: f32, xpos: f32, ypos: f32, color: (f32, f32, f32)) -> Self {
        Self { height, width, xpos, ypos, color }
    }

    pub fn from_screen_points(state: &State, height: f32, width: f32, xpos: f32, ypos: f32, color: (f32, f32, f32)) -> Self {
        // -1.0, -1.0 = down left
        //let width = ((width * 2.0) / state.width as f32) - 1.0;
        //let height = ((height * 2.0) / state.height as f32) - 1.0;
        let xpos = ((xpos * 2.0) / state.width as f32) - 1.0;
        let ypos = ((ypos * 2.0) / state.height as f32) - 1.0;
        let width = (width * 2.0) / state.width as f32;
        let height = (height * 2.0) / state.height as f32;
        Self { height, width, xpos, ypos, color }
    }
}


pub struct RectRenderer {
    pub shader: RectShader,
    pub vao: u32,
    pub vbo: u32,
}

impl RectRenderer {
    pub fn new(shader: RectShader) -> Self {
        // vao / vbo for texture quads
        let mut vertex_array_object = 0;
        let mut vertex_buffer_object = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut vertex_array_object);
            gl::GenBuffers(1, &mut vertex_buffer_object);
            gl::BindVertexArray(vertex_array_object);
            gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer_object);
            gl::BufferData(gl::ARRAY_BUFFER, std::mem::size_of::<f32>() as isize * 6 * 3, 0 as *const _, gl::DYNAMIC_DRAW);
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as i32, 0 as *const _);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        Self { shader, vao: vertex_array_object, vbo: vertex_buffer_object }
    }

    pub fn draw_rect(&self, _state: &State, rect: DrawRect) {
        self.shader.use_program();
        unsafe {
            let uniform_location = gl::GetUniformLocation(self.shader.id, c"rectColor".as_ptr().cast());
            assert!(uniform_location != -1);

            gl::Uniform3f(uniform_location, rect.color.0, rect.color.1, rect.color.2);
        }

        let (h, w) = (rect.height, rect.width);
        let (xpos, ypos) = (rect.xpos, rect.ypos);


        let vertices: [[f32; 3]; 6] = [
            [xpos,     ypos + h, 0.0],
            [xpos,     ypos,     0.0],
            [xpos + w, ypos,     0.0],

            [xpos,     ypos + h, 0.0],
            [xpos + w, ypos,     0.0],
            [xpos + w, ypos + h, 0.0],
        ];

        unsafe {
            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(gl::ARRAY_BUFFER, std::mem::size_of_val(&vertices) as isize, vertices.as_ptr().cast(), gl::STATIC_DRAW);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);

            gl::BindVertexArray(0);
        }
    }
}
