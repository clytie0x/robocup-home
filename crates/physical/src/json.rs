//! A small JSON reader for the capture files. It rejects trailing junk.

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        let Value::Object(fields) = self else {
            return None;
        };
        fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(value) if value.is_finite() => Some(*value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(text) => Some(text),
            _ => None,
        }
    }
}

pub fn parse(text: &str) -> Result<Value, String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        at: 0,
    };
    parser.skip();
    let value = parser.value()?;
    parser.skip();
    if parser.at != parser.bytes.len() {
        return Err(format!("trailing junk at {}", parser.at));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        while let Some(byte) = self.bytes.get(self.at) {
            if byte.is_ascii_whitespace() {
                self.at += 1;
            } else {
                break;
            }
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.skip();
        let byte = self
            .peek()
            .ok_or_else(|| format!("truncated json at {}", self.at))?;
        match byte {
            b'n' => self.literal(b"null", Value::Null),
            b't' => self.literal(b"true", Value::Bool(true)),
            b'f' => self.literal(b"false", Value::Bool(false)),
            b'"' => Ok(Value::String(self.string()?)),
            b'[' => self.array(),
            b'{' => self.object(),
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err(format!("unexpected byte at {}", self.at)),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn literal(&mut self, token: &[u8], value: Value) -> Result<Value, String> {
        if self.bytes[self.at..].starts_with(token) {
            self.at += token.len();
            Ok(value)
        } else {
            Err(format!("bad literal at {}", self.at))
        }
    }

    fn array(&mut self) -> Result<Value, String> {
        self.at += 1;
        let mut items = Vec::new();
        loop {
            self.skip();
            if self.peek() == Some(b']') {
                self.at += 1;
                return Ok(Value::Array(items));
            }
            if !items.is_empty() {
                if self.peek() != Some(b',') {
                    return Err(format!("expected comma at {}", self.at));
                }
                self.at += 1;
            }
            items.push(self.value()?);
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.at += 1;
        let mut fields = Vec::new();
        loop {
            self.skip();
            if self.peek() == Some(b'}') {
                self.at += 1;
                return Ok(Value::Object(fields));
            }
            if !fields.is_empty() {
                if self.peek() != Some(b',') {
                    return Err(format!("expected comma at {}", self.at));
                }
                self.at += 1;
                self.skip();
            }
            if self.peek() != Some(b'"') {
                return Err(format!("expected key at {}", self.at));
            }
            let key = self.string()?;
            self.skip();
            if self.peek() != Some(b':') {
                return Err(format!("expected colon at {}", self.at));
            }
            self.at += 1;
            fields.push((key, self.value()?));
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.at += 1;
        let mut out = String::new();
        while let Some(byte) = self.peek() {
            self.at += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escape = self.peek().ok_or_else(|| "truncated escape".to_string())?;
                    self.at += 1;
                    match escape {
                        b'"' | b'\\' | b'/' => out.push(escape as char),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(self.hex4()?),
                        _ => return Err(format!("bad escape at {}", self.at)),
                    }
                }
                byte if byte < 0x20 => return Err(format!("raw control at {}", self.at)),
                byte => out.push(byte as char),
            }
        }
        Err("unterminated string".to_string())
    }

    fn hex4(&mut self) -> Result<char, String> {
        if self.at + 4 > self.bytes.len() {
            return Err("truncated unicode escape".to_string());
        }
        let text = std::str::from_utf8(&self.bytes[self.at..self.at + 4])
            .map_err(|_| "bad unicode escape".to_string())?;
        self.at += 4;
        let code = u32::from_str_radix(text, 16).map_err(|_| "bad unicode escape".to_string())?;
        char::from_u32(code).ok_or_else(|| "unsupported unicode escape".to_string())
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.at += 1;
                }
            }
            _ => return Err(format!("bad number at {start}")),
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(format!("bad fraction at {start}"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.at += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(format!("bad exponent at {start}"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.at += 1;
            }
        }
        let token = std::str::from_utf8(&self.bytes[start..self.at])
            .map_err(|_| "bad number".to_string())?;
        let value: f64 = token
            .parse()
            .map_err(|_| format!("bad number at {start}"))?;
        if !value.is_finite() {
            return Err(format!("non-finite number at {start}"));
        }
        Ok(Value::Number(value))
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn objects_arrays_and_trailing_junk() {
        let value = parse(r#"{"req":0.117,"A":[[1.5,0],[2,0.0]]}"#).unwrap();
        assert_eq!(value.get("req").and_then(|item| item.as_f64()), Some(0.117));
        assert!(parse(r#"{"a":1} trailing"#).is_err());
        assert!(parse("NaN").is_err());
        assert!(parse("01").is_err());
    }
}
