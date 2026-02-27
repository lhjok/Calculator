use calc::Calculator;
use once_cell::sync::Lazy;
use iced::window::Position;
use textwrap::fill;

use iced::{
    keyboard::{
        self, key::{
            Key, Named,
            Physical, Code,
        }, Modifiers,
    },
    Background, Font, Padding,
    Subscription, Element, Theme,
    Task, Color, Alignment, Length,
    font::{ Weight, Family }, Pixels,
    event::{ self, Event, Status },
    window::{ self, icon, icon::Icon },
    window::settings::PlatformSpecific,
    alignment::{
        Horizontal,
        Vertical,
    }
};

use iced::widget::{
    column, row, rule, text,
    space, scrollable, Id,
    button, container,
    text::LineHeight, operation,
    container::Style,
    scrollable::{
        Scrollbar, Direction,
        RelativeOffset,
    }
};

#[derive(Clone)]
enum State {
    Set,
    None
}

#[derive(Clone)]
struct GCalculator {
    show: String,
    value: String,
    calc: Calculator,
    history: Vec<CalcResult>,
    scroll: RelativeOffset,
    state: State,
}

#[derive(Clone)]
struct CalcResult {
    result: Option<(String, String)>
}

#[derive(Debug, Clone)]
enum Message {
    Digit(String),
    Operator(String, String),
    Func(String)
}

fn trunc(lens: &str) -> String {
    let valid = lens
        .chars()
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>();

    if valid.len() > 45 {
        valid[valid.len()-45..].concat()
    } else {
        valid.concat()
    }
}

fn oper_repl(repl: &str) -> String {
    let len = repl.len();
    let mut result = String::with_capacity(len);
    for char in repl.chars() {
        match char {
            '÷' => result.push('/'),
            '×' => result.push('*'),
            'π' => result.push('P'),
            'γ' => result.push('Y'),
            _ => result.push(char),
        }
    }
    result
}

fn handle_key(
    key: Key, physical_key: Physical, modi: Modifiers,
) -> Option<Message> {
    let operator = |oper: String| -> Message {
        Message::Operator(oper.clone(), oper)
    };
    match (physical_key, key.clone()) {
        (Physical::Code(code), _) => match code {
            Code::Numpad0 => Some(Message::Digit(String::from("0"))),
            Code::Numpad1 => Some(Message::Digit(String::from("1"))),
            Code::Numpad2 => Some(Message::Digit(String::from("2"))),
            Code::Numpad3 => Some(Message::Digit(String::from("3"))),
            Code::Numpad4 => Some(Message::Digit(String::from("4"))),
            Code::Numpad5 => Some(Message::Digit(String::from("5"))),
            Code::Numpad6 => Some(Message::Digit(String::from("6"))),
            Code::Numpad7 => Some(Message::Digit(String::from("7"))),
            Code::Numpad8 => Some(Message::Digit(String::from("8"))),
            Code::Numpad9 => Some(Message::Digit(String::from("9"))),
            Code::NumpadDecimal => Some(operator(String::from("."))),
            _ => match key {
                Key::Named(Named::Delete) => if modi.control() {
                    Some(operator(String::from("D")))
                } else { Some(operator(String::from("C"))) },
                Key::Character(key) => { match key.as_str() {
                    "[" => Some(operator(String::from("("))),
                    "]" => Some(operator(String::from(")"))),
                    "+" => Some(operator(String::from("+"))),
                    "-" => Some(operator(String::from("-"))),
                    "*" => Some(operator(String::from("×"))),
                    "/" => Some(operator(String::from("÷"))),
                    "p" if modi.shift() => Some(operator(String::from("π"))),
                    "y" if modi.shift() => Some(operator(String::from("γ"))),
                    "0" => if modi.shift() {
                        Some(operator(String::from(")")))
                    } else { Some(Message::Digit(String::from("0"))) },
                    "1" => Some(Message::Digit(String::from("1"))),
                    "2" => Some(Message::Digit(String::from("2"))),
                    "3" => Some(Message::Digit(String::from("3"))),
                    "4" => Some(Message::Digit(String::from("4"))),
                    "5" => if modi.shift() {
                        Some(operator(String::from("%")))
                    } else { Some(Message::Digit(String::from("5"))) },
                    "6" => if modi.shift() {
                        Some(operator(String::from("^")))
                    } else { Some(Message::Digit(String::from("6"))) },
                    "7" => Some(Message::Digit(String::from("7"))),
                    "8" => if modi.shift() {
                        Some(operator(String::from("×")))
                    } else { Some(Message::Digit(String::from("8"))) },
                    "9" => if modi.shift() {
                        Some(operator(String::from("(")))
                    } else { Some(Message::Digit(String::from("9"))) },
                    "e" => Some(Message::Digit(String::from("e"))),
                    "." => Some(operator(String::from("."))),
                    "=" => if modi.shift() {
                        Some(operator(String::from("+")))
                    } else { Some(operator(String::from("="))) },
                    _ => None,
                }},
                Key::Named(Named::Enter) =>
                    Some(operator(String::from("="))),
                Key::Named(Named::Backspace) =>
                    Some(operator(String::from("\u{25C4}"))),
                _ => None,
            },
        },
        _ => None,
    }
}

static SCROLL: Lazy<Id> = Lazy::new(Id::unique);
const ICON: &[u8] = include_bytes!("../assets/calculator.png");

const CONSOLA_NORMAL: Font = Font {
    family: Family::Name("Consolas"),
    ..Font::DEFAULT
};

const CONSOLA_BOLD: Font = Font {
    family: Family::Name("Consolas"),
    weight: Weight::Bold,
    ..Font::DEFAULT
};

impl CalcResult {
    fn express(&self) -> String {
        let express = self.result.as_ref();
        express.map(|value| value.0.to_string())
            .unwrap_or_default()
    }

    fn result(&self) -> String {
        let result = self.result.as_ref();
        result.map(|value| value.1.to_string())
            .unwrap_or_default()
    }
}

impl Default for GCalculator {
    fn default() -> Self {
        GCalculator {
            show: String::from("0"),
            value: String::from("0"),
            calc: Calculator::new(),
            scroll: RelativeOffset::START,
            history: Vec::new(),
            state: State::None,
        }
    }
}

impl GCalculator {
    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Digit(num) => {
                self.func_digit_event(num);
                Task::none()
            }
            Message::Func(func) => {
                self.func_digit_event(func);
                Task::none()
            }
            Message::Operator(op, lb) => {
                let expr = self.value.clone();
                if self.oper_event(&op, lb) {
                    let valid = if self.show[0..1]
                        .parse::<f64>().is_ok() {
                            self.value.clone()
                        } else {
                            self.show.clone()
                        };
                    let to_list = CalcResult {
                        result: Some((expr, valid))
                    };
                    self.history.push(to_list);
                    if self.history.len() > 30 {
                        self.history.remove(0);
                    }
                    if self.history.len() >= 1 {
                        self.scroll = RelativeOffset::END;
                        return operation::snap_to(
                            SCROLL.clone(),
                            self.scroll
                        )
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let custom_rule: for<'a> fn(&'a _) -> _;
        custom_rule = |_: &Theme| -> rule::Style {
            rule::Style {
                snap: true,
                radius: 0.0.into(),
                color: Color::from([0.3, 0.3, 0.3]),
                fill_mode: rule::FillMode::Full,
            }
        };

        let custom_main: for<'a> fn(&'a _) -> _;
        custom_main = |_: &Theme| -> Style {
            let color = Color::from([0.2, 0.2, 0.2]);
            Style {
                background: Some(Background::Color(color)),
                ..Style::default()
            }
        };

        let list_item = |d: &CalcResult, i: usize|
            -> Element<Message> {
            column![
                if i == 0 { column![
                    space::vertical().height(5.0)
                ]} else { column![
                    rule::horizontal(1.0)
                        .style(custom_rule),
                    space::vertical().height(5.0)
                ]},
                text(format!("{}=", fill(&d.express(), 60)))
                    .size(21.0)
                    .width(Length::Fill)
                    .height(Length::Shrink)
                    .font(CONSOLA_NORMAL)
                    .line_height(LineHeight::Absolute(Pixels(23.0))),
                text(format!("{}", fill(&d.result(), 60)))
                    .size(21.0)
                    .width(Length::Fill)
                    .height(Length::Shrink)
                    .font(CONSOLA_BOLD)
                    .style(|_theme| text::Style {
                        color: Some(Color::from_rgb8(123, 104, 238)),
                    }).line_height(LineHeight::Absolute(Pixels(23.0))),
                space::vertical().height(3.0)
            ].into()
        };

        let history_list = if self.history.len() != 0 {
            self.history.iter().enumerate().fold(
                column![],
                |column, (index, event)| {
                    column.push(list_item(event, index))
                }
            )
        } else {
            column![
                space::vertical().height(2.0),
                text("No Calculation History")
                    .size(19.0)
                    .font(CONSOLA_BOLD)
                    .style(|_theme| text::Style {
                        color: Some(Color::from([0.35, 0.35, 0.35])),
                    })
            ].into()
        };

        let result_main = container(
            column![
                text(self.show.clone())
                    .size(28.0)
                    .height(Length::Shrink)
                    .font(CONSOLA_BOLD)
                    .align_x(Horizontal::Right)
                    .align_y(Vertical::Center)
            ].width(Length::Fill)
             .height(60.0)
             .align_x(Alignment::End)
             .padding(Padding {
                top: 17.0, right: 11.0,
                bottom: 11.0, left: 11.0,
            })
        ).width(Length::Fill)
         .style(custom_main);

        let display = Element::from(
            column![
                scrollable(
                    column![history_list]
                        .width(Length::Fill)
                        .align_x(Alignment::Start)
                        .padding(Padding {
                            top: 11.0, right: 11.0,
                            bottom: 0.0, left: 11.0,
                        })
                ).height(274.0)
                 .direction(Direction::Vertical(
                    Scrollbar::default()
                        .width(2.0)
                        .scroller_width(2.0)
                        .margin(0.0)
                 )).id(SCROLL.clone()),
                result_main
            ].width(Length::Fill)
        );

        let digit = |num: char| -> Element<Message> {
            let num = String::from(num);
            let digit = text(num.clone())
                .size(24.0)
                .font(CONSOLA_BOLD)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);
            let button = button(digit)
                .width(Length::Fill)
                .height(Length::Fill)
                .on_press(Message::Digit(num))
                .padding(Padding {
                    top: 4.0, right: 0.0,
                    bottom: 0.0, left: 0.0,
                });
            Element::from(button)
        };

        let oper_label = |op: char, lb: char, sz: f32, pd: f32|
            -> Element<Message> {
            let op = String::from(op);
            let lb = String::from(lb);
            let oper = text(op.clone())
                .size(sz)
                .font(CONSOLA_BOLD)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);
            let button = button(oper)
                .width(Length::Fill)
                .height(Length::Fill)
                .on_press(Message::Operator(op, lb))
                .padding(Padding {
                    top: pd, right: 0.0,
                    bottom: 0.0, left: 0.0,
                });
            Element::from(button)
        };

        let operator = |op: char, sz: f32, pd: f32|
            -> Element<Message> {
            oper_label(op.clone(), op, sz, pd)
        };

        let func_label = |fun: &'static str, lb: &'static str|
            -> Element<Message> {
            let lb = String::from(lb);
            let func = text(fun)
                .size(17.0)
                .font(CONSOLA_BOLD)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);
            let button = button(func)
                .width(Length::Fill)
                .height(Length::Fill)
                .on_press(Message::Func(lb))
                .padding(Padding {
                    top: 3.0, right: 0.0,
                    bottom: 0.0, left: 0.0,
                });
            Element::from(button)
        };

        column![
            display,
            column![
                row![
                    func_label("Cot", "cot("), func_label("Coth", "coth("),
                    func_label("Ai", "ai("), func_label("Cbrt", "cbrt("),
                    func_label("Li2", "li("), func_label("Erfc", "erfc("),
                    func_label("Sec", "sec("), func_label("Csc", "csc("),
                    func_label("Csch", "csch("),func_label("Eint", "eint("),
                    func_label("Trunc", "trunc("),
                ].height(36.0).spacing(3.0),
                row![
                    func_label("Recip", "recip("), func_label("Erf", "erf("),
                    func_label("Acosh", "acosh("), func_label("Sgn", "sgn("),
                    func_label("Asinh", "asinh("), func_label("Frac", "frac("),
                    func_label("Atanh", "atanh("), func_label("Sech", "sech("),
                    func_label("Ceil", "ceil("), func_label("Floor", "floor("),
                    func_label("Zeta", "zeta("),
                ].height(36.0).spacing(3.0),
                row![
                    digit('7'), digit('8'), digit('9'),
                    operator('÷', 26.0, 3.0), operator('\u{25C4}', 27.0, 3.0),
                    operator('C', 24.0, 3.0), func_label("Cos", "cos("),
                    func_label("Sin", "sin("), func_label("Tan", "tan("),
                    func_label("Acos", "acos("), func_label("Gamma", "gamma("),
                ].height(45.0).spacing(3.0),
                row![
                    digit('4'), digit('5'), digit('6'),
                    operator('×', 26.0, 3.0), operator('(', 24.0, 3.0),
                    operator(')', 24.0, 3.0), func_label("Cosh", "cosh("),
                    func_label("Sinh", "sinh("), func_label("Tanh", "tanh("),
                    func_label("Atan", "atan("), func_label("DiGam", "digamma("),
                ].height(45.0).spacing(3.0),
                row![
                    digit('1'), digit('2'), digit('3'),
                    oper_label('−', '-', 26.0, 3.0), operator('π', 24.0, 3.0),
                    oper_label('\u{039B}', '^', 21.0, 3.0), func_label("Sqrt", "sqrt("),
                    func_label("Log2", "log("), func_label("Log10", "logx("),
                    func_label("Asin", "asin("), func_label("Exp10", "expx("),
                ].height(45.0).spacing(3.0),
                row![
                    operator('%', 24.0, 5.0), digit('0'), operator('.', 24.0, 0.0),
                    operator('+', 26.0, 3.0), operator('γ', 23.0, 0.0),
                    operator('=', 25.0, 3.0), func_label("Fac", "fac("),
                    func_label("Abs", "abs("), func_label("Ln", "ln("),
                    func_label("Exp", "exp("), func_label("Exp2", "expt("),
                ].height(45.0).spacing(3.0),
            ].padding(3.0).spacing(3.0)
        ].into()
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, status, _id| {
            if let Status::Captured = status {
                return None;
            }
            match event {
                Event::Keyboard(
                    keyboard::Event::KeyPressed {
                        modifiers, key,
                        physical_key, ..
                    }
                ) => handle_key(key, physical_key, modifiers),
                _ => None
            }
        })
    }

    fn oper_event(&mut self, op: &str, label: String) -> bool {
        match op {
            "D" => self.history = Vec::new(),
            "C" => {
                self.value = String::from("0");
                self.show = String::from("0");
            },
            "\u{25C4}" => {
                if self.value.len() == 1 ||
                    self.value == "π" || self.value == "γ" {
                    self.value = String::from("0");
                    self.show = String::from("0");
                } else {
                    self.value.pop();
                    self.show = trunc(self.value.as_str());
                }
            },
            "=" => {
                self.state = State::Set;
                if self.value != "0" {
                    let expr = oper_repl(self.value.as_str());
                    match self.calc.run_round(expr, Some(6)) {
                        Ok(valid) => {
                            self.value = valid.clone();
                            self.show = trunc(valid.as_str())
                        },
                        Err(msg) => {
                            self.calc.reset();
                            self.value = String::from("0");
                            self.show = msg.to_string()
                        }
                    }
                    return true;
                }
            },
            "." => {
                if let State::Set = self.state {
                    self.value = String::from("0");
                    self.show = String::from("0");
                    self.state = State::None;
                } else {
                    self.value += &label;
                    self.show = trunc(self.value.as_str());
                }
            },
            ch @ "(" | ch @ "−" | ch @ "π" | ch @ "γ" | ch @ "-" => {
                if let State::Set = self.state {
                    self.state = State::None;
                    if (ch == "−" || ch == "-") && self.value != "0" {
                        self.value += &label;
                        self.show = trunc(self.value.as_str());
                    } else {
                        self.value = label.clone();
                        self.show = label.clone();
                    }
                } else if self.value == "0" {
                    self.value = label.clone();
                    self.show = label.clone();
                } else {
                    self.value += &label;
                    self.show = trunc(self.value.as_str());
                }
            },
            _ => {
                if let State::Set = self.state {
                    self.value += &label;
                    self.show = trunc(self.value.as_str());
                    self.state = State::None;
                } else {
                    self.value += &label;
                    self.show = trunc(self.value.as_str());
                }
            }
        }
        false
    }

    fn func_digit_event(&mut self, label: String) {
        if let State::Set = self.state {
            self.value = label.clone();
            self.show = label.clone();
            self.state = State::None;
        } else if self.value == "0" {
            self.value = label.clone();
            self.show = label.clone();
        } else {
            self.value += &label;
            self.show = trunc(self.value.as_str());
        }
    }
}

fn load_icon_strictly() -> Option<Icon> {
    let img = image::load_from_memory(ICON).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    icon::from_rgba(rgba.into_raw(), width, height).ok()
}

pub fn main() -> iced::Result {
    iced::application(
        GCalculator::default,
        GCalculator::update,
        GCalculator::view,
    ).window(window::Settings {
        size: [715.0, 607.0].into(),
        position: Position::Centered,
        resizable: false,
        icon: load_icon_strictly(),
        platform_specific: PlatformSpecific {
            application_id: String::from("calc"),
            ..Default::default()
        }, ..window::Settings::default()
    }).antialiasing(true)
    .title(|_: &GCalculator| String::from("Advanced Calculator"))
    .theme(|_: &GCalculator| Theme::Dark)
    .subscription(GCalculator::subscription)
    .run()
}