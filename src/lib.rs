use rug::ops::Pow;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use rug::float::Constant;
use rug::Float;

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

#[derive(Clone)]
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
    NoTerminator,
    EmptyExpression,
    InvalidNumber,
}

pub struct Calculator {
    numbers: Vec<Float>,
    function: HashMap<u32, String>,
    operator: Vec<u8>,
    marker: Marker,
    state: State,
}

static MAX: Lazy<Float> = Lazy::new(||{
    let max = Float::parse("1e+764").unwrap();
    Float::with_val(2560, max)
});

static MATH: &[&str] = &["abs","atan","cos","sin",
"tan","csc","sec","cot","coth","ceil","floor","eint",
"trunc","cosh","sinh","tanh","sech","ln","csch","fac",
"acos","frac","sgn","ai","erf","gamma","digamma","exp",
"acosh","asinh","atanh","recip","log","logx","sqrt",
"li","cbrt","asin","erfc","expt","expx","zeta"];

trait Symbol {
    fn priority(&self) -> Result<u8, CalcError>;
    fn computing(&self, n: &mut Calculator) -> Result<Float, CalcError>;
}

trait Bignum {
    fn fmod(&self, n: &Float) -> Float;
    fn accuracy(self) -> Result<Float, CalcError>;
    fn to_round(&self, digits: Option<usize>) -> Result<String, CalcError>;
}

trait Other {
    fn parse_rug_raw(&self) -> (bool, Vec<u8>, i32);
    fn to_fixed_clean(&self) -> Result<String, CalcError>;
    fn to_fixed_round(&self, prec: i32) -> Result<String, CalcError>;
    fn math(&self, v: Float) -> Result<Float, CalcError>;
    fn extract(&self, n: usize, i: usize) -> Result<Float, CalcError>;
}

impl Symbol for u8 {
    fn priority(&self) -> Result<u8, CalcError> {
        match self {
            b'+' | b'-' => Ok(1),
            b'*' | b'/' | b'%' => Ok(2),
            b'^' => Ok(3),
            _ => Err(CalcError::UnknownOperator)
        }
    }

    fn computing(&self, num: &mut Calculator) -> Result<Float, CalcError> {
        let c1 = num.numbers.pop().ok_or(CalcError::ExpressionError)?;
        let c2 = num.numbers.pop().ok_or(CalcError::ExpressionError)?;
        match self {
            b'+' => Float::with_val(2560, &c2 + &c1).accuracy(),
            b'-' => Float::with_val(2560, &c2 - &c1).accuracy(),
            b'*' => Float::with_val(2560, &c2 * &c1).accuracy(),
            b'/' if &c1 != &0.0 => Float::with_val(2560, &c2 / &c1).accuracy(),
            b'%' if &c1 != &0.0 => c2.fmod(&c1).accuracy(),
            b'^' => Float::with_val(2560, &c2.pow(&c1)).accuracy(),
            _ => Err(CalcError::DivideByZero)
        }
    }
}

impl Bignum for Float {
    fn fmod(&self, n: &Float) -> Float {
        let mut m = Float::with_val(2560, self / n);
        if self < &0.0 {
            m.ceil_mut()
        } else { m.floor_mut() };
        Float::with_val(2560, self - &m * n)
    }

    fn accuracy(self) -> Result<Float, CalcError> {
        if self.is_nan() || self.is_infinite() {
            Err(CalcError::BeyondAccuracy)
        } else if self.abs() > *MAX {
            Err(CalcError::BeyondAccuracy)
        } else { Ok(self) }
    }

    fn to_round(&self, digits: Option<usize>) -> Result<String, CalcError> {
        if let Some(precision) = digits {
            if precision < 2 || precision > 700 {
                let err = String::from("Set Precision Greater Than 1");
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

impl Other for String {
    fn parse_rug_raw(&self) -> (bool, Vec<u8>, i32) {
        // 将科学计数法拆分成（符号、数字、指数）
        let bytes = self.as_bytes();
        let is_neg = bytes.starts_with(&[b'-']);
        let start = if is_neg { 1 } else { 0 };
        // 找到科学计数(e)的位置
        let e_pos = bytes.iter().position(|&b| b == b'e');
        let end = e_pos.unwrap_or(bytes.len());
        let mantissa = &bytes[start..end];
        // 创建一个等同大小的数字缓冲区
        let mut digits = Vec::with_capacity(mantissa.len());
        let mut dot_pos = None;
        // 遍历和添加数字并定位点的位置
        for (index, &byte) in mantissa.iter().enumerate() {
            if byte == b'.' {
                dot_pos = Some(index as i32);
            } else { digits.push(byte); }
        }
        // 找出(e)后面的数字
        let raw_exp: i32 = e_pos.map(|pos|{
            self[pos+1..].parse().unwrap_or(0)
        }).unwrap_or(0);
        // 确保小数点位置逻辑统一
        let adj_exp = dot_pos.map(|pos| raw_exp+pos)
        .unwrap_or(raw_exp+digits.len() as i32);
        (is_neg, digits, adj_exp)
    }

    fn to_fixed_clean(&self) -> Result<String, CalcError> {
        // 将科学计数法拆分成（符号、数字、指数）
        let (negative, digits, exp) = self.parse_rug_raw();
        // 创建一个计算好的缓冲区
        let mut cursor = 0;
        let digits_len = digits.len();
        let exp_abs = (exp.unsigned_abs()+2) as usize;
        let mut buf = vec![b'0'; digits_len+exp_abs];
        // 检查当前是否为负数
        if negative {
            buf[cursor] = b'-';  // 首位添加符号
            cursor += 1;
        }
        // 按顺序移动cursor值
        if exp <= 0 {   // 当指数小于等于0时
            buf[cursor..cursor+2]
            .copy_from_slice(b"0.");
            cursor += 2;
            let zeros = exp.abs() as usize;
            cursor += zeros;
            buf[cursor..cursor+digits_len]
            .copy_from_slice(&digits);
            cursor += digits_len;
        } else {
            // 当指数大于0时，执行如下。
            let dot_idx = exp as usize;
            if dot_idx >= digits_len {
                buf[cursor..cursor+digits_len]
                .copy_from_slice(&digits);
                cursor += digits_len;
                let trailing_zeros = dot_idx-digits_len;
                cursor += trailing_zeros;
            } else {
                buf[cursor..cursor+dot_idx]
                .copy_from_slice(&digits[..dot_idx]);
                cursor += dot_idx;
                buf[cursor] = b'.';
                cursor += 1;
                let rem = digits_len-dot_idx;
                buf[cursor..cursor+rem]
                .copy_from_slice(&digits[dot_idx..]);
                cursor += rem;
            }
        }
        // 清楚尾部多余的零
        let mut final_len = cursor;
        if buf[..final_len].contains(&b'.') {
            while final_len > 0 {
                match buf[final_len-1] {
                    b'0' => final_len -= 1,
                    b'.' => { final_len -= 1; break; }
                    _ => break,
                }
            }
        }
        // 最后缓冲区转换成字符串
        buf.truncate(final_len);
        Ok(String::from_utf8(buf).unwrap())
    }

    fn to_fixed_round(&self, prec: i32) -> Result<String, CalcError> {
        // 将科学计数法拆分成（符号、数字、指数）
        let (negative, digits, exp) = self.parse_rug_raw();
        let round_idx = (exp+prec) as usize;
        // 判断四舍五入进位
        let mut carry = false;   // 进位开关默认关闭
        if round_idx < digits.len() && digits[round_idx] >= b'5' {
            carry = true;   // 打开进位开关
        }
        // 创建一个足够大的缓冲区
        let mut buf = [b'0'; 4096];
        let mut cursor = 4095;   // 移位倒计数，逆向填充逻辑。
        let max_bound = (exp+prec)-1;
        let min_bound = std::cmp::min(0, exp-1);
        for index in (min_bound..=max_bound).rev() {
            // 只有在(prec>0)且到达exp位置时插入
            if index == exp-1 && prec > 0 {
                buf[cursor] = b'.';   // 插入小数点
                cursor -= 1;
            }
            // 遍历出(digits)单个数字
            let index = index as usize;
            let mut digit = if index < digits.len() {
                digits[index]
            } else { b'0' };
            // 关闭进位开关结束进位
            if carry {
                if digit == b'9' {
                    digit = b'0';
                } else {
                    digit += 1;
                    carry = false;
                }
            }
            // 把单位数添加进缓冲区
            buf[cursor] = digit;
            cursor -= 1;
        }
        // 循环结束后还有进位则直接补'1'。
        if carry {
            buf[cursor] = b'1';
            cursor -= 1;
        }
        // 如果有符号则添加到缓冲区
        if negative {
            buf[cursor] = b'-';
            cursor -= 1;
        }
        // 缓冲区数据转成字符串
        let result = buf[cursor+1..4096].to_vec();
        Ok(String::from_utf8(result).unwrap())
    }

    fn math(&self, v: Float) -> Result<Float, CalcError> {
        match self.as_str() {
            "ai" => v.ai().accuracy(),
            "li" => v.li2().accuracy(),
            "erf" => v.erf().accuracy(),
            "erfc" => v.erfc().accuracy(),
            "abs" => v.abs().accuracy(),
            "ln" if v > 0.0 => v.ln().accuracy(),
            "exp" => v.exp().accuracy(),
            "expt" => v.exp2().accuracy(),
            "expx" => v.exp10().accuracy(),
            "trunc" => v.trunc().accuracy(),
            "zeta" if v != 1.0 => v.zeta().accuracy(),
            "gamma" if v != 0.0 => v.gamma().accuracy(),
            "digamma" if v != 0.0 => v.digamma().accuracy(),
            "eint" if v != 0.0 => v.eint().accuracy(),
            "log" if v > 0.0 => v.log2().accuracy(),
            "logx" if v > 0.0 => v.log10().accuracy(),
            "cos" => v.cos().accuracy(),
            "sin" => v.sin().accuracy(),
            "tan" => v.tan().accuracy(),
            "sec" => v.sec().accuracy(),
            "csc" if v != 0.0 => v.csc().accuracy(),
            "cot" if v != 0.0 => v.cot().accuracy(),
            "cosh" => v.cosh().accuracy(),
            "sinh" => v.sinh().accuracy(),
            "tanh" => v.tanh().accuracy(),
            "ceil" => v.ceil().accuracy(),
            "floor" => v.floor().accuracy(),
            "frac" => v.fract().accuracy(),
            "sgn" => v.signum().accuracy(),
            "recip" if v != 0.0 => v.recip().accuracy(),
            "csch" if v != 0.0 => v.csch().accuracy(),
            "sech" => v.sech().accuracy(),
            "coth" if v != 0.0 => v.coth().accuracy(),
            "acos" if v >= -1.0 && v <= 1.0 => v.acos().accuracy(),
            "asin" if v >= -1.0 && v <= 1.0 => v.asin().accuracy(),
            "atan" => v.atan().accuracy(),
            "acosh" if v >= 1.0 => v.acosh().accuracy(),
            "asinh" => v.asinh().accuracy(),
            "atanh" if v > -1.0 && v < 1.0 => v.atanh().accuracy(),
            "cbrt" => v.cbrt().accuracy(),
            "sqrt" if v >= 0.0 => v.sqrt().accuracy(),
            "fac" => {
                let to_u32 = v.to_u32_saturating().unwrap();
                let fac = Float::factorial(to_u32);
                Float::with_val(2560, fac).accuracy()
            },
            _ => Err(CalcError::ParameterError)
        }
    }

    fn extract(&self, n: usize, i: usize) -> Result<Float, CalcError> {
        match Float::parse(&self[n..i]) {
            Ok(valid) => Float::with_val(2560, valid).accuracy(),
            Err(_) => Err(CalcError::InvalidNumber)
        }
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
            CalcError::NoTerminator => String::from("No Terminator"),
            CalcError::EmptyExpression => String::from("Empty Expression"),
            CalcError::InvalidNumber => String::from("Invalid Number"),
        }
    }
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            numbers: Vec::new(),
            function: HashMap::new(),
            operator: Vec::new(),
            state: State::Initial,
            marker: Marker::Init,
        }
    }

    pub fn run(&mut self, expr: &String) -> Result<Float, CalcError> {
        let expr = format!("{}=", expr);
        let mut locat: usize = 0;
        let mut bracket: u32 = 0;

        for (index, &valid) in expr.as_bytes().iter().enumerate() {
            match valid {
                b'0'..=b'9' | b'.' => {
                    if !matches!(self.marker, Marker::RParen | Marker::Const | Marker::Func) {
                        self.marker = Marker::Number;
                        continue;
                    }
                    return Err(CalcError::ExpressionError);
                }

                b'a'..=b'z' => {
                    if !matches!(self.marker, Marker::RParen | Marker::Const | Marker::NegSub | Marker::Number) {
                        self.marker = Marker::Func;
                        continue;
                    }
                    return Err(CalcError::ExpressionError);
                }

                ch @ b'+' | ch @ b'-' | ch @ b'*' | ch @ b'/' | ch @ b'%' | ch @ b'^' => {
                    if ch == b'-' && matches!(self.marker, Marker::Init | Marker::LParen | Marker::Char) {
                        self.marker = Marker::NegSub;
                        continue;
                    } else if !matches!(self.marker, Marker::Number | Marker::RParen | Marker::Const) {
                        return Err(CalcError::ExpressionError);
                    }

                    if matches!(self.state, State::Operator | State::Initial) {
                        self.numbers.push(expr.extract(locat, index)?);
                        self.state = State::Operand;
                    }

                    while self.operator.len() != 0 && self.operator.last().unwrap() != &b'(' {
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
                }

                ch @ b'(' => {
                    if matches!(self.marker, Marker::Func) {
                        let valid = expr[locat..index].to_owned();
                        if MATH.binary_search(&valid.as_str()).is_ok() {
                            self.function.insert(bracket+1, valid);
                        } else {
                            return Err(CalcError::ExpressionError);
                        }
                    }

                    if matches!(self.state, State::Operator | State::Initial) {
                        if !matches!(self.marker, Marker::Number | Marker::NegSub) {
                            self.operator.push(ch);
                            locat = index + 1;
                            self.marker = Marker::LParen;
                            bracket = bracket + 1;
                            continue;
                        }
                    }
                    return Err(CalcError::ExpressionError);
                }

                b')' => {
                    if matches!(self.state, State::Operator | State::Initial) {
                        if matches!(self.marker, Marker::Number) {
                            self.numbers.push(expr.extract(locat, index)?);
                            self.state = State::Operand;
                        }
                    }

                    if matches!(self.state, State::Operand) {
                        if bracket > 0 {
                            while self.operator.last().unwrap() != &b'(' {
                                let value = self.operator.pop().unwrap().computing(self)?;
                                self.numbers.push(value);
                            }

                            if let Some(fun) = self.function.remove(&bracket) {
                                let value = fun.math(self.numbers.pop().unwrap())?;
                                self.numbers.push(value);
                            }

                            locat = index + 1;
                            self.operator.pop();
                            self.marker = Marker::RParen;
                            bracket = bracket - 1;
                            continue;
                        }
                    }
                    return Err(CalcError::ExpressionError);
                }

                b'=' | b'\n' | b'\r' => {
                    if matches!(self.marker, Marker::Init) {
                        return Err(CalcError::EmptyExpression);
                    } else if bracket > 0 || matches!(self.marker, Marker::NegSub | Marker::Char | Marker::Func) {
                        return Err(CalcError::ExpressionError);
                    }

                    if matches!(self.state, State::Operator | State::Initial) {
                        self.numbers.push(expr.extract(locat, index)?);
                        self.state = State::Operand;
                    }

                    while self.operator.len() != 0 {
                        let value = self.operator.pop().unwrap().computing(self)?;
                        self.numbers.push(value);
                    }
                    return Ok(self.numbers.pop().unwrap());
                }

                ch @ b'P' | ch @ b'E' | ch @ b'C' | ch @ b'L' => {
                    if matches!(self.state, State::Operator | State::Initial) {
                        let constant = match ch {
                            b'P' => &Constant::Pi,
                            b'E' => &Constant::Euler,
                            b'C' => &Constant::Catalan,
                            b'L' => &Constant::Log2,
                            _ => return Err(CalcError::UnknownError)
                        };

                        if !matches!(self.marker, Marker::Number | Marker::Func) {
                            let value = if matches!(self.marker, Marker::NegSub) {
                                0.0 - Float::with_val(128, constant)
                            } else {
                                Float::with_val(128, constant)
                            };
                            self.numbers.push(value);
                            self.state = State::Operand;
                            self.marker = Marker::Const;
                            locat = index + 1;
                            continue;
                        }
                    }
                    return Err(CalcError::ExpressionError);
                }

                _ => return Err(CalcError::OperatorUndefined)
            }
        }
        Err(CalcError::NoTerminator)
    }

    pub fn run_round(
        &mut self, expr: &String, digits: Option<usize>
    ) -> Result<String, CalcError> {
        match self.run(expr) {
            Ok(value) => Ok(value.to_round(digits)?),
            Err(err) => Err(err)
        }
    }
}