pub mod ast;
pub mod eval;
pub mod func;
pub mod lexer;

pub use ast::{BinaryOp, Expr, Program, Stmt};
pub use eval::{eval_program, Value};
pub use lexer::{tokenize, Token, TokenKind};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StructDef {
    pub fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UserFn {
    pub params: Vec<String>,
    pub body: Vec<ast::Stmt>,
}

pub struct CanvasBuffer<'a> {
    pub width: u32,
    pub height: u32,
    pub pixels: &'a mut [[u8; 4]],
}

impl<'a> CanvasBuffer<'a> {
    pub fn new(width: u32, height: u32, pixels: &'a mut [[u8; 4]]) -> Result<Self, String> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| "Dimensiune buffer prea mare".to_string())?;
        if pixels.len() != expected {
            return Err(format!(
                "Dimensiune buffer invalidă: așteptat {} pixeli, primit {}",
                expected,
                pixels.len()
            ));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        self.pixels.get(idx).copied()
    }

    pub fn put_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        if let Some(p) = self.pixels.get_mut(idx) {
            *p = rgba;
            true
        } else {
            false
        }
    }
}

pub struct EvalContext<'a> {
    pub canvas: CanvasBuffer<'a>,
    pub output: Vec<String>,
    pub vars: HashMap<String, Value>,
    pub funcs: std::collections::HashMap<String, UserFn>,
    pub structs: std::collections::HashMap<String, StructDef>,
    pub call_depth: usize,
}

/// Parsează o culoare hex: #rgb, #rgba, #rrggbb sau #rrggbbaa.
pub fn parse_hex_color(input: &str) -> Result<[u8; 4], String> {
    let s = input.trim();
    let hex = s
        .strip_prefix('#')
        .ok_or_else(|| format!("Culoare hex invalidă (lipsește '#'): '{s}'"))?;

    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Culoare hex invalidă (caractere non-hex): '{s}'"));
    }

    match hex.len() {
        3 => {
            let expand = |nibble: u8| (nibble << 4) | nibble;
            let r = expand(u8::from_str_radix(&hex[0..1], 16).unwrap());
            let g = expand(u8::from_str_radix(&hex[1..2], 16).unwrap());
            let b = expand(u8::from_str_radix(&hex[2..3], 16).unwrap());
            Ok([r, g, b, 255])
        }
        4 => {
            let expand = |nibble: u8| (nibble << 4) | nibble;
            let r = expand(u8::from_str_radix(&hex[0..1], 16).unwrap());
            let g = expand(u8::from_str_radix(&hex[1..2], 16).unwrap());
            let b = expand(u8::from_str_radix(&hex[2..3], 16).unwrap());
            let a = expand(u8::from_str_radix(&hex[3..4], 16).unwrap());
            Ok([r, g, b, a])
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
            Ok([r, g, b, 255])
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap();
            Ok([r, g, b, a])
        }
        _ => Err(format!(
            "Lungime hex invalidă pentru '{s}' (folosește #rgb, #rgba, #rrggbb sau #rrggbbaa)"
        )),
    }
}

pub fn format_hex_rgba([r, g, b, a]: [u8; 4]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
}

pub fn eval_script(source: &str, ctx: &mut EvalContext<'_>) -> Result<(), String> {
    eval_program(source, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_index() {
        let src = r#"
            let pal = [#060412ff, #14213dff, #3a86ffff, #ff2a85ff, #ffbe0bff]
            let n = len(pal)
            for i in 0..n {
                put_pixel(pal[i], i, 0)
            }
        "#;

        let mut pixels = vec![[0_u8; 4]; 8 * 2];
        let mut ctx = EvalContext {
            canvas: CanvasBuffer::new(8, 2, &mut pixels).unwrap(),
            output: Vec::new(),
            vars: HashMap::new(),
        };

        let toks = tokenize(src).unwrap();
        println!("TOKENS: {:?}", toks);
        eval_program(src, &mut ctx).unwrap();
        assert_eq!(ctx.canvas.get_pixel(0, 0), Some([0x06, 0x04, 0x12, 0xff]));
        assert_eq!(ctx.canvas.get_pixel(4, 0), Some([0xff, 0xbe, 0x0b, 0xff]));
    }
}