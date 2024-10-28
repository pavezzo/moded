use std::collections::HashMap;

use ab_glyph::{Font, ScaleFont};

use crate::State;

pub struct Character {
    pub texture_id: u32,
    pub width: f32,
    pub height: f32,
    pub bearing_horizontal: f32,
    pub bearing_vertical: f32,
    pub advance_horizontal: f32,
    pub advance_vertical: f32,
    pub position_min_x: f32,
    pub position_min_y: f32,
    pub position_max_x: f32,
    pub position_max_y: f32,
}


pub struct CharacterCache {
    map: HashMap<char, Character>,
    font: ab_glyph::FontVec,
    char_scale: f32,
}

impl CharacterCache {
    pub fn from_font_bytes(state: &State, font_bytes: &[u8]) -> Self {
        unsafe { gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1) };

        let font = ab_glyph::FontVec::try_from_vec(font_bytes.to_vec()).unwrap();
        let mut map = HashMap::new();

        for ch in ' '..='~' {
            let glyph = font.glyph_id(ch).with_scale(state.char_scale);

            let outline = font.outline_glyph(glyph.clone());
            let bounds = if let Some(outline) = &outline {
                outline.px_bounds()
            } else {
                font.glyph_bounds(&glyph)
            };

            let position_min_x = bounds.min.x;
            let position_min_y = bounds.min.y;
            let position_max_x = bounds.max.x;
            let position_max_y = bounds.max.y;

            let mut pixels = vec![0u8; (bounds.width() as usize) * (bounds.height() as usize)];
            // do this because space doesn't have outline glyph
            if let Some(outline) = outline {
                outline.draw(|x, y, coverage| {
                    let ind = (y as usize * bounds.width() as usize) + x as usize;
                    pixels[ind] = (coverage * 255.0) as u8;
                });
            }

            let texture = unsafe { Self::register_character_texture(&pixels, bounds.width() as i32, bounds.height() as i32) };

            let character = Character {
                texture_id: texture,
                width: bounds.width(),
                height: bounds.height(),
                bearing_horizontal: font.as_scaled(state.char_scale).h_side_bearing(font.glyph_id(ch)), 
                bearing_vertical: font.as_scaled(state.char_scale).v_side_bearing(font.glyph_id(ch)),
                advance_horizontal: font.as_scaled(state.char_scale).h_advance(font.glyph_id(ch)),
                advance_vertical: font.as_scaled(state.char_scale).v_advance(font.glyph_id(ch)),
                position_min_x,
                position_min_y,
                position_max_x,
                position_max_y,
            };

            map.insert(ch, character);
        }

        Self { map, font, char_scale: state.char_scale }
    }

    pub fn get(&self, ch: char) -> Option<&Character> {
        self.map.get(&ch)
    }

    pub fn try_insert(&mut self, ch: char) {
        let glyph = self.font.glyph_id(ch).with_scale(self.char_scale);

        let outline = self.font.outline_glyph(glyph.clone());
        let bounds = if let Some(outline) = &outline {
            outline.px_bounds()
        } else {
            self.font.glyph_bounds(&glyph)
        };

        let position_min_x = bounds.min.x;
        let position_min_y = bounds.min.y;
        let position_max_x = bounds.max.x;
        let position_max_y = bounds.max.y;

        let mut pixels = vec![0u8; (bounds.width() as usize) * (bounds.height() as usize)];
        // do this because space doesn't have outline glyph
        if let Some(outline) = outline {
            outline.draw(|x, y, coverage| {
                let ind = (y as usize * bounds.width() as usize) + x as usize;
                pixels[ind] = (coverage * 255.0) as u8;
            });
        }

        let texture = unsafe { Self::register_character_texture(&pixels, bounds.width() as i32, bounds.height() as i32) };

        let character = Character {
            texture_id: texture,
            width: bounds.width(),
            height: bounds.height(),
            bearing_horizontal: self.font.as_scaled(self.char_scale).h_side_bearing(self.font.glyph_id(ch)), 
            bearing_vertical: self.font.as_scaled(self.char_scale).v_side_bearing(self.font.glyph_id(ch)),
            advance_horizontal: self.font.as_scaled(self.char_scale).h_advance(self.font.glyph_id(ch)),
            advance_vertical: self.font.as_scaled(self.char_scale).v_advance(self.font.glyph_id(ch)),
            position_min_x,
            position_min_y,
            position_max_x,
            position_max_y,
        };

        self.map.insert(ch, character);
    }

    unsafe fn register_character_texture(data: &[u8], width: i32, height: i32) -> u32 {
        let mut texture = 0;

        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RED as i32, width, height, 0, gl::RED, gl::UNSIGNED_BYTE, data.as_ptr().cast());

        // set texture options
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        texture
    }
}
