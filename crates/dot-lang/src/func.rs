use crate::ast::Expr;
use crate::eval::{eval_expr, Value};
use crate::EvalContext;

pub fn eval_builtin(
    name: &str,
    args: &[Expr],
    ctx: &mut EvalContext<'_>,
    line: usize,
) -> Result<Value, String> {
    match name {
        "push" => return builtin_push(args, ctx, line),
        "pop" => return builtin_pop(args, ctx, line),
        _ => {}
    }

    let mut eval_args = Vec::with_capacity(args.len());
    for a in args {
        eval_args.push(eval_expr(a, ctx, line)?);
    }

    match name {
        "echo" => builtin_echo(&eval_args, ctx),
        "width" => builtin_width(&eval_args, ctx, line),
        "height" => builtin_height(&eval_args, ctx, line),
        "get_pixel" => builtin_get_pixel(&eval_args, ctx, line),
        "put_pixel" => builtin_put_pixel(&eval_args, ctx, line),
        "set_pixel" => builtin_set_pixel(&eval_args, ctx, line),
        "erase_pixel" => builtin_erase_pixel(&eval_args, ctx, line),
        "rect" => builtin_rect(&eval_args, ctx, line),
        "line" => builtin_line(&eval_args, ctx, line),
        "circle" => builtin_circle(&eval_args, ctx, line),
        "mix" => builtin_mix(&eval_args, line),
        "rgba" => builtin_rgba(&eval_args, line),
        "abs" => builtin_abs(&eval_args, line),
        "sin" => builtin_sin(&eval_args, line),
        "cos" => builtin_cos(&eval_args, line),
        "tan" => builtin_tan(&eval_args, line),
        "atan" => builtin_atan(&eval_args, line),
        "atan2" => builtin_atan2(&eval_args, line),
        "sqrt" => builtin_sqrt(&eval_args, line),
        "pi" => builtin_pi(&eval_args, line),
        "min" => builtin_min(&eval_args, line),
        "max" => builtin_max(&eval_args, line),
        "clamp" => builtin_clamp(&eval_args, line),
        "floor" => builtin_floor(&eval_args, line),
        "int" => builtin_int(&eval_args, line),
        "log" => builtin_log(&eval_args, line),
        "len" => builtin_len(&eval_args, line),
        other => Err(format!("Linia {line}: funcție necunoscută '{other}'")),
    }
}

fn builtin_echo(args: &[Value], ctx: &mut EvalContext<'_>) -> Result<Value, String> {
    let msg = args
        .iter()
        .map(Value::format_for_echo)
        .collect::<Vec<_>>()
        .join(" ");
    ctx.output.push(msg);
    Ok(Value::Nil)
}

fn builtin_width(args: &[Value], ctx: &EvalContext<'_>, line: usize) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "Linia {line}: width așteaptă 0 argumente, primit {}",
            args.len()
        ));
    }
    Ok(Value::Int(ctx.canvas.width as i64))
}

fn builtin_height(args: &[Value], ctx: &EvalContext<'_>, line: usize) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "Linia {line}: height așteaptă 0 argumente, primit {}",
            args.len()
        ));
    }
    Ok(Value::Int(ctx.canvas.height as i64))
}

fn builtin_get_pixel(
    args: &[Value],
    ctx: &EvalContext<'_>,
    line: usize,
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "Linia {line}: get_pixel așteaptă 2 argumente (x, y), primit {}",
            args.len()
        ));
    }
    let x = args[0].as_u32_coord("x", line)?;
    let y = args[1].as_u32_coord("y", line)?;
    let rgba = ctx.canvas.get_pixel(x, y).ok_or_else(|| {
        format!(
            "Linia {line}: coordonate în afara limitelor ({x}, {y}) pentru canvas {}x{}",
            ctx.canvas.width, ctx.canvas.height
        )
    })?;
    Ok(Value::Color(rgba))
}

fn builtin_put_pixel(
    args: &[Value],
    ctx: &mut EvalContext<'_>,
    line: usize,
) -> Result<Value, String> {
    if args.len() != 3 {
        return Err(format!(
            "Linia {line}: put_pixel așteaptă 3 argumente (color, x, y), primit {}",
            args.len()
        ));
    }
    let col = args[0].as_color(line)?;
    let x = args[1].as_u32_coord("x", line)?;
    let y = args[2].as_u32_coord("y", line)?;
    if !ctx.canvas.put_pixel(x, y, col) {
        return Err(format!(
            "Linia {line}: coordonate în afara limitelor ({x}, {y}) pentru canvas {}x{}",
            ctx.canvas.width, ctx.canvas.height
        ));
    }
    Ok(Value::Color(col))
}

fn builtin_set_pixel(
    args: &[Value],
    ctx: &mut EvalContext<'_>,
    line: usize,
) -> Result<Value, String> {
    match args.len() {
        2 => {
            let col = args[0].as_color(line)?;
            let points = args[1].as_point_list(line)?;
            let count = points.len() as i64;
            for (x, y) in points {
                if !ctx.canvas.put_pixel(x, y, col) {
                    return Err(format!(
                        "Linia {line}: coordonate în afara limitelor ({x}, {y}) pentru canvas {}x{}",
                        ctx.canvas.width, ctx.canvas.height
                    ));
                }
            }
            Ok(Value::Int(count))
        }
        3 => {
            let col = args[0].as_color(line)?;
            let x = args[1].as_u32_coord("x", line)?;
            let y = args[2].as_u32_coord("y", line)?;
            if !ctx.canvas.put_pixel(x, y, col) {
                return Err(format!(
                    "Linia {line}: coordonate în afara limitelor ({x}, {y}) pentru canvas {}x{}",
                    ctx.canvas.width, ctx.canvas.height
                ));
            }
            Ok(Value::Int(1))
        }
        n => Err(format!(
            "Linia {line}: set_pixel așteaptă (color, [[x, y], ...]) sau (color, x, y), primit {n} argumente"
        )),
    }
}

fn builtin_erase_pixel(
    args: &[Value],
    ctx: &mut EvalContext<'_>,
    line: usize,
) -> Result<Value, String> {
    let transparent = [0_u8, 0_u8, 0_u8, 0_u8];
    match args.len() {
        1 => {
            let points = args[0].as_point_list(line)?;
            let count = points.len() as i64;
            for (x, y) in points {
                if !ctx.canvas.put_pixel(x, y, transparent) {
                    return Err(format!(
                        "Linia {line}: coordonate în afara limitelor ({x}, {y}) pentru canvas {}x{}",
                        ctx.canvas.width, ctx.canvas.height
                    ));
                }
            }
            Ok(Value::Int(count))
        }
        2 => {
            let x = args[0].as_u32_coord("x", line)?;
            let y = args[1].as_u32_coord("y", line)?;
            if !ctx.canvas.put_pixel(x, y, transparent) {
                return Err(format!(
                    "Linia {line}: coordonate în afara limitelor ({x}, {y}) pentru canvas {}x{}",
                    ctx.canvas.width, ctx.canvas.height
                ));
            }
            Ok(Value::Int(1))
        }
        n => Err(format!(
            "Linia {line}: erase_pixel așteaptă ([[x, y], ...]) sau (x, y), primit {n} argumente"
        )),
    }
}

fn builtin_rect(
    args: &[Value],
    ctx: &mut EvalContext<'_>,
    line: usize,
) -> Result<Value, String> {
    if args.len() != 5 {
        return Err(format!(
            "Linia {line}: rect așteaptă 5 argumente (color, x, y, w, h), primit {}",
            args.len()
        ));
    }

    let col = args[0].as_color(line)?;
    let x = args[1].as_u32_coord("x", line)?;
    let y = args[2].as_u32_coord("y", line)?;
    let w = args[3].as_u32_coord("w", line)?;
    let h = args[4].as_u32_coord("h", line)?;

    let end_x = x
        .checked_add(w)
        .ok_or_else(|| format!("Linia {line}: depășire la calculul x + w"))?;
    let end_y = y
        .checked_add(h)
        .ok_or_else(|| format!("Linia {line}: depășire la calculul y + h"))?;

    if end_x > ctx.canvas.width || end_y > ctx.canvas.height {
        return Err(format!(
            "Linia {line}: dreptunghiul ({x}, {y}, {w}, {h}) depășește canvas-ul {}x{}",
            ctx.canvas.width, ctx.canvas.height
        ));
    }

    let mut drawn = 0_i64;
    for py in y..end_y {
        for px in x..end_x {
            if ctx.canvas.put_pixel(px, py, col) {
                drawn += 1;
            }
        }
    }

    Ok(Value::Int(drawn))
}

fn builtin_mix(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 3 {
        return Err(format!(
            "Linia {line}: mix așteaptă 3 argumente (c1, c2, t), primit {}",
            args.len()
        ));
    }
    let c1 = args[0].as_color(line)?;
    let c2 = args[1].as_color(line)?;
    let t = args[2].as_f64(line)?.clamp(0.0, 1.0);

    let lerp = |a: u8, b: u8| -> u8 {
        let fa = a as f64;
        let fb = b as f64;
        (fa + (fb - fa) * t).round().clamp(0.0, 255.0) as u8
    };

    Ok(Value::Color([
        lerp(c1[0], c2[0]),
        lerp(c1[1], c2[1]),
        lerp(c1[2], c2[2]),
        lerp(c1[3], c2[3]),
    ]))
}

fn builtin_rgba(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 4 {
        return Err(format!(
            "Linia {line}: rgba așteaptă 4 argumente (r, g, b, a), primit {}",
            args.len()
        ));
    }

    let to_u8 = |v: &Value, ch: &str| -> Result<u8, String> {
        match v {
            Value::Int(i) => Ok((*i).clamp(0, 255) as u8),
            Value::Float(f) => {
                if *f >= 0.0 && *f <= 1.0 {
                    Ok(((*f) * 255.0).round().clamp(0.0, 255.0) as u8)
                } else {
                    Ok(f.round().clamp(0.0, 255.0) as u8)
                }
            }
            other => Err(format!(
                "Linia {line}: canalul '{ch}' așteaptă număr, primit {:?}",
                other
            )),
        }
    };

    Ok(Value::Color([
        to_u8(&args[0], "r")?,
        to_u8(&args[1], "g")?,
        to_u8(&args[2], "b")?,
        to_u8(&args[3], "a")?,
    ]))
}

fn builtin_abs(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: abs așteaptă 1 argument, primit {}",
            args.len()
        ));
    }

    match &args[0] {
        Value::Int(i) => Ok(Value::Int(i.saturating_abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        other => Err(format!(
            "Linia {line}: abs așteaptă număr, primit {:?}",
            other
        )),
    }
}

fn builtin_sin(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: sin așteaptă 1 argument (radiani), primit {}",
            args.len()
        ));
    }
    let v = args[0].as_f64(line)?;
    Ok(Value::Float(v.sin()))
}

fn builtin_cos(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: cos așteaptă 1 argument (radiani), primit {}",
            args.len()
        ));
    }
    let v = args[0].as_f64(line)?;
    Ok(Value::Float(v.cos()))
}

fn builtin_tan(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: tan așteaptă 1 argument (radiani), primit {}",
            args.len()
        ));
    }
    let v = args[0].as_f64(line)?;
    Ok(Value::Float(v.tan()))
}

fn builtin_atan(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: atan așteaptă 1 argument, primit {}",
            args.len()
        ));
    }
    let v = args[0].as_f64(line)?;
    Ok(Value::Float(v.atan()))
}

fn builtin_atan2(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "Linia {line}: atan2 așteaptă 2 argumente (y, x), primit {}",
            args.len()
        ));
    }
    let y = args[0].as_f64(line)?;
    let x = args[1].as_f64(line)?;
    Ok(Value::Float(y.atan2(x)))
}

fn builtin_sqrt(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: sqrt așteaptă 1 argument, primit {}",
            args.len()
        ));
    }
    let v = args[0].as_f64(line)?;
    if v.is_nan() || v < 0.0 {
        return Err(format!(
            "Linia {line}: sqrt așteaptă număr >= 0, primit {v}"
        ));
    }
    Ok(Value::Float(v.sqrt()))
}

fn builtin_pi(args: &[Value], line: usize) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "Linia {line}: pi așteaptă 0 argumente, primit {}",
            args.len()
        ));
    }
    Ok(Value::Float(std::f64::consts::PI))
}

fn builtin_min(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "Linia {line}: min așteaptă 2 argumente (a, b), primit {}",
            args.len()
        ));
    }
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int((*a).min(*b))),
        _ => {
            let fa = args[0].as_f64(line)?;
            let fb = args[1].as_f64(line)?;
            Ok(Value::Float(fa.min(fb)))
        }
    }
}

fn builtin_max(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "Linia {line}: max așteaptă 2 argumente (a, b), primit {}",
            args.len()
        ));
    }
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int((*a).max(*b))),
        _ => {
            let fa = args[0].as_f64(line)?;
            let fb = args[1].as_f64(line)?;
            Ok(Value::Float(fa.max(fb)))
        }
    }
}

fn builtin_clamp(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 3 {
        return Err(format!(
            "Linia {line}: clamp așteaptă 3 argumente (x, lo, hi), primit {}",
            args.len()
        ));
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::Int(x), Value::Int(lo), Value::Int(hi)) => {
            if lo > hi {
                return Err(format!(
                    "Linia {line}: clamp așteaptă lo <= hi, primit lo={lo}, hi={hi}"
                ));
            }
            Ok(Value::Int((*x).clamp(*lo, *hi)))
        }
        _ => {
            let fx = args[0].as_f64(line)?;
            let flo = args[1].as_f64(line)?;
            let fhi = args[2].as_f64(line)?;
            if flo.is_nan() || fhi.is_nan() || flo > fhi {
                return Err(format!(
                    "Linia {line}: clamp așteaptă lo <= hi, primit lo={flo}, hi={fhi}"
                ));
            }
            Ok(Value::Float(fx.clamp(flo, fhi)))
        }
    }
}

fn builtin_floor(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: floor așteaptă 1 argument, primit {}",
            args.len()
        ));
    }
    let v = args[0].as_f64(line)?;
    Ok(Value::Float(v.floor()))
}

fn builtin_log(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: log așteaptă 1 argument, primit {}",
            args.len()
        ));
    }
    let v = args[0].as_f64(line)?;
    if v.is_nan() || v <= 0.0 {
        return Err(format!(
            "Linia {line}: log așteaptă număr strict pozitiv (> 0), primit {v}"
        ));
    }
    Ok(Value::Float(v.ln()))
}

fn builtin_len(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: len așteaptă 1 argument, primit {}",
            args.len()
        ));
    }
    match &args[0] {
        Value::List(items) => Ok(Value::Int(items.len() as i64)),
        Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
        other => Err(format!(
            "Linia {line}: len așteaptă listă sau șir, primit {:?}",
            other
        )),
    }
}

fn builtin_int(args: &[Value], line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: int așteaptă 1 argument, primit {}",
            args.len()
        ));
    }
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(f) => {
            if !f.is_finite() {
                return Err(format!(
                    "Linia {line}: int nu poate converti un număr infinit sau NaN ({f})"
                ));
            }
            Ok(Value::Int(f.trunc() as i64))
        }
        Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
        other => Err(format!(
            "Linia {line}: int așteaptă număr sau bool, primit {:?}",
            other
        )),
    }
}

fn builtin_push(args: &[Expr], ctx: &mut EvalContext<'_>, line: usize) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "Linia {line}: push așteaptă 2 argumente (listă, valoare), primit {}",
            args.len()
        ));
    }

    let val = eval_expr(&args[1], ctx, line)?;

    if let Expr::Var(var_name) = &args[0] {
        let entry = ctx.vars.get_mut(var_name).ok_or_else(|| {
            format!("Linia {line}: variabilă nedefinită '{var_name}'")
        })?;
        match entry {
            Value::List(items) => {
                items.push(val);
                Ok(Value::List(items.clone()))
            }
            other => Err(format!(
                "Linia {line}: push așteaptă o listă ca prim argument, primit {:?}",
                other
            )),
        }
    } else {
        // Fallback dacă se pasează un literal sau o expresie, ex: push([1, 2], 3)
        let target = eval_expr(&args[0], ctx, line)?;
        match target {
            Value::List(mut items) => {
                items.push(val);
                Ok(Value::List(items))
            }
            other => Err(format!(
                "Linia {line}: push așteaptă o listă ca prim argument, primit {:?}",
                other
            )),
        }
    }
}

fn builtin_pop(args: &[Expr], ctx: &mut EvalContext<'_>, line: usize) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "Linia {line}: pop așteaptă 1 argument (listă), primit {}",
            args.len()
        ));
    }

    if let Expr::Var(var_name) = &args[0] {
        let entry = ctx.vars.get_mut(var_name).ok_or_else(|| {
            format!("Linia {line}: variabilă nedefinită '{var_name}'")
        })?;
        match entry {
            Value::List(items) => items.pop().ok_or_else(|| {
                format!("Linia {line}: pop apelat pe o listă goală ('{var_name}')")
            }),
            other => Err(format!(
                "Linia {line}: pop așteaptă o listă ca argument, primit {:?}",
                other
            )),
        }
    } else {
        let target = eval_expr(&args[0], ctx, line)?;
        match target {
            Value::List(mut items) => items.pop().ok_or_else(|| {
                format!("Linia {line}: pop apelat pe o listă goală")
            }),
            other => Err(format!(
                "Linia {line}: pop așteaptă o listă ca argument, primit {:?}",
                other
            )),
        }
    }
}

fn builtin_line(
    args: &[Value],
    ctx: &mut EvalContext<'_>,
    line: usize,
) -> Result<Value, String> {
    if args.len() != 5 {
        return Err(format!(
            "Linia {line}: line așteaptă 5 argumente (color, x0, y0, x1, y1), primit {}",
            args.len()
        ));
    }

    let col = args[0].as_color(line)?;
    let mut x0 = args[1].as_f64(line)?.round() as i64;
    let mut y0 = args[2].as_f64(line)?.round() as i64;
    let x1 = args[3].as_f64(line)?.round() as i64;
    let y1 = args[4].as_f64(line)?.round() as i64;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut drawn = 0_i64;
    loop {
        if x0 >= 0 && y0 >= 0 && ctx.canvas.put_pixel(x0 as u32, y0 as u32, col) {
            drawn += 1;
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    Ok(Value::Int(drawn))
}

fn builtin_circle(
    args: &[Value],
    ctx: &mut EvalContext<'_>,
    line: usize,
) -> Result<Value, String> {
    if args.len() != 4 && args.len() != 5 {
        return Err(format!(
            "Linia {line}: circle așteaptă 4 sau 5 argumente (color, cx, cy, r [, filled]), primit {}",
            args.len()
        ));
    }

    let col = args[0].as_color(line)?;
    let cx = args[1].as_f64(line)?.round() as i64;
    let cy = args[2].as_f64(line)?.round() as i64;
    let r = args[3].as_f64(line)?.round() as i64;
    let filled = if args.len() == 5 {
        args[4].is_truthy()
    } else {
        false
    };

    if r < 0 {
        return Err(format!(
            "Linia {line}: raza cercului trebuie să fie >= 0, primit {r}"
        ));
    }

    let mut drawn = 0_i64;
    let mut plot = |px: i64, py: i64| {
        if px >= 0 && py >= 0 && ctx.canvas.put_pixel(px as u32, py as u32, col) {
            drawn += 1;
        }
    };

    if r == 0 {
        plot(cx, cy);
        return Ok(Value::Int(drawn));
    }

    let mut x = 0_i64;
    let mut y = r;
    let mut d = 3 - 2 * r;

    while y >= x {
        if filled {
            for px in (cx - x)..=(cx + x) {
                plot(px, cy + y);
                plot(px, cy - y);
            }
            for px in (cx - y)..=(cx + y) {
                plot(px, cy + x);
                plot(px, cy - x);
            }
        } else {
            plot(cx + x, cy + y);
            plot(cx - x, cy + y);
            plot(cx + x, cy - y);
            plot(cx - x, cy - y);
            plot(cx + y, cy + x);
            plot(cx - y, cy + x);
            plot(cx + y, cy - x);
            plot(cx - y, cy - x);
        }

        x += 1;
        if d > 0 {
            y -= 1;
            d = d + 4 * (x - y) + 10;
        } else {
            d = d + 4 * x + 6;
        }
    }

    Ok(Value::Int(drawn))
}