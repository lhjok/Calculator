use std::cmp::max;
use std::error::Error;
use std::fmt::{
    Display, Formatter,
    Result as FmtResult,
};
use rug::ops::Pow;
use rug::float::Constant;
use rug::{ Assign, Float };
use rug::ops::NegAssign;
use phf::phf_map;
use phf::Map;

#[derive(Clone)]
enum Marker {
    Init,
    Number,
    NegSub,
    LParen,
    RParen,
    Comma,
    Char,
    Const,
    Func,
    Fact,
    Sci,
}

#[derive(Clone)]
enum State {
    Initial,
    Operator,
    Operand,
}

#[derive(Clone)]
enum NegPos {
    Init,
    Record,
    Read,
}

#[derive(Clone, Debug)]
pub enum CalcError {
    /// Division by zero or modulo by zero.
    DivideByZero,
    /// An internal engine error that shouldn't normally be reached.
    UnknownError,
    /// A generic container for custom error strings.
    Custom(String),
    /// The character is not recognized as a valid operator or command.
    UnknownOperator,
    /// General syntax or structural error in the expression (e.g., unexpected token or missing operand).
    ExpressionError(usize),
    /// Unmatched or unbalanced parentheses (`(` or `)`) in the expression.
    UnmatchedParentheses(usize),
    /// The input value is outside the function's domain (e.g., `ln(-1)`).
    ParameterError((usize, String)),
    /// The specified function name does not exist.
    FunctionUndefined((usize, usize)),
    /// Failed to parse a string literal into a valid floating-point number.
    InvalidNumber((usize, usize)),
    /// The character is not a recognized operator.
    OperatorUndefined(usize),
    /// Provided string is empty or contains only whitespace.
    EmptyExpression,
    /// Result is too large or too small for the current precision limits.
    BeyondAccuracy,
    /// An unexpected stack state occurred during evaluation.
    StackError,
}

impl Display for CalcError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            CalcError::Custom(error) => write!(f, "{}", error),
            CalcError::UnknownError => write!(f, "Unknown Error"),
            CalcError::BeyondAccuracy => write!(f, "Beyond Accuracy"),
            CalcError::UnknownOperator => write!(f, "Unknown Operator"),
            CalcError::ParameterError((locat, error)) => write!(f, "{} Column {}", error, locat),
            CalcError::ExpressionError(locat) => write!(f, "Expression Error: Column {}", locat),
            CalcError::UnmatchedParentheses(number) => write!(f, "Unmatched Parentheses: {} unmatched", number),
            CalcError::FunctionUndefined((l, i)) => write!(f, "Function Undefined: Columns {}-{}", l, i),
            CalcError::OperatorUndefined(locat) => write!(f, "Operator Undefined: Column {}", locat),
            CalcError::InvalidNumber((l, i)) => write!(f, "Invalid Number: Columns {}-{}", l, i),
            CalcError::EmptyExpression => write!(f, "Empty Expression"),
            CalcError::DivideByZero => write!(f, "Divide By Zero"),
            CalcError::StackError => write!(f, "Stack Error"),
        }
    }
}

// Standard Error trait implementation for CalcError.
impl Error for CalcError {}

/// Type alias for a single-parameter mathematical function.
type MathFn1 = fn(Float, &mut Float, &Context, usize) -> Result<Float, CalcError>;
static MATH1: Map<&'static [u8], MathFn1> = phf_map! {
    b"ai" => |v, _, c, _| v.ai().accuracy(c.max.as_ref()),
    b"li2" => |v, _, c, _| v.li2().accuracy(c.max.as_ref()),
    b"erf" => |v, _, c, _| v.erf().accuracy(c.max.as_ref()),
    b"erfc" => |v, _, c, _| v.erfc().accuracy(c.max.as_ref()),
    b"abs" => |v, _, c, _| v.abs().accuracy(c.max.as_ref()),
    b"ln" => |v, _, c, n| if v <= 0.0 {
        let err = "Ln Requires Param > 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.ln().accuracy(c.max.as_ref()) },
    b"exp" => |v, _, c, _| v.exp().accuracy(c.max.as_ref()),
    b"exp2" => |v, _, c, _| v.exp2().accuracy(c.max.as_ref()),
    b"exp10" => |v, _, c, _| v.exp10().accuracy(c.max.as_ref()),
    b"trunc" => |v, _, c, _| v.trunc().accuracy(c.max.as_ref()),
    b"zeta" => |v, _, c, n| if v == 1.0 {
        let err = "Zeta Requires Param ≠ 1".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.zeta().accuracy(c.max.as_ref()) },
    b"gamma" => |v, f, c, n| if v <= 0.0 && is_integer(&v, f) {
        let err = "Gamma Requires Param > 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.gamma().accuracy(c.max.as_ref()) },
    b"digamma" => |v, f, c, n| if v <= 0.0 && is_integer(&v, f) {
        let err = "DiGamma Requires Param > 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.digamma().accuracy(c.max.as_ref()) },
    b"eint" => |v, _, c, n| if v == 0.0 {
        let err = "Eint Requires Param ≠ 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.eint().accuracy(c.max.as_ref()) },
    b"log2" => |v, _, c, n| if v <= 0.0 {
        let err = "Log2 Requires Param > 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.log2().accuracy(c.max.as_ref()) },
    b"log10" => |v, _, c, n| if v <= 0.0 {
        let err = "Log10 Requires Param > 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.log10().accuracy(c.max.as_ref()) },
    b"cos" => |v, _, c, _| v.cos().accuracy(c.max.as_ref()),
    b"sin" => |v, _, c, _| v.sin().accuracy(c.max.as_ref()),
    b"tan" => |v, _, c, _| v.tan().accuracy(c.max.as_ref()),
    b"sec" => |v, _, c, _| v.sec().accuracy(c.max.as_ref()),
    b"csc" => |v, _, c, n| if v == 0.0 {
        let err = "Csc Requires Param ≠ 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.csc().accuracy(c.max.as_ref()) },
    b"cot" => |v, _, c, n| if v == 0.0 {
        let err = "Cot Requires Param ≠ 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.cot().accuracy(c.max.as_ref()) },
    b"cosh" => |v, _, c, _| v.cosh().accuracy(c.max.as_ref()),
    b"sinh" => |v, _, c, _| v.sinh().accuracy(c.max.as_ref()),
    b"tanh" => |v, _, c, _| v.tanh().accuracy(c.max.as_ref()),
    b"ceil" => |v, _, c, _| v.ceil().accuracy(c.max.as_ref()),
    b"signum" => |v, _, c, _| v.signum().accuracy(c.max.as_ref()),
    b"floor" => |v, _, c, _| v.floor().accuracy(c.max.as_ref()),
    b"fract" => |v, _, c, _| v.fract().accuracy(c.max.as_ref()),
    b"ln_gamma" => |v, _, c, n| if v <= 0.0 {
        let err = "LnGamma Requires Param > 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.ln_gamma().accuracy(c.max.as_ref()) },
    b"j0" => |v, _, c, _| v.j0().accuracy(c.max.as_ref()),
    b"j1" => |v, _, c, _| v.j1().accuracy(c.max.as_ref()),
    b"y0" => |v, _, c, n| if v <= 0.0 {
        let err = "Y0 Requires Param > 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.y0().accuracy(c.max.as_ref()) },
    b"y1" => |v, _, c, n| if v <= 0.0 {
        let err = "Y1 Requires Param > 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.y1().accuracy(c.max.as_ref()) },
    b"square" => |v, _, c, _| v.square().accuracy(c.max.as_ref()),
    b"recip" => |v, _, c, n| if v == 0.0 {
        let err = "Recip Requires Param ≠ 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.recip().accuracy(c.max.as_ref()) },
    b"csch" => |v, _, c, n| if v == 0.0 {
        let err = "Csch Requires Param ≠ 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.csch().accuracy(c.max.as_ref()) },
    b"sech" => |v, _, c, _| v.sech().accuracy(c.max.as_ref()),
    b"coth" => |v, _, c, n| if v == 0.0 {
        let err = "Coth Requires Param ≠ 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.coth().accuracy(c.max.as_ref()) },
    b"acos" => |v, _, c, n| if v < -1.0 || v > 1.0 {
        let err = "Acos Requires Param ≥ -1 and ≤ 1".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.acos().accuracy(c.max.as_ref()) },
    b"asin" => |v, _, c, n| if v < -1.0 || v > 1.0 {
        let err = "Asin Requires Param ≥ -1 and ≤ 1".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.asin().accuracy(c.max.as_ref()) },
    b"atan" => |v, _, c, _| v.atan().accuracy(c.max.as_ref()),
    b"acosh" => |v, _, c, n| if v < 1.0 {
        let err = "Acosh Requires Param ≥ 1".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.acosh().accuracy(c.max.as_ref()) },
    b"asinh" => |v, _, c, _| v.asinh().accuracy(c.max.as_ref()),
    b"atanh" => |v, _, c, n| if v <= -1.0 || v >= 1.0 {
        let err = "Atanh Requires Param > -1 and < 1".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.atanh().accuracy(c.max.as_ref()) },
    b"cbrt" => |v, _, c, _| v.cbrt().accuracy(c.max.as_ref()),
    b"sqrt" => |v, _, c, n| if v < 0.0 {
        let err = "Sqrt Requires Param ≥ 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v.sqrt().accuracy(c.max.as_ref()) },
    b"fac" => |v, f, c, n| if v < 0.0 || !is_integer(&v, f) {
        let err = "Factorial Requires Param Non-negative Integer".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else {
        let to_u32 = v.to_u32_saturating().ok_or(CalcError::UnknownError)?;
        let max_allowed = std::cmp::min(10000, 100+((c.prec/64)*100));
        if to_u32 < max_allowed {
            let fac = Float::factorial(to_u32);
            Float::with_val(c.prec, fac).accuracy(c.max.as_ref())
        } else {
            let err = format!("Factorial Requires Expected Param < {}", max_allowed);
            Err(CalcError::ParameterError((n, err)))
        }
    },
};

/// Type alias for a two-parameter mathematical function.
type MathFn2 = fn(Float, &Float, &mut Float, &Context, usize) -> Result<Float, CalcError>;
static MATH2: Map<&'static [u8], MathFn2> = phf_map! {
    b"sinu" => |v1, v2, f, c, n| if *v2 < 0.0 || !is_integer(&v2, f) {
        let err = "SinU Requires Param2 Non-negative Integer".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else {
        let to_u32 = v2.to_u32_saturating().ok_or(CalcError::UnknownError)?;
        v1.sin_u(to_u32).accuracy(c.max.as_ref())
    },
    b"tanu" => |v1, v2, f, c, n| if *v2 < 0.0 || !is_integer(&v2, f) {
        let err = "TanU Requires Param2 Non-negative Integer".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else {
        let to_u32 = v2.to_u32_saturating().ok_or(CalcError::UnknownError)?;
        v1.tan_u(to_u32).accuracy(c.max.as_ref())
    },
    b"gamma_inc" => |v1, v2, _, c, n| if v1 <= 0.0 || *v2 < 0.0 {
        let err = "GammaInc Requires Param1 > 0 and Param2 ≥ 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v1.gamma_inc(v2).accuracy(c.max.as_ref()) },
    b"remainder" | b"fmod" | b"rem" => |v1, v2, _, c, n| if *v2 == 0.0 {
        let err = "Remainder Requires Param2 ≠ 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v1.remainder(v2).accuracy(c.max.as_ref()) },
    b"agm" => |v1, v2, _, c, n| if v1 < 0.0 || *v2 < 0.0 {
        let err = "Agm Requires Param(1 and 2) ≥ 0".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else { v1.agm(v2).accuracy(c.max.as_ref()) },
    b"max" => |v1, v2, _, c, _| v1.max(v2).accuracy(c.max.as_ref()),
    b"min" => |v1, v2, _, c, _| v1.min(v2).accuracy(c.max.as_ref()),
    b"atan2" => |v1, v2, _, c, _| v1.atan2(v2).accuracy(c.max.as_ref()),
    b"positive_diff" => |v1, v2, _, c, _| v1.positive_diff(v2).accuracy(c.max.as_ref()),
    b"copysign" => |v1, v2, _, c, _| v1.copysign(v2).accuracy(c.max.as_ref()),
    b"hypot" => |v1, v2, _, c, _| v1.hypot(v2).accuracy(c.max.as_ref()),
    b"pow" => |v1, v2, _, c, _| v1.pow(v2).accuracy(c.max.as_ref()),
    b"root" => |v1, v2, f, c, n| if *v2 <= 0.0 || !is_integer(&v2, f) {
        let err = "Root Requires Param2 > 0 and Non-negative Integer".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else {
        let to_u32 = v2.to_u32_saturating().ok_or(CalcError::UnknownError)?;
        v1.root(to_u32).accuracy(c.max.as_ref())
    },
    b"rooti" => |v1, v2, f, c, n| if *v2 == 0.0 || !is_integer(&v2, f) {
        let err = "RootI Requires Param2 ≠ 0 and Exact Integer".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else {
        let to_i32 = v2.to_i32_saturating().ok_or(CalcError::UnknownError)?;
        v1.root_i(to_i32).accuracy(c.max.as_ref())
    },
    b"jn" => |v1, v2, f, c, n| if !is_integer(&v2, f) {
        let err = "Jn Requires Param2 Exact Integer".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else {
        let to_i32 = v2.to_i32_saturating().ok_or(CalcError::UnknownError)?;
        v1.jn(to_i32).accuracy(c.max.as_ref())
    },
    b"yn" => |v1, v2, f, c, n| if v1 <= 0.0 || !is_integer(&v2, f) {
        let err = "Yn Requires Param1 > 0 and Param2 Exact Integer".to_owned();
        Err(CalcError::ParameterError((n, err)))
    } else {
        let to_i32 = v2.to_i32_saturating().ok_or(CalcError::UnknownError)?;
        v1.yn(to_i32).accuracy(c.max.as_ref())
    },
};

#[derive(Clone)]
struct Context {
    pub max: Option<Float>,
    pub prec: u32,
}

#[derive(Clone)]
pub struct Calculator {
    marker: Marker,
    context: Context,
    operator: Vec<u8>,
    negative: Vec<NegPos>,
    function: Vec<(Option<MathFn1>, Option<MathFn2>)>,
    parameter: Vec<Option<usize>>,
    numbers: Vec<Float>,
    negstate: NegPos,
    bracket: usize,
    filter: bool,
    state: State,
}

fn is_integer(v: &Float, scratch: &mut Float) -> bool {
    scratch.assign(v.fract_ref());
    scratch.is_zero()
}

fn max_value(prec: u32) -> Float {
    let k = (prec as f64 * 0.0025).floor() as u32;
    let d = (prec as f64 * 0.3010299956639812).floor() as u32;
    let max_val = Float::i_pow_u(10, d-k);
    Float::with_val(prec, max_val)
}

fn extract(expr: &[u8], c: &Context, n: usize, i: usize) -> Result<Float, CalcError> {
    match Float::parse(&expr[n..i]) {
        Ok(valid) => Float::with_val(c.prec, valid).accuracy(c.max.as_ref()),
        Err(_) => Err(CalcError::InvalidNumber((n, i))),
    }
}

/// Extension trait for `u8` to handle operator logic.
trait ByteExt {
    /// Returns the precedence of the operator. Higher values mean higher priority.
    fn priority(&self) -> Result<u8, CalcError>;
    /// Performs the mathematical operation using the top two numbers from the calculator stack.
    fn computing(&self, n: &mut Calculator) -> Result<Float, CalcError>;
}

/// Extension trait for `rug::Float` to provide formatting and accuracy checks.
trait FloatExt {
    /// Validates if the result is a finite number and within the allowed magnitude.
    fn accuracy(self, max_val: Option<&Float>) -> Result<Float, CalcError>;
    /// Converts the arbitrary-precision float to a rounded string representation.
    /// Supports custom precision up to 10000 decimal places.
    fn to_round(&self, digits: Option<usize>) -> Result<String, CalcError>;
}

/// Extension trait for `String` to facilitate raw data parsing and formatting.
trait StringExt {
    /// Parses a raw Rug Float string into its components (sign, digits, exponent).
    fn parse_rug_raw(&self) -> (bool, Vec<u8>, i32);
    /// Formats a float string into a fixed-point representation without trailing zeros.
    fn to_fixed_clean(&self) -> Result<String, CalcError>;
    /// Rounds the floating-point string to a specific number of decimal places.
    fn to_fixed_round(&self, prec: i32) -> Result<String, CalcError>;
}

impl ByteExt for u8 {
    fn priority(&self) -> Result<u8, CalcError> {
        match self {
            b'+' | b'-' => Ok(1),
            b'*' | b'/' | b'%' => Ok(2),
            b'^' => Ok(3),
            _ => Err(CalcError::UnknownOperator),
        }
    }

    fn computing(&self, num: &mut Calculator) -> Result<Float, CalcError> {
        let ctx = &num.context;
        let c1 = num.numbers.pop().ok_or(CalcError::StackError)?;
        let c2 = num.numbers.pop().ok_or(CalcError::StackError)?;
        match self {
            b'+' => Float::with_val(ctx.prec, &c2 + &c1).accuracy(ctx.max.as_ref()),
            b'-' => Float::with_val(ctx.prec, &c2 - &c1).accuracy(ctx.max.as_ref()),
            b'*' => Float::with_val(ctx.prec, &c2 * &c1).accuracy(ctx.max.as_ref()),
            b'/' if &c1 != &0.0 => Float::with_val(ctx.prec, &c2 / &c1).accuracy(ctx.max.as_ref()),
            b'%' if &c1 != &0.0 => c2.remainder(&c1).accuracy(ctx.max.as_ref()),
            b'^' => Float::with_val(ctx.prec, &c2.pow(&c1)).accuracy(ctx.max.as_ref()),
            _ => Err(CalcError::DivideByZero),
        }
    }
}

impl FloatExt for Float {
    fn accuracy(self, max_val: Option<&Float>) -> Result<Float, CalcError> {
        if self.is_nan() || self.is_infinite() {
            return Err(CalcError::BeyondAccuracy);
        } else if let Some(max) = max_val {
            if self > *max || self < *max.as_neg() {
                return Err(CalcError::BeyondAccuracy);
            }
        } Ok(self)
    }

    fn to_round(&self, digits: Option<usize>) -> Result<String, CalcError> {
        if let Some(precision) = digits {
            if precision < 1 || precision > 10000 {
                let err = String::from("Set Precision: ≥ 1 and < 10000");
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
        let digits_len = digits.len();
        let exp_abs = (exp.unsigned_abs()+2) as usize;
        let capacity = digits_len + exp_abs;
        if exp.abs() > 10000 || capacity > 65536 {
            return Err(CalcError::BeyondAccuracy);
        }
        let mut cursor = 0;
        let mut buf = vec![b'0'; capacity];
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
            final_len = dec_len.min(10000);
            while final_len > 0 {
                match buf[final_len-1] {
                    b'0' => final_len -= 1,
                    b'.' => { final_len -= 1; break; }
                    _ => break,
                }
            }
        }
        buf.truncate(final_len);
        let result = String::from_utf8(buf)
            .map_err(|_| CalcError::UnknownError)?;
        Ok(result)
    }

    fn to_fixed_round(&self, prec: i32) -> Result<String, CalcError> {
        let (negative, digits, exp) = self.parse_rug_raw();
        let round_pos = exp + prec;
        let round_idx = round_pos as usize;
        let carry = if round_pos >= 0 && round_idx < digits.len() {
            digits[round_idx] >= b'5' } else { false };
        let max_bound = round_pos - 1;
        let min_bound = std::cmp::min(0, exp-1);
        let capacity = ((max_bound-min_bound).max(0) as usize) + 9;
        if exp.abs() > 10000 || capacity > 65536 {
            return Err(CalcError::BeyondAccuracy);
        }
        let mut buf = vec![b'0'; capacity];
        let mut cursor = capacity - 1;
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
        let final_buf = &buf[cursor+1..capacity];
        let mut final_len = final_buf.len();
        for index in final_buf.iter().rev() {
            match index {
                b'0' => final_len -= 1,
                b'.' => { final_len -= 1; break; }
                _ => break,
            }
        }
        let final_vec = final_buf[..final_len].to_vec();
        let result = String::from_utf8(final_vec)
            .map_err(|_| CalcError::UnknownError)?;
        Ok(result)
    }
}

impl Context {
    fn new(prec: u32) -> Self {
        Self { prec, max: None }
    }
    fn set_max_some(&mut self) {
        self.max = Some(max_value(self.prec));
    }
    fn set_max_none(&mut self) {
        self.max = None;
    }
}

impl Calculator {
    /// Initializes a new Calculator instance with pre-allocated capacities.
    pub fn new(prec: u32) -> Self {
        let prec = max(64, prec);
        Self {
            marker: Marker::Init,
            state: State::Initial,
            context: Context::new(prec),
            numbers: Vec::with_capacity(96),
            operator: Vec::with_capacity(64),
            negative: vec![NegPos::Init; 64],
            function: vec![(None, None); 32],
            parameter: vec![None; 32],
            negstate: NegPos::Init,
            filter: false,
            bracket: 0,
        }
    }

    /// Sets whether to filter or skip whitespace and control characters during parsing.
    /// By default, whitespace is not filtered.
    pub fn set_filter(&mut self, filter: bool) -> &mut Self {
        self.filter = filter; self
    }

    /// Pre-allocates capacity for the numbers stack to avoid frequent reallocations.
    pub fn set_numbers_size(&mut self, size: usize) -> &mut Self {
        self.numbers = Vec::with_capacity(size); self
    }

    /// Pre-allocates capacity for the operator stack to avoid frequent reallocations.
    pub fn set_operator_size(&mut self, size: usize) -> &mut Self {
        self.operator = Vec::with_capacity(size);
        self.negative = vec![NegPos::Init; size]; self
    }

    /// Resizes and initializes the function and parameter storage vectors to the specified size.
    pub fn set_function_size(&mut self, size: usize) -> &mut Self {
        self.function = vec![(None, None); size];
        self.parameter = vec![None; size]; self
    }

    fn reset(&mut self) {
        self.numbers.clear();
        self.state = State::Initial;
        self.function.fill((None, None));
        self.negative.fill(NegPos::Init);
        self.negstate = NegPos::Init;
        self.marker = Marker::Init;
        self.parameter.fill(None);
        self.operator.clear();
        self.bracket = 0;
    }

    fn implicit_mul(&mut self) {
        self.state = State::Operator;
        self.operator.push(b'*');
    }

    fn apply_unary_minus(&mut self, value: &mut Float) {
        let len = self.operator.len();
        if let Some(negpos) = self.negative.get(len) {
            if matches!(negpos, NegPos::Record) {
                self.negative[len] = NegPos::Init;
                value.neg_assign();
            }
        }
    }

    fn apply_power_negation(&mut self, op: u8, value: &mut Float) {
        let len = self.operator.len();
        if let Some(negpos) = self.negative.get(len) {
            if op == b'^' && matches!(negpos, NegPos::Read) {
                self.negative[len] = NegPos::Init;
                self.negstate = NegPos::Init;
                value.neg_assign();
            }
        }
    }

    fn eval_power_first(&mut self) -> Result<(), CalcError> {
        while let Some(operator) = self.operator.last() {
            if operator.priority()? == 3 {
                if let Some(op) = self.operator.pop() {
                    let mut value = op.computing(self)?;
                    self.apply_power_negation(op, &mut value);
                    self.numbers.push(value);
                }
            } else {
                return Ok(());
            }
        } Ok(())
    }

    fn is_resize_neg(&mut self, target_idx: usize) {
        if target_idx >= self.negative.len() {
            let len = self.negative.len();
            self.negative.resize(target_idx+(len/2), NegPos::Init);
        }
    }

    fn is_resize_func(&mut self, target_idx: usize) {
        if target_idx >= self.function.len() {
            let len = self.function.len();
            self.function.resize(target_idx+(len/2), (None, None));
            self.parameter.resize(target_idx+(len/2), None);
        }
    }

    fn finish(&mut self, expr: &[u8], locat: usize, end_idx: usize) -> Result<Float, CalcError> {
        if matches!(self.marker, Marker::Init) {
            return Err(CalcError::EmptyExpression);
        } else if matches!(self.marker, Marker::NegSub | Marker::Char | Marker::Func) {
            return Err(CalcError::ExpressionError(end_idx));
        } else if self.bracket > 0 {
            return Err(CalcError::UnmatchedParentheses(self.bracket));
        }
        if matches!(self.state, State::Operator | State::Initial) {
            self.numbers.push(extract(expr, &self.context, locat, end_idx)?);
            self.state = State::Operand;
        }
        while let Some(op) = self.operator.pop() {
            let mut value = op.computing(self)?;
            self.apply_power_negation(op, &mut value);
            self.numbers.push(value);
        }
        let result = self.numbers.pop()
            .ok_or(CalcError::StackError)?;
        self.reset(); Ok(result)
    }

    fn parse(&mut self, bytes: &[u8]) -> Result<Float, CalcError> {
        let mut locat: usize = 0;
        let mut scratch = Float::new(self.context.prec);
        for (index, &valid) in bytes.iter().enumerate() {
            match valid {
                b'0'..=b'9' | b'.' => {
                    if matches!(self.marker, Marker::Func) {
                        continue;
                    } else if matches!(self.marker, Marker::Fact | Marker::Const) {
                        return Err(CalcError::ExpressionError(index+1));
                    } else if matches!(self.marker, Marker::RParen) {
                        self.eval_power_first()?;
                        self.implicit_mul();
                    }
                    self.marker = Marker::Number;
                    continue;
                },
                ch @ b'a'..=b'z' | ch @ b'E' | ch @ b'_' => {
                    if ch == b'e' || ch == b'E' {
                        if matches!(self.marker, Marker::Number) {
                            self.marker = Marker::Sci;
                            continue;
                        }
                    }
                    if matches!(self.marker, Marker::NegSub) {
                        self.is_resize_neg(self.operator.len());
                        self.negative[self.operator.len()] = NegPos::Record;
                        locat = index;
                    } else if matches!(self.marker, Marker::RParen | Marker::Const | Marker::Fact) {
                        self.eval_power_first()?;
                        self.implicit_mul();
                    } else if matches!(self.marker, Marker::Number | Marker::Sci) {
                        let ctx = &self.context;
                        let index = if matches!(self.marker, Marker::Sci) {
                            index - 1 } else { index };
                        self.numbers.push(extract(bytes, ctx, locat, index)?);
                        self.eval_power_first()?;
                        self.implicit_mul();
                        locat = index;
                    } 
                    self.marker = Marker::Func;
                    continue;
                },
                ch @ b'+' | ch @ b'-' | ch @ b'*' | ch @ b'/' | ch @ b'%' | ch @ b'^' => {
                    if (ch == b'-' || ch == b'+') && matches!(self.marker, Marker::Sci) {
                        self.marker = Marker::Number; continue;
                    } else if matches!(self.marker, Marker::Sci) {
                        return Err(CalcError::InvalidNumber((locat, index)));
                    }
                    if ch == b'-' && matches!(self.marker, Marker::Init | Marker::LParen | Marker::Char | Marker::Comma) {
                        self.marker = Marker::NegSub;
                        self.negstate = NegPos::Record;
                        continue;
                    } else if !matches!(self.marker, Marker::Number | Marker::RParen | Marker::Const | Marker::Fact) {
                        return Err(CalcError::ExpressionError(index+1));
                    }
                    if matches!(self.state, State::Operator | State::Initial) {
                        if ch == b'^' && matches!(self.negstate, NegPos::Record) {
                            self.is_resize_neg(self.operator.len());
                            self.negative[self.operator.len()] = NegPos::Read;
                            self.negstate = NegPos::Read; locat += 1;
                        } else if !matches!(self.negstate, NegPos::Read) {
                            self.negstate = NegPos::Init;
                        }
                        let ctx = &self.context;
                        self.numbers.push(extract(bytes, ctx, locat, index)?);
                        self.state = State::Operand;
                    }
                    while self.operator.len() != 0 && self.operator.last() != Some(&b'(') {
                        let mut curr_operator = ch.priority()?;
                        let operator = self.operator.last().ok_or(CalcError::StackError)?;
                        let prev_operator = operator.priority()?;
                        if prev_operator == 3 && curr_operator == 3 {
                            curr_operator += 1; }
                        if prev_operator >= curr_operator {
                            if let Some(op) = self.operator.pop() {
                                let mut value = op.computing(self)?;
                                self.apply_power_negation(op, &mut value);
                                self.numbers.push(value);
                            } else {
                                return Err(CalcError::StackError);
                            }
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
                b'!' => {
                    let func = MATH1.get(b"fac").unwrap();
                    if matches!(self.marker, Marker::Number) {
                        let value = extract(bytes, &self.context, locat, index)?;
                        self.numbers.push(func(value, &mut scratch, &self.context, index)?);
                    } else if matches!(self.marker, Marker::RParen | Marker::Const) {
                        let value = self.numbers.pop().ok_or(CalcError::StackError)?;
                        self.numbers.push(func(value, &mut scratch, &self.context, index)?);
                    } else {
                        return Err(CalcError::ExpressionError(index+1));
                    }
                    self.state = State::Operand;
                    self.marker = Marker::Fact;
                    locat = index + 1;
                    continue;
                },
                ch @ b'(' => {
                    if matches!(self.marker, Marker::Func) {
                        self.negstate = NegPos::Init;
                        let name = &bytes[locat..index];
                        let target_idx = self.bracket + 1;
                        if let Some(&func_ptr) = MATH1.get(name) {
                            self.is_resize_func(target_idx);
                            self.function[target_idx] = (Some(func_ptr), None);
                            self.parameter[target_idx] = Some(1);
                        } else if let Some(&func_ptr) = MATH2.get(name) {
                            self.is_resize_func(target_idx);
                            self.function[target_idx] = (None, Some(func_ptr));
                            self.parameter[target_idx] = Some(1);
                        } else {
                            return Err(CalcError::FunctionUndefined((locat, index)));
                        }
                    } else if matches!(self.marker, Marker::RParen | Marker::Const | Marker::Fact) {
                        self.eval_power_first()?;
                        self.implicit_mul();
                    } else if matches!(self.marker, Marker::NegSub) {
                        self.is_resize_neg(self.operator.len());
                        self.negative[self.operator.len()] = NegPos::Record;
                        self.negstate = NegPos::Init;
                    } else if matches!(self.marker, Marker::Number | Marker::Sci) {
                        let ctx = &self.context;
                        self.numbers.push(extract(bytes, ctx, locat, index)?);
                        self.eval_power_first()?;
                        self.implicit_mul();
                    }
                    locat = index + 1;
                    self.bracket += 1;
                    self.marker = Marker::LParen;
                    self.operator.push(ch);
                    continue;
                },
                ch @ b')' | ch @ b',' => {
                    if matches!(self.state, State::Operator | State::Initial) {
                        if matches!(self.marker, Marker::Number | Marker::Sci) {
                            let ctx = &self.context;
                            self.numbers.push(extract(bytes, ctx, locat, index)?);
                            self.state = State::Operand;
                        }
                    }
                    if matches!(self.state, State::Operand) {
                        if self.bracket > 0 {
                            while self.operator.last() != Some(&b'(') {
                                if let Some(op) = self.operator.pop() {
                                    let mut value = op.computing(self)?;
                                    self.apply_power_negation(op, &mut value);
                                    self.numbers.push(value);
                                } else {
                                    return Err(CalcError::StackError);
                                }
                            }
                            if ch == b',' {
                                let increment = self.parameter[self.bracket].map(|v| v + 1);
                                self.parameter[self.bracket] = increment;
                                self.state = State::Initial;
                                self.marker = Marker::Comma;
                                locat = index + 1;
                                continue;
                            }
                            self.operator.pop();
                            if let Some(func) = self.function[self.bracket].0.take() {
                                if self.parameter[self.bracket] == Some(1) {
                                    let value = self.numbers.pop().ok_or(CalcError::StackError)?;
                                    let mut func_val = func(value, &mut scratch, &self.context, index)?;
                                    self.apply_unary_minus(&mut func_val);
                                    self.parameter[self.bracket] = None;
                                    self.numbers.push(func_val);
                                } else {
                                    let err = "Expecting a parameter".to_owned();
                                    return Err(CalcError::ParameterError((index+1, err)));
                                }
                            } else if let Some(func) = self.function[self.bracket].1.take() {
                                if self.parameter[self.bracket] == Some(2) {
                                    let value2 = self.numbers.pop().ok_or(CalcError::StackError)?;
                                    let value1 = self.numbers.pop().ok_or(CalcError::StackError)?;
                                    let mut func_val = func(value1, &value2, &mut scratch, &self.context, index)?;
                                    self.apply_unary_minus(&mut func_val);
                                    self.parameter[self.bracket] = None;
                                    self.numbers.push(func_val);
                                } else {
                                    let err = "Expects two parameters".to_owned();
                                    return Err(CalcError::ParameterError((index+1, err)));
                                }
                            }
                            let len = self.operator.len();
                            if matches!(self.negative[len], NegPos::Record) {
                                let value = self.numbers.last_mut()
                                    .ok_or(CalcError::StackError)?;
                                self.negative[len] = NegPos::Init;
                                value.neg_assign();
                            }
                            locat = index + 1;
                            self.marker = Marker::RParen;
                            self.bracket -= 1;
                            continue;
                        }
                    }
                    return Err(CalcError::ExpressionError(index+1));
                },
                b'=' => return self.finish(bytes, locat, index),
                ch @ b'P' | ch @ b'Y' | ch @ b'G' | ch @ b'L' => {
                    let constant = match ch {
                        b'P' => &Constant::Pi,
                        b'Y' => &Constant::Euler,
                        b'G' => &Constant::Catalan,
                        b'L' => &Constant::Log2,
                        _ => return Err(CalcError::UnknownError)
                    };
                    let prec = self.context.prec;
                    let mut value = Float::with_val(prec, constant);
                    if matches!(self.marker, Marker::RParen | Marker::Const | Marker::Fact) {
                        self.eval_power_first()?;
                        self.implicit_mul();
                    }
                    if !matches!(self.marker, Marker::Func) {
                        if matches!(self.marker, Marker::NegSub) {
                            value.neg_assign();
                        } else if matches!(self.marker, Marker::Number | Marker::Sci) {
                            let ctx = &self.context;
                            self.numbers.push(extract(bytes, ctx, locat, index)?);
                            self.eval_power_first()?;
                            self.implicit_mul();
                        }
                        self.numbers.push(value);
                        self.state = State::Operand;
                        self.marker = Marker::Const;
                        locat = index + 1;
                        continue;
                    }
                    return Err(CalcError::ExpressionError(index+1));
                },
                _ => return Err(CalcError::OperatorUndefined(index+1)),
            }
        }
        self.finish(bytes, locat, bytes.len())
    }

    pub fn run<S: AsRef<[u8]>>(&mut self, expr: S) -> Result<Float, CalcError> {
        let vec_expr: Vec<u8>;
        self.context.set_max_none();
        let new_expr = if self.filter {
            vec_expr = expr.as_ref().iter().copied()
                .filter(|&b| !b.is_ascii_whitespace()).collect();
            vec_expr.as_ref() } else { expr.as_ref() };
        self.parse(new_expr).map_err(|error| {
            self.reset(); error
        })
    }

    pub fn run_round<S: AsRef<[u8]>>(
        &mut self, expr: S, digits: Option<usize>
    ) -> Result<String, CalcError> {
        self.context.set_max_some();
        match self.run(expr) {
            Ok(value) => Ok(value.to_round(digits)?),
            Err(err) => Err(err)
        }
    }
}