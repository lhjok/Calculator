use rug::Float;
use rug::ops::Pow;
use std::cmp::max;
use rug::float::Constant;
use phf::phf_map;
use phf::Map;

#[derive(Clone)]
enum Marker {
    Init,
    Number,
    NegSub,
    LParen,
    RParen,
    Char,
    Const,
    Func,
}

#[derive(Clone)]
enum State {
    Initial,
    Operator,
    Operand,
}

#[derive(Clone, Debug)]
pub enum CalcError {
    UnknownOperator,
    Custom(String),
    DivideByZero,
    BeyondAccuracy,
    UnknownError,
    ParameterError,
    ExpressionError,
    FunctionUndefined,
    OperatorUndefined,
    EmptyExpression,
    InvalidNumber,
}

type MathFn = fn(Float, &Context) -> Result<Float, CalcError>;
static MATH: Map<&'static [u8], MathFn> = phf_map! {
    b"ai" => |v, c| v.ai().accuracy(&c.max),
    b"li" => |v, c| v.li2().accuracy(&c.max),
    b"erf" => |v, c| v.erf().accuracy(&c.max),
    b"erfc" => |v, c| v.erfc().accuracy(&c.max),
    b"abs" => |v, c| v.abs().accuracy(&c.max),
    b"ln" => |v, c| if v <= 0.0 {
        Err(CalcError::ParameterError)
    } else { v.ln().accuracy(&c.max) },
    b"exp" => |v, c| v.exp().accuracy(&c.max),
    b"expt" => |v, c| v.exp2().accuracy(&c.max),
    b"expx" => |v, c| v.exp10().accuracy(&c.max),
    b"trunc" => |v, c| v.trunc().accuracy(&c.max),
    b"zeta" => |v, c| if v == 1.0 {
        Err(CalcError::ParameterError)
    } else { v.zeta().accuracy(&c.max) },
    b"gamma" => |v, c| if v == 0.0 {
        Err(CalcError::ParameterError)
    } else { v.gamma().accuracy(&c.max) },
    b"digamma" => |v, c| if v == 0.0 {
        Err(CalcError::ParameterError)
    } else { v.digamma().accuracy(&c.max) },
    b"eint" => |v, c| if v == 0.0 {
        Err(CalcError::ParameterError)
    } else { v.eint().accuracy(&c.max) },
    b"log" => |v, c| if v <= 0.0 {
        Err(CalcError::ParameterError)
    } else { v.log2().accuracy(&c.max) },
    b"logx" => |v, c| if v <= 0.0 {
        Err(CalcError::ParameterError)
    } else { v.log10().accuracy(&c.max) },
    b"cos" => |v, c| v.cos().accuracy(&c.max),
    b"sin" => |v, c| v.sin().accuracy(&c.max),
    b"tan" => |v, c| v.tan().accuracy(&c.max),
    b"sec" => |v, c| v.sec().accuracy(&c.max),
    b"csc" => |v, c| if v == 0.0 {
        Err(CalcError::ParameterError)
    } else { v.csc().accuracy(&c.max) },
    b"cot" => |v, c| if v == 0.0 {
        Err(CalcError::ParameterError)
    } else { v.cot().accuracy(&c.max) },
    b"cosh" => |v, c| v.cosh().accuracy(&c.max),
    b"sinh" => |v, c| v.sinh().accuracy(&c.max),
    b"tanh" => |v, c| v.tanh().accuracy(&c.max),
    b"ceil" => |v, c| v.ceil().accuracy(&c.max),
    b"floor" => |v, c| v.floor().accuracy(&c.max),
    b"frac" => |v, c| v.fract().accuracy(&c.max),
    b"sgn" => |v, c| v.signum().accuracy(&c.max),
    b"recip" => |v, c| if v == 0.0 {
        Err(CalcError::ParameterError)
    } else { v.recip().accuracy(&c.max) },
    b"csch" => |v, c| if v == 0.0 {
        Err(CalcError::ParameterError)
    } else { v.csch().accuracy(&c.max) },
    b"sech" => |v, c| v.sech().accuracy(&c.max),
    b"coth" => |v, c| if v == 0.0 {
        Err(CalcError::ParameterError)
    } else { v.coth().accuracy(&c.max) },
    b"acos" => |v, c| if v < -1.0 || v > 1.0 {
        Err(CalcError::ParameterError)
    } else { v.acos().accuracy(&c.max) },
    b"asin" => |v, c| if v < -1.0 || v > 1.0 {
        Err(CalcError::ParameterError)
    } else { v.asin().accuracy(&c.max) },
    b"atan" => |v, c| v.atan().accuracy(&c.max),
    b"acosh" => |v, c| if v < 1.0 {
        Err(CalcError::ParameterError)
    } else { v.acosh().accuracy(&c.max) },
    b"asinh" => |v, c| v.asinh().accuracy(&c.max),
    b"atanh" => |v, c| if v <= -1.0 || v >= 1.0 {
        Err(CalcError::ParameterError)
    } else { v.atanh().accuracy(&c.max) },
    b"cbrt" => |v, c| v.cbrt().accuracy(&c.max),
    b"sqrt" => |v, c| if v < 0.0 {
        Err(CalcError::ParameterError)
    } else { v.sqrt().accuracy(&c.max) },
    b"fac" => |v, c| {
        let to_u32 = v.to_u32_saturating().unwrap();
        let fac = Float::factorial(to_u32);
        Float::with_val(c.prec, fac).accuracy(&c.max)
    },
};

#[derive(Clone)]
struct Context {
    pub max: Float,
    pub prec: u32,
}

#[derive(Clone)]
pub struct Calculator {
    marker: Marker,
    context: Context,
    operator: Vec<u8>,
    function: Vec<Option<MathFn>>,
    numbers: Vec<Float>,
    bracket: usize,
    state: State,
}

fn max_value(prec: u32) -> Float {
    let k = (prec as f64 * 0.0025).floor() as u32;
    let d = (prec as f64 * 0.3010299956639812).floor() as u32;
    let max_val = Float::i_pow_u(10, d-k);
    Float::with_val(prec, max_val)
}

fn extract(expr: &[u8], c: &Context, n: usize, i: usize) -> Result<Float, CalcError> {
    match Float::parse(&expr[n..i]) {
        Ok(valid) => Float::with_val(c.prec, valid).accuracy(&c.max),
        Err(_) => Err(CalcError::InvalidNumber)
    }
}

trait ByteExt {
    fn priority(&self) -> Result<u8, CalcError>;
    fn computing(&self, n: &mut Calculator) -> Result<Float, CalcError>;
}

trait FloatExt {
    fn fmod(&self, n: &Float, prec: u32) -> Float;
    fn accuracy(self, n: &Float) -> Result<Float, CalcError>;
    fn to_round(&self, digits: Option<usize>) -> Result<String, CalcError>;
}

trait StringExt {
    fn parse_rug_raw(&self) -> (bool, Vec<u8>, i32);
    fn to_fixed_clean(&self) -> Result<String, CalcError>;
    fn to_fixed_round(&self, prec: i32) -> Result<String, CalcError>;
}

impl ByteExt for u8 {
    fn priority(&self) -> Result<u8, CalcError> {
        match self {
            b'+' | b'-' => Ok(1),
            b'*' | b'/' | b'%' => Ok(2),
            b'^' => Ok(3),
            _ => Err(CalcError::UnknownOperator)
        }
    }

    fn computing(&self, num: &mut Calculator) -> Result<Float, CalcError> {
        let context = &num.context;
        let c1 = num.numbers.pop().ok_or(CalcError::ExpressionError)?;
        let c2 = num.numbers.pop().ok_or(CalcError::ExpressionError)?;
        match self {
            b'+' => Float::with_val(context.prec, &c2 + &c1).accuracy(&context.max),
            b'-' => Float::with_val(context.prec, &c2 - &c1).accuracy(&context.max),
            b'*' => Float::with_val(context.prec, &c2 * &c1).accuracy(&context.max),
            b'/' if &c1 != &0.0 => Float::with_val(context.prec, &c2 / &c1).accuracy(&context.max),
            b'%' if &c1 != &0.0 => c2.fmod(&c1, context.prec).accuracy(&context.max),
            b'^' => Float::with_val(context.prec, &c2.pow(&c1)).accuracy(&context.max),
            _ => Err(CalcError::DivideByZero)
        }
    }
}

impl FloatExt for Float {
    fn fmod(&self, n: &Float, prec: u32) -> Float {
        let mut m = Float::with_val(prec, self / n);
        if self < &0.0 {
            m.ceil_mut()
        } else { m.floor_mut() };
        Float::with_val(prec, self - &m * n)
    }

    fn accuracy(self, max: &Float) -> Result<Float, CalcError> {
        if self.is_nan() || self.is_infinite() {
            Err(CalcError::BeyondAccuracy)
        } else if self > *max || self < *max.as_neg() {
            Err(CalcError::BeyondAccuracy)
        } else { Ok(self) }
    }

    fn to_round(&self, digits: Option<usize>) -> Result<String, CalcError> {
        if let Some(precision) = digits {
            if precision < 1 || precision > 700 {
                let err = String::from("Set Precision Greater Than Equal 1");
                return Err(CalcError::Custom(err));
            }
        }
        let raw = self.to_string_radix(10, None);
        match digits {
            None => raw.to_fixed_clean(),
            Some(digits) => raw.to_fixed_round(digits as i32),
        }
    }
}

impl StringExt for String {
    fn parse_rug_raw(&self) -> (bool, Vec<u8>, i32) {
        let bytes = self.as_bytes();
        let is_neg = bytes.starts_with(&[b'-']);
        let start = if is_neg { 1 } else { 0 };
        let e_pos = bytes.iter().position(|&b| b == b'e');
        let end = e_pos.unwrap_or(bytes.len());
        let mantissa = &bytes[start..end];
        let mut digits = Vec::with_capacity(mantissa.len());
        let mut dot_pos = None;
        for (index, &byte) in mantissa.iter().enumerate() {
            if byte == b'.' {
                dot_pos = Some(index as i32);
            } else { digits.push(byte); }
        }
        let raw_exp: i32 = e_pos.map(|pos|{
            self[pos+1..].parse().unwrap_or(0)
        }).unwrap_or(0);
        let adj_exp = dot_pos.map(|pos| raw_exp+pos)
        .unwrap_or(raw_exp+digits.len() as i32);
        (is_neg, digits, adj_exp)
    }

    fn to_fixed_clean(&self) -> Result<String, CalcError> {
        let (negative, digits, exp) = self.parse_rug_raw();
        let mut cursor = 0;
        let digits_len = digits.len();
        let exp_abs = (exp.unsigned_abs()+2) as usize;
        let mut buf = vec![b'0'; digits_len+exp_abs];
        if negative {
            buf[cursor] = b'-';
            cursor += 1;
        }
        let dot_pos: Option<usize>;
        if exp <= 0 {
            buf[cursor..cursor+2]
            .copy_from_slice(b"0.");
            dot_pos = Some(cursor+1);
            cursor += 2;
            let zeros = exp.abs() as usize;
            cursor += zeros;
            buf[cursor..cursor+digits_len]
            .copy_from_slice(&digits);
            cursor += digits_len;
        } else {
            let dot_idx = exp as usize;
            if dot_idx >= digits_len {
                buf[cursor..cursor+digits_len]
                .copy_from_slice(&digits);
                cursor += digits_len;
                let trailing_zeros = dot_idx-digits_len;
                cursor += trailing_zeros;
                dot_pos = None;
            } else {
                buf[cursor..cursor+dot_idx]
                .copy_from_slice(&digits[..dot_idx]);
                cursor += dot_idx;
                buf[cursor] = b'.';
                dot_pos = Some(cursor);
                cursor += 1;
                let rem = digits_len-dot_idx;
                buf[cursor..cursor+rem]
                .copy_from_slice(&digits[dot_idx..]);
                cursor += rem;
            }
        }
        let mut final_len = cursor;
        if let Some(dot) = dot_pos {
            let dec_len = cursor-dot;
            final_len = if dec_len < 700
            { dec_len } else { 700 };
            while final_len > 0 {
                match buf[final_len-1] {
                    b'0' => final_len -= 1,
                    b'.' => { final_len -= 1; break; }
                    _ => break,
                }
            }
        }
        buf.truncate(final_len);
        Ok(String::from_utf8(buf).unwrap())
    }

    fn to_fixed_round(&self, prec: i32) -> Result<String, CalcError> {
        let (negative, digits, exp) = self.parse_rug_raw();
        let round_idx = (exp+prec) as usize;
        let mut carry = false;
        if round_idx < digits.len() && digits[round_idx] >= b'5' {
            carry = true;
        }
        let mut buf = [b'0'; 4096];
        let mut cursor = 4095;
        let max_bound = (exp+prec)-1;
        let min_bound = std::cmp::min(0, exp-1);
        for index in (min_bound..=max_bound).rev() {
            if index == exp-1 && prec > 0 {
                buf[cursor] = b'.';
                cursor -= 1;
            }
            let index = index as usize;
            let mut digit = if index < digits.len() {
                digits[index]
            } else { b'0' };
            if carry {
                if digit == b'9' {
                    digit = b'0';
                } else {
                    digit += 1;
                    carry = false;
                }
            }
            buf[cursor] = digit;
            cursor -= 1;
        }
        if carry {
            buf[cursor] = b'1';
            cursor -= 1;
        }
        if negative {
            buf[cursor] = b'-';
            cursor -= 1;
        }
        let final_buf = &buf[cursor+1..4096];
        let mut final_len = final_buf.len();
        for index in final_buf.iter().rev() {
            match index {
                b'0' => final_len -= 1,
                b'.' => { final_len -= 1; break; }
                _ => break,
            }
        }
        let result = final_buf[..final_len].to_vec();
        Ok(String::from_utf8(result).unwrap())
    }
}

impl Context {
    fn new(prec: u32) -> Self {
        let max = max_value(prec);
        Self { prec, max }
    }
}

impl CalcError {
    pub fn to_string(&self) -> String {
        match self {
            CalcError::UnknownOperator => String::from("Unknown Operator"),
            CalcError::Custom(error) => String::from(error),
            CalcError::DivideByZero => String::from("Divide By Zero"),
            CalcError::BeyondAccuracy => String::from("Beyond Accuracy"),
            CalcError::UnknownError => String::from("Unknown Error"),
            CalcError::ParameterError => String::from("Parameter Error"),
            CalcError::ExpressionError => String::from("Expression Error"),
            CalcError::FunctionUndefined => String::from("Function Undefined"),
            CalcError::OperatorUndefined => String::from("Operator Undefined"),
            CalcError::EmptyExpression => String::from("Empty Expression"),
            CalcError::InvalidNumber => String::from("Invalid Number"),
        }
    }
}

impl Calculator {
    pub fn new(prec: u32) -> Self {
        let prec = max(64, prec);
        Self {
            state: State::Initial,
            context: Context::new(prec),
            numbers: Vec::with_capacity(32),
            function: vec![None; 32],
            operator: Vec::with_capacity(32),
            marker: Marker::Init,
            bracket: 0,
        }
    }

    pub fn reset(&mut self) {
        self.numbers.clear();
        self.state = State::Initial;
        self.function.fill(None);
        self.marker = Marker::Init;
        self.operator.clear();
        self.bracket = 0;
    }

    fn finish(&mut self, expr: &[u8], locat: usize, end_idx: usize) -> Result<Float, CalcError> {
        if matches!(self.marker, Marker::Init) {
            return Err(CalcError::EmptyExpression);
        } else if self.bracket > 0 || matches!(self.marker, Marker::NegSub | Marker::Char | Marker::Func) {
            return Err(CalcError::ExpressionError);
        }
        if matches!(self.state, State::Operator | State::Initial) {
            self.numbers.push(extract(expr, &self.context, locat, end_idx)?);
            self.state = State::Operand;
        }
        while let Some(op) = self.operator.pop() {
            let value = op.computing(self)?;
            self.numbers.push(value);
        }
        let result = self.numbers.pop().unwrap();
        self.reset(); Ok(result)
    }

    pub fn run<S: AsRef<[u8]>>(&mut self, expr: S) -> Result<Float, CalcError> {
        let bytes = expr.as_ref();
        let mut locat: usize = 0;
        for (index, &valid) in bytes.iter().enumerate() {
            match valid {
                b'0'..=b'9' | b'.' => {
                    if !matches!(self.marker, Marker::RParen | Marker::Const | Marker::Func) {
                        self.marker = Marker::Number;
                        continue;
                    }
                    return Err(CalcError::ExpressionError);
                },
                ch @ b'a'..=b'z' | ch @ b'E' => {
                    if (ch == b'e' || ch == b'E') && matches!(self.marker, Marker::Number) {
                        continue;
                    } else if !matches!(self.marker, Marker::RParen | Marker::Const | Marker::NegSub | Marker::Number) {
                        self.marker = Marker::Func;
                        continue;
                    }
                    return Err(CalcError::ExpressionError);
                },
                ch @ b'+' | ch @ b'-' | ch @ b'*' | ch @ b'/' | ch @ b'%' | ch @ b'^' => {
                    if (ch == b'-' || ch == b'+') && matches!(self.marker, Marker::Number) {
                        let prev_byte = bytes.get(index-1);
                        if matches!(prev_byte, Some(b'e' | b'E')) {
                            continue;
                        }
                    }
                    if ch == b'-' && matches!(self.marker, Marker::Init | Marker::LParen | Marker::Char) {
                        self.marker = Marker::NegSub;
                        continue;
                    } else if !matches!(self.marker, Marker::Number | Marker::RParen | Marker::Const) {
                        return Err(CalcError::ExpressionError);
                    }
                    if matches!(self.state, State::Operator | State::Initial) {
                        self.numbers.push(extract(bytes, &self.context, locat, index)?);
                        self.state = State::Operand;
                    }
                    while self.operator.len() != 0 && self.operator.last() != Some(&b'(') {
                        if self.operator.last().unwrap().priority()? >= ch.priority()? {
                            let value = self.operator.pop().unwrap().computing(self)?;
                            self.numbers.push(value);
                        } else {
                            break;
                        }
                    }
                    self.operator.push(ch);
                    self.state = State::Operator;
                    self.marker = Marker::Char;
                    locat = index + 1;
                    continue;
                },
                ch @ b'(' => {
                    if matches!(self.marker, Marker::Func) {
                        let name = &bytes[locat..index];
                        if let Some(&func_ptr) = MATH.get(name) {
                            self.function[self.bracket+1] = Some(func_ptr);
                        } else {
                            return Err(CalcError::FunctionUndefined);
                        }
                    }
                    if matches!(self.state, State::Operator | State::Initial) {
                        if !matches!(self.marker, Marker::Number | Marker::NegSub) {
                            self.operator.push(ch);
                            locat = index + 1;
                            self.marker = Marker::LParen;
                            self.bracket += 1;
                            continue;
                        }
                    }
                    return Err(CalcError::ExpressionError);
                },
                b')' => {
                    if matches!(self.state, State::Operator | State::Initial) {
                        if matches!(self.marker, Marker::Number) {
                            self.numbers.push(extract(bytes, &self.context, locat, index)?);
                            self.state = State::Operand;
                        }
                    }
                    if matches!(self.state, State::Operand) {
                        if self.bracket > 0 {
                            while self.operator.last() != Some(&b'(') {
                                let value = self.operator.pop().unwrap().computing(self)?;
                                self.numbers.push(value);
                            }
                            if let Some(func) = self.function[self.bracket].take() {
                                let value = self.numbers.pop().unwrap();
                                self.numbers.push(func(value, &self.context)?);
                            }
                            locat = index + 1;
                            self.operator.pop();
                            self.marker = Marker::RParen;
                            self.bracket -= 1;
                            continue;
                        }
                    }
                    return Err(CalcError::ExpressionError);
                },
                b'=' | b'\n' | b'\r' => {
                    return self.finish(bytes, locat, index);
                },
                ch @ b'P' | ch @ b'Y' | ch @ b'C' | ch @ b'L' => {
                    if matches!(self.state, State::Operator | State::Initial) {
                        let constant = match ch {
                            b'P' => &Constant::Pi,
                            b'Y' => &Constant::Euler,
                            b'C' => &Constant::Catalan,
                            b'L' => &Constant::Log2,
                            _ => return Err(CalcError::UnknownError)
                        };
                        if !matches!(self.marker, Marker::Number | Marker::Func) {
                            let value = if matches!(self.marker, Marker::NegSub) {
                                0.0 - Float::with_val(self.context.prec, constant)
                            } else {
                                Float::with_val(self.context.prec, constant)
                            };
                            self.numbers.push(value);
                            self.state = State::Operand;
                            self.marker = Marker::Const;
                            locat = index + 1;
                            continue;
                        }
                    }
                    return Err(CalcError::ExpressionError);
                },
                _ => return Err(CalcError::OperatorUndefined),
            }
        }
        self.finish(bytes, locat, bytes.len())
    }

    pub fn run_round<S: AsRef<[u8]>>(
        &mut self, expr: S, digits: Option<usize>
    ) -> Result<String, CalcError> {
        match self.run(expr) {
            Ok(value) => Ok(value.to_round(digits)?),
            Err(err) => Err(err)
        }
    }
}