#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Let,
    Func,
    Struct,
    Return,
    If,
    Else,
    While,
    For,
    In,
    DotDot,
    DotDotEq,
    Dot,
    Colon,
    Break,
    Continue,
    Ident(String),
    Int(i64),
    Float(f64),
    HexColor(String),
    Str(String),
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,
    Eq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0usize;
    let mut line = 1usize;

    while i < chars.len() {
        let c = chars[i];

        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }

        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Comentariu pe linie: # ...
        if c == '#' {
            // Dacă imediat urmează caractere hex și nu e spațiu/comentariu clar, verificăm dacă e culoare hex
            let start_hex = i + 1;
            let mut hex_len = 0usize;
            while start_hex + hex_len < chars.len() && chars[start_hex + hex_len].is_ascii_hexdigit() {
                hex_len += 1;
            }
            let next_after_hex = chars.get(start_hex + hex_len).copied();
            let is_valid_hex_token = matches!(hex_len, 3 | 4 | 6 | 8)
                && matches!(
                    next_after_hex,
                    None | Some(',' | ')' | '}' | ']' | ';' | ' ' | '\t' | '\r' | '\n')
                );

            if is_valid_hex_token {
                let raw: String = chars[i..start_hex + hex_len].iter().collect();
                tokens.push(Token {
                    kind: TokenKind::HexColor(raw),
                    line,
                });
                i = start_hex + hex_len;
                continue;
            } else {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
        }

        // Șiruri între ghilimele simple sau duble
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut buf = String::new();
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == '\n' {
                    line += 1;
                }
                if ch == quote {
                    closed = true;
                    i += 1;
                    break;
                }
                if ch == '\\' && i + 1 < chars.len() {
                    i += 1;
                    let esc = chars[i];
                    match esc {
                        'n' => buf.push('\n'),
                        't' => buf.push('\t'),
                        'r' => buf.push('\r'),
                        other => buf.push(other),
                    }
                    i += 1;
                    continue;
                }
                buf.push(ch);
                i += 1;
            }
            if !closed {
                return Err(format!("Linia {line}: șir de caractere neînchis"));
            }
            tokens.push(Token {
                kind: TokenKind::Str(buf),
                line,
            });
            continue;
        }

        // Numere (întregi sau float)
        if c.is_ascii_digit() {
            let start = i;
            let mut has_dot = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch.is_ascii_digit() {
                    i += 1;
                } else if ch == '.' && !has_dot && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                    has_dot = true;
                    i += 1;
                } else {
                    break;
                }
            }
            let text: String = chars[start..i].iter().collect();
            if has_dot {
                let val = text
                    .parse::<f64>()
                    .map_err(|_| format!("Linia {line}: număr real invalid '{text}'"))?;
                tokens.push(Token {
                    kind: TokenKind::Float(val),
                    line,
                });
            } else {
                let val = text
                    .parse::<i64>()
                    .map_err(|_| format!("Linia {line}: număr întreg invalid '{text}'"))?;
                tokens.push(Token {
                    kind: TokenKind::Int(val),
                    line,
                });
            }
            continue;
        }

        // Identificatori și cuvinte cheie
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            let kind = match ident.as_str() {
                "let" => TokenKind::Let,
                "func" => TokenKind::Func,
                "struct" => TokenKind::Struct,
                "return" => TokenKind::Return,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "for" => TokenKind::For,
                "in" => TokenKind::In,
                "break" => TokenKind::Break,
                "continue" => TokenKind::Continue,
                _ => TokenKind::Ident(ident),
            };
            tokens.push(Token { kind, line });
            continue;
        }

        // Operatori compuși și simboluri
        match c {
            '=' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token { kind: TokenKind::EqEq, line });
                    i += 2;
                } else {
                    tokens.push(Token { kind: TokenKind::Eq, line });
                    i += 1;
                }
            }
            '!' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token { kind: TokenKind::BangEq, line });
                    i += 2;
                } else {
                    return Err(format!("Linia {line}: caracter neașteptat '!'"));
                }
            }
            '+' => {
                tokens.push(Token { kind: TokenKind::Plus, line });
                i += 1;
            }
            '-' => {
                tokens.push(Token { kind: TokenKind::Minus, line });
                i += 1;
            }
            '*' => {
                tokens.push(Token { kind: TokenKind::Star, line });
                i += 1;
            }
            '/' => {
                tokens.push(Token { kind: TokenKind::Slash, line });
                i += 1;
            }
            '(' => {
                tokens.push(Token { kind: TokenKind::LParen, line });
                i += 1;
            }
            ')' => {
                tokens.push(Token { kind: TokenKind::RParen, line });
                i += 1;
            }
            '{' => {
                tokens.push(Token { kind: TokenKind::LBrace, line });
                i += 1;
            }
            '}' => {
                tokens.push(Token { kind: TokenKind::RBrace, line });
                i += 1;
            }
            ',' => {
                tokens.push(Token { kind: TokenKind::Comma, line });
                i += 1;
            }
            ';' => {
                tokens.push(Token { kind: TokenKind::Semicolon, line });
                i += 1;
            }
            '[' => {
                tokens.push(Token { kind: TokenKind::LBracket, line });
                i += 1;
            }
            ']' => {
                tokens.push(Token { kind: TokenKind::RBracket, line });
                i += 1;
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token { kind: TokenKind::LtEq, line });
                    i += 2;
                } else {
                    tokens.push(Token { kind: TokenKind::Lt, line });
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token { kind: TokenKind::GtEq, line });
                    i += 2;
                } else {
                    tokens.push(Token { kind: TokenKind::Gt, line });
                    i += 1;
                }
            }
            '&' => {
                if i + 1 < chars.len() && chars[i + 1] == '&' {
                    tokens.push(Token { kind: TokenKind::AndAnd, line });
                    i += 2;
                } else {
                    return Err(format!("Linia {line}: caracter neașteptat '&' (așteptat '&&')"));
                }
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    tokens.push(Token { kind: TokenKind::OrOr, line });
                    i += 2;
                } else {
                    return Err(format!("Linia {line}: caracter neașteptat '|' (așteptat '||')"));
                }
            }
            '%' => {
                tokens.push(Token { kind: TokenKind::Percent, line });
                i += 1;
            }
            ':' => {
                tokens.push(Token { kind: TokenKind::Colon, line });
                i += 1;
            }
            '.' => {
                if i + 1 < chars.len() && chars[i + 1] == '.' {
                    if i + 2 < chars.len() && chars[i + 2] == '=' {
                        tokens.push(Token { kind: TokenKind::DotDotEq, line });
                        i += 3;
                    } else {
                        tokens.push(Token { kind: TokenKind::DotDot, line });
                        i += 2;
                    }
                } else {
                    tokens.push(Token { kind: TokenKind::Dot, line });
                    i += 1;
                }
            }
            other => {
                return Err(format!("Linia {line}: caracter neașteptat '{other}'"));
            }
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        line,
    });
    Ok(tokens)
}