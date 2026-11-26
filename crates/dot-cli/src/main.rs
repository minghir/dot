use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use dot_lang::{eval_script, eval_script_file, CanvasBuffer, EvalContext};


use ico::{IconDir, IconDirEntry, IconImage};
use image::{ImageFormat, RgbaImage};

struct CliArgs {
    script_path: Option<PathBuf>,
    out_path: Option<PathBuf>,
    stdout_ico: bool,
    width: u32,
    height: u32,
}

fn parse_args() -> Result<CliArgs, String> {
    let mut args = env::args().skip(1);
    let mut script_path = None;
    let mut out_path = None;
    let mut stdout_ico = false;
    let mut width = 32_u32;
    let mut height = 32_u32;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--out" => {
                let val = args.next().ok_or("Lipsește valoarea după --out")?;
                out_path = Some(PathBuf::from(val));
            }
            "--stdout-ico" => {
                stdout_ico = true;
            }
            "--size" => {
                let val = args.next().ok_or("Lipsește valoarea după --size (ex: 32x32)")?;
                let (w_str, h_str) = val
                    .split_once('x')
                    .ok_or("Format invalid --size, folosește ex: 32x32")?;
                width = w_str.parse().map_err(|_| "Lățime invalidă")?;
                height = h_str.parse().map_err(|_| "Înălțime invalidă")?;
            }
            "-h" | "--help" => {
                eprintln!(
                    "Utilizare: dot [script.dot] [--size WxH] [--out fisier.ico|fisier.png] [--stdout-ico]"
                );
                std::process::exit(0);
            }
            other if other.starts_with('-') => {
                return Err(format!("Opțiune necunoscută: {other}"));
            }
            other => {
                if script_path.is_none() {
                    script_path = Some(PathBuf::from(other));
                } else {
                    return Err(format!("Argument neașteptat: {other}"));
                }
            }
        }
    }

    Ok(CliArgs {
        script_path,
        out_path,
        stdout_ico,
        width,
        height,
    })
}

fn write_ico_bytes(width: u32, height: u32, pixels: &[[u8; 4]]) -> Result<Vec<u8>, String> {
    let mut rgba_bytes = Vec::with_capacity(pixels.len() * 4);
    for px in pixels {
        rgba_bytes.extend_from_slice(px);
    }

    let icon_image = IconImage::from_rgba_data(width, height, rgba_bytes);
    let mut icon_dir = IconDir::new(ico::ResourceType::Icon);
    let entry = IconDirEntry::encode(&icon_image)
        .map_err(|e| format!("Eroare la codarea intrării ICO: {e}"))?;
    icon_dir.add_entry(entry);

    let mut buf = Vec::new();
    icon_dir
        .write(&mut buf)
        .map_err(|e| format!("Eroare la scrierea ICO: {e}"))?;
    Ok(buf)
}

fn write_png_bytes(width: u32, height: u32, pixels: &[[u8; 4]]) -> Result<Vec<u8>, String> {
    let mut rgba_bytes = Vec::with_capacity(pixels.len() * 4);
    for px in pixels {
        rgba_bytes.extend_from_slice(px);
    }

    let img = RgbaImage::from_raw(width, height, rgba_bytes)
        .ok_or_else(|| "Buffer RGBA invalid pentru PNG".to_string())?;

    let mut cursor = io::Cursor::new(Vec::new());
    img.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| format!("Eroare la scrierea PNG: {e}"))?;
    Ok(cursor.into_inner())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Eroare: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = parse_args()?;

    let count = (cli.width as usize)
        .checked_mul(cli.height as usize)
        .ok_or_else(|| "Dimensiuni prea mari".to_string())?;
    let mut pixels = vec![[0_u8, 0_u8, 0_u8, 0_u8]; count];

    let canvas = CanvasBuffer::new(cli.width, cli.height, &mut pixels)?;
    let mut ctx = EvalContext {
        canvas,
        output: Vec::new(),
        vars: HashMap::new(),
        funcs: HashMap::new(),
        structs: HashMap::new(),
        call_depth: 0,
        base_dir: env::current_dir().ok(),
        imported_files: HashSet::new(),
    };

    match &cli.script_path {
        Some(path) => eval_script_file(path, &mut ctx)?,
        None => {
            let mut source = String::new();
            io::stdin()
                .read_to_string(&mut source)
                .map_err(|e| format!("Nu pot citi de la stdin: {e}"))?;
            eval_script(&source, &mut ctx)?;
        }
    }

    for line in &ctx.output {
        eprintln!("{line}");
    }

    if let Some(out) = &cli.out_path {
        let is_png = out
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.eq_ignore_ascii_case("png"))
            .unwrap_or(false);

        let bytes = if is_png {
            write_png_bytes(cli.width, cli.height, &pixels)?
        } else {
            write_ico_bytes(cli.width, cli.height, &pixels)?
        };

        fs::write(out, bytes)
            .map_err(|e| format!("Nu pot scrie '{}': {e}", out.display()))?;
    } else if cli.stdout_ico {
        let bytes = write_ico_bytes(cli.width, cli.height, &pixels)?;
        io::stdout()
            .write_all(&bytes)
            .map_err(|e| format!("Eroare la scrierea ICO pe stdout: {e}"))?;
    }

    Ok(())
}