use rug::ops::Pow;
use rug::{float::Constant, Float};
use std::{char::from_digit, cell::RefCell};
use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Clone)]
enum Sign {
    Init,
    Data,
    Char
}

pub struct Calc {
    sign: RefCell<Sign>,
    numbers: RefCell<Vec<Float>>,
    operator: RefCell<Vec<u8>>,
    func: RefCell<HashMap<u32, String>>,
    expression: String,
}

static MAX: Lazy<Float> = Lazy::new(||{
    let max = Float::parse("1e+764").unwrap();
    Float::with_val(2560, max)
});

static MIN: Lazy<Float> = Lazy::new(||{
    let min = Float::parse("-1e+764").unwrap();
    Float::with_val(2560, min)
});

trait Symbol {
    fn priority(&self) -> Result<u8, String>;
    fn computing(&self, n: &Calc) -> Result<Float, String>;
}

trait Bignum {
    fn fmod(&self, n: &Float) -> Float;
    fn accuracy(self) -> Result<Float, String>;
    fn to_round(&self, n: Option<usize>) -> Result<String, String>;
}

trait Other {
    fn to_fixed(&self) -> String;
    fn clean_zero(self) -> String;
    fn math(&self, v: Float) -> Result<Float, String>;
    fn extract(&self, n: usize, i: usize) -> Result<Float, String>;
}

impl Symbol for u8 {
    fn priority(&self) -> Result<u8, String> {
        match self {
            b'+' | b'-' => Ok(1),
            b'*' | b'/' | b'%' => Ok(2),
            b'^' => Ok(3),
            _ => Err("Unknown Operator".to_string())
        }
    }

    fn computing(&self, num: &Calc) -> Result<Float, String> {
        let c1 = num.numbers.borrow_mut().pop().unwrap();
        let c2 = num.numbers.borrow_mut().pop().unwrap();
        match self {
            b'+' => Float::with_val(2560, &c2 + &c1).accuracy(),
            b'-' => Float::with_val(2560, &c2 - &c1).accuracy(),
            b'*' => Float::with_val(2560, &c2 * &c1).accuracy(),
            b'/' if &c1 != &0.0 => Float::with_val(2560, &c2 / &c1).accuracy(),
            b'%' if &c1 != &0.0 => c2.fmod(&c1).accuracy(),
            b'^' => Float::with_val(2560, &c2.pow(&c1)).accuracy(),
            _ => Err("Divide By Zero".to_string())
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

    fn accuracy(self) -> Result<Float, String> {
        if *MAX < self && *MIN > self {
            return Err("Beyond Accuracy".to_string());
        }
        Ok(self)
    }

    fn to_round(&self, digits: Option<usize>) -> Result<String, String> {
        let fix = if self < &1.0 && self > &-1.0 && self != &0.0 {
            self.to_string_radix(10, None).to_fixed()
        } else {
            self.to_string_radix(10, None).clean_zero()
        };

        match digits {
            None => Ok(fix),
            Some(x) => {
                if let None = fix.find('.') {
                    return Ok(fix);
                } else if x < 2 {
                    return Err("Set Accuracy Greater Than 1".to_string());
                }

                let mut n: usize = 0;
                let mut dig: usize = 0;
                let mut point: bool = false;
                let mut res = String::new();

                for (i, v) in fix.as_bytes().iter().enumerate() {
                    match v {
                        b'-' => n = 1,
                        b'.' => {
                            dig = 0;
                            point = true;
                        },
                        _ => dig += 1
                    }
                    if dig < x && i == fix.len()-1 {
                        return Ok(fix);
                    } else if point == true && dig == x && i <= fix.len()-1 {
                        let a = fix[i..i+1].parse::<u32>().unwrap();
                        let b = fix[i-1..i].parse::<u32>().unwrap();
                        res = fix[..i].to_string();
                        if a < 5 {
                            return Ok(res.clean_zero());
                        } else if b < 9 {
                            res.pop();
                            res.push(from_digit(b+1, 10).unwrap());
                            return Ok(res);
                        }
                        break;
                    }
                }

                let rev = res.chars().rev().collect::<String>();
                for (i, v) in rev.as_bytes().iter().enumerate() {
                    if v == &b'.' {
                        continue;
                    }
                    let a = rev[i..i+1].parse::<u32>().unwrap();
                    if a == 9 {
                        res.remove(res.len()-1-i);
                        if i == rev.len()-1-n {
                            res.insert_str(0+n, &(a+1).to_string());
                            return Ok(res.clean_zero());
                        }
                        res.insert(res.len()-i, from_digit(0, 10).unwrap());
                    } else if a < 9 {
                        res.remove(res.len()-1-i);
                        res.insert(res.len()-i, from_digit(a+1, 10).unwrap());
                        return Ok(res.clean_zero());
                    }
                    let point = rev[i+1..i+2].as_bytes();
                    if point == &[b'.'] {
                        continue;
                    }
                    let b = rev[i+1..i+2].parse::<u32>().unwrap();
                    if b < 9 {
                        res.remove(res.len()-2-i);
                        res.insert(res.len()-1-i, from_digit(b+1, 10).unwrap());
                        return Ok(res.clean_zero());
                    }
                }
                Err("Unknown Error".to_string())
            }
        }
    }
}

impl Other for String {
    fn to_fixed(&self) -> String {
        let mut exp: i32 = 0;
        let (mut zero, mut i_or_u) = (0, 0);
        let (mut temp, mut res) = (String::new(), String::new());

        for (i, v) in self.as_bytes().iter().enumerate() {
            if v == &b'e' {
                temp = self[..i].to_string();
                exp = self[i+1..].parse::<i32>().unwrap();
                break;
            }
        }

        if exp == 0 {
            temp = self.clone();
        }

        for (i, v) in temp.as_bytes().iter().enumerate() {
            match v {
                b'.' => {
                    res = temp[..i].to_string();
                    res += &temp[i+1..];
                    zero = 0;
                },
                b'-' => {
                    if exp < 0 {
                        i_or_u = 1
                    } else {
                        exp += 1
                    };
                    zero = 0;
                },
                b'0' => zero += 1,
                _ => zero = 0
            }
        }

        if exp < 0 {
            exp = exp.abs();
            for _ in 0..exp {
                res.insert(i_or_u, '0');
                exp -= 1;
            }
            if i_or_u != 0 {
                exp += 1;
            }
        }

        if exp == 0 && res.len()-zero == 1 {
            return res[..res.len()-zero].to_string();
        } else if exp == 0 && res.len()-zero > 1 {
            res = res[..res.len()-zero].to_string();
            res.insert(1, '.');
            return res;
        }

        let u_exp = exp as usize + 1;
        res.insert(u_exp, '.');
        if u_exp >= res.len()-1-zero {
            return res[..u_exp].to_string();
        }
        res[..res.len()-zero].to_string()
    }

    fn clean_zero(self) -> String {
        let mut find: bool = false;
        let (mut zero, mut dig) = (0, 0);
        for valid in self.as_bytes().iter() {
            match valid {
                b'0' => {
                    dig += 1;
                    zero += 1;
                },
                b'.' => {
                    dig = 0;
                    find = true;
                    zero = 0;
                },
                _ => {
                    dig += 1;
                    zero = 0;
                },
            }
        }
        if find == true {
            if zero == dig {
                return self[..self.len()-dig-1].to_string();
            }
            return self[..self.len()-zero].to_string();
        }
        self
    }

    fn math(&self, v: Float) -> Result<Float, String> {
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
            _ => Err("Parameter Error".to_string())
        }
    }

    fn extract(&self, n: usize, i: usize) -> Result<Float, String> {
        match Float::parse(&self[n..i]) {
            Ok(valid) => Float::with_val(2560, valid).accuracy(),
            Err(_) => Err("Invalid Number".to_string())
        }
    }
}

impl Calc {
    pub fn new(expr: String) -> Self {
        Self {
            sign: RefCell::new(Sign::Init),
            numbers: RefCell::new(Vec::new()),
            operator: RefCell::new(Vec::new()),
            func: RefCell::new(HashMap::new()),
            expression: expr + "=",
        }
    }

    pub fn run(&self) -> Result<Float, String> {
        let num = &self.numbers;
        let ope = &self.operator;
        let expr = &self.expression;
        let math = ["abs","atan","cos","sin","tan","csc","sec","cot","coth","ceil",
        "floor","eint","trunc","cosh","sinh","tanh","sech","ln","csch","acos","fac",
        "frac","sgn","ai","erf","gamma","digamma","acosh","asinh","atanh","recip",
        "log","logx","li","sqrt","cbrt","asin","erfc","exp","expt","expx","zeta"];
        let mut mark: u8 = b'I'; // I = Init, C = Char, N = Number, F = Func, P = Pi
        let mut locat: usize = 0;
        let mut bracket: u32 = 0;

        for (index, &valid) in expr.as_bytes().iter().enumerate() {
            match valid {
                b'0'..=b'9' | b'.' => {
                    if mark != b')' && mark != b'P' && mark != b'F' {
                        mark = b'N';
                        continue;
                    }
                    return Err("Expression Error".to_string());
                }

                b'a'..=b'z' => {
                    if mark != b')' && mark != b'P' && mark != b'-' && mark != b'N' {
                        mark = b'F';
                        continue;
                    }
                    return Err("Expression Error".to_string());
                }

                ch @ b'+' | ch @ b'-' | ch @ b'*' | ch @ b'/' | ch @ b'%' | ch @ b'^' => {
                    if ch == b'-' && ( mark == b'I' || mark == b'(' || mark == b'C' ) {
                        mark = b'-';
                        continue;
                    } else if mark != b'N' && mark != b')' && mark != b'P' {
                        return Err("Expression Error".to_string());
                    }

                    if let Sign::Char | Sign::Init = self.sign.clone().into_inner() {
                        num.borrow_mut().push(expr.extract(locat, index)?);
                        *self.sign.borrow_mut() = Sign::Data;
                    }

                    while ope.borrow().len() != 0 && ope.borrow().last().unwrap() != &b'(' {
                        if ope.borrow().last().unwrap().priority()? >= ch.priority()? {
                            let value = ope.borrow_mut().pop().unwrap().computing(self)?;
                            num.borrow_mut().push(value);
                        } else {
                            break;
                        }
                    }

                    ope.borrow_mut().push(ch);
                    *self.sign.borrow_mut() = Sign::Char;
                    locat = index + 1;
                    mark = b'C';
                    continue;
                }

                ch @ b'(' => {
                    if mark == b'F' {
                        let valid = expr[locat..index].to_string();
                        if math.iter().any(|&value| value == valid) {
                            self.func.borrow_mut().insert(bracket+1, valid);
                        } else {
                            return Err("Function Undefined".to_string());
                        }
                    }

                    if let Sign::Char | Sign::Init = self.sign.clone().into_inner() {
                        if mark != b'N' && mark != b'-' {
                            ope.borrow_mut().push(ch);
                            locat = index + 1;
                            bracket = bracket + 1;
                            mark = b'(';
                            continue;
                        }
                    }
                    return Err("Expression Error".to_string());
                }

                b')' => {
                    if let Sign::Char | Sign::Init = self.sign.clone().into_inner() {
                        if mark == b'N' {
                            num.borrow_mut().push(expr.extract(locat, index)?);
                            *self.sign.borrow_mut() = Sign::Data;
                        }
                    }

                    if let Sign::Data = self.sign.clone().into_inner() {
                        if bracket > 0 {
                            while ope.borrow().last().unwrap() != &b'(' {
                                let value = ope.borrow_mut().pop().unwrap().computing(self)?;
                                num.borrow_mut().push(value);
                            }

                            if let Some(fun) = self.func.borrow_mut().remove(&bracket) {
                                let value = fun.math(num.borrow_mut().pop().unwrap())?;
                                num.borrow_mut().push(value);
                            }

                            ope.borrow_mut().pop();
                            locat = index + 1;
                            bracket = bracket - 1;
                            mark = b')';
                            continue;
                        }
                    }
                    return Err("Expression Error".to_string());
                }

                b'=' | b'\n' | b'\r' => {
                    if mark == b'I' {
                        return Err("Empty Expression".to_string());
                    } else if bracket > 0 || mark == b'-' || mark == b'C' || mark == b'F' {
                        return Err("Expression Error".to_string());
                    }

                    if let Sign::Char | Sign::Init = self.sign.clone().into_inner() {
                        num.borrow_mut().push(expr.extract(locat, index)?);
                        *self.sign.borrow_mut() = Sign::Data;
                    }

                    while ope.borrow().len() != 0 {
                        let value = ope.borrow_mut().pop().unwrap().computing(self)?;
                        num.borrow_mut().push(value);
                    }
                    return Ok(num.borrow_mut().pop().unwrap());
                }

                ch @ b'P' | ch @ b'E' | ch @ b'C' | ch @ b'L' => {
                    if let Sign::Char | Sign::Init = self.sign.clone().into_inner() {
                        let constant = match ch {
                            b'P' => &Constant::Pi,
                            b'E' => &Constant::Euler,
                            b'C' => &Constant::Catalan,
                            _ => &Constant::Log2
                        };

                        if mark != b'N' && mark != b'F' {
                            let value = if mark == b'-' {
                                0.0 - Float::with_val(128, constant)
                            } else {
                                Float::with_val(128, constant)
                            };
                            num.borrow_mut().push(value);
                            *self.sign.borrow_mut() = Sign::Data;
                            locat = index + 1;
                            mark = b'P';
                            continue;
                        }
                    }
                    return Err("Expression Error".to_string());
                }

                _ => return Err("Operator Undefined".to_string())
            }
        }
        Err("No Terminator".to_string())
    }

    pub fn run_round(&self, digits: Option<usize>) -> Result<String, String> {
        match self.run() {
            Ok(value) => Ok(value.to_round(digits)?),
            Err(err) => Err(err)
        }
    }
}