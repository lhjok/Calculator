use calc::Calc;
use once_cell::sync::Lazy;
use iced::executor::Default;
use textwrap::fill;

use iced::{
    keyboard::{
        self, KeyCode,
        Modifiers
    },
    subscription,
    Background, Font, Settings,
    Subscription, Element, Theme,
    Application, Command, Color,
    Alignment, Length,
    event::{ Event, Status },
    theme::{ self, Container },
    window::{ self, icon },
    alignment::{
        Horizontal,
        Vertical
    }
};

use iced::widget::{
    column, row, rule, text,
    vertical_space, scrollable,
    button, container, Rule,
    container::Appearance,
    scrollable::{
        Id, Properties,
        RelativeOffset
    }
};

#[derive(Clone)]
enum State {
    Set,
    None
}

#[derive(Clone)]
struct Calculator {
    show: String,
    value: String,
    history: Vec<CalcResult>,
    state: State,
    scroll: RelativeOffset
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

fn trunc(lens: String) -> String {
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

fn oper_repl(repl: String) -> String {
    repl.replace("÷", "/")
        .replace("×", "*")
        .replace("π", "P")
        .replace("γ", "E")
}

fn handle_key(key: KeyCode, modi: Modifiers)
    -> Option<Message> {
    let operator = |oper: String| -> Message {
        Message::Operator(oper.clone(), oper)
    };

    match key {
        KeyCode::P if modi.control() =>
            Some(operator(String::from("π"))),
        KeyCode::Y if modi.control() =>
            Some(operator(String::from("γ"))),
        KeyCode::Delete => if modi.control() {
            Some(operator(String::from("D")))
        } else { Some(operator(String::from("C"))) },
        KeyCode::LBracket =>
            Some(operator(String::from("("))),
        KeyCode::RBracket =>
            Some(operator(String::from(")"))),
        KeyCode::Plus | KeyCode::NumpadAdd =>
            Some(operator(String::from("+"))),
        KeyCode::Minus | KeyCode::NumpadSubtract =>
            Some(operator(String::from("-"))),
        KeyCode::Asterisk | KeyCode::NumpadMultiply =>
            Some(operator(String::from("×"))),
        KeyCode::Slash | KeyCode::NumpadDivide =>
            Some(operator(String::from("÷"))),
        KeyCode::Key0 | KeyCode::Numpad0 =>
            if modi.control() {
                Some(Message::Digit(String::from(")")))
            } else { Some(Message::Digit(String::from("0"))) },
        KeyCode::Key1 | KeyCode::Numpad1 =>
            Some(Message::Digit(String::from("1"))),
        KeyCode::Key2 | KeyCode::Numpad2 =>
            Some(Message::Digit(String::from("2"))),
        KeyCode::Key3 | KeyCode::Numpad3 =>
            Some(Message::Digit(String::from("3"))),
        KeyCode::Key4 | KeyCode::Numpad4 =>
            Some(Message::Digit(String::from("4"))),
        KeyCode::Key5 | KeyCode::Numpad5 =>
            if modi.control() {
                Some(Message::Digit(String::from("%")))
            } else { Some(Message::Digit(String::from("5"))) },
        KeyCode::Key6 | KeyCode::Numpad6 =>
            if modi.control() {
                Some(Message::Digit(String::from("^")))
            } else { Some(Message::Digit(String::from("6"))) },
        KeyCode::Key7 | KeyCode::Numpad7 =>
            Some(Message::Digit(String::from("7"))),
        KeyCode::Key8 | KeyCode::Numpad8 =>
            Some(Message::Digit(String::from("8"))),
        KeyCode::Key9 | KeyCode::Numpad9 =>
            if modi.control() {
                Some(Message::Digit(String::from("(")))
            } else { Some(Message::Digit(String::from("9"))) },
        KeyCode::Period | KeyCode::NumpadDecimal =>
            Some(Message::Digit(String::from("."))),
        KeyCode::Equals | KeyCode::NumpadEquals =>
            Some(operator(String::from("="))),
        KeyCode::Enter | KeyCode::NumpadEnter =>
            Some(operator(String::from("="))),
        KeyCode::Backspace =>
            Some(operator(String::from("\u{25C4}"))),
        _ => None
    }
}

static SCROLL: Lazy<Id> = Lazy::new(Id::unique);
const ICON: &[u8] = include_bytes!("../assets/calculator.png");
const CONSOLA: Font = Font::External {
    name: "Consola",
    bytes: include_bytes!("../fonts/consolab.ttf")
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

impl Calculator {
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
                    self.show = trunc(self.value.clone());
                }
            },
            "=" => {
                self.state = State::Set;
                let expr = oper_repl(self.value.clone());
                if self.value != "0" {
                    match Calc::new(expr).run_round(Some(7)) {
                        Ok(valid) => {
                            self.value = valid.clone();
                            self.show = trunc(valid)
                        },
                        Err(msg) => {
                            self.value = String::from("0");
                            self.show = msg
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
                    self.show = trunc(self.value.clone());
                }
            },
            ch @ "(" | ch @ "−" | ch @ "π" | ch @ "γ" => {
                if let State::Set = self.state {
                    self.state = State::None;
                    if ch == "−" && self.value != "0" {
                        self.value += &label;
                        self.show = trunc(self.value.clone());
                    } else {
                        self.value = label.clone();
                        self.show = label.clone();
                    }
                } else if self.value == "0" {
                    self.value = label.clone();
                    self.show = label.clone();
                } else {
                    self.value += &label;
                    self.show = trunc(self.value.clone());
                }
            },
            _ => {
                if let State::Set = self.state {
                    self.value += &label;
                    self.show = trunc(self.value.clone());
                    self.state = State::None;
                } else {
                    self.value += &label;
                    self.show = trunc(self.value.clone());
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
            self.show = trunc(self.value.clone());
        }
    }
}

impl Application for Calculator {
    type Flags = ();
    type Message = Message;
    type Executor = Default;
    type Theme = Theme;

    fn new(_: Self::Flags) ->
        (Calculator, Command<Self::Message>) {
        (Calculator {
            show: String::from("0"),
            value: String::from("0"),
            history: Vec::new(),
            state: State::None,
            scroll: RelativeOffset::START
        }, Command::none())
    }

    fn title(&self) -> String {
        String::from("Senior Calculator")
    }

    fn update(&mut self, msg: Self::Message) 
        -> Command<Self::Message> {
        match msg {
            Message::Digit(num) => {
                self.func_digit_event(num);
                Command::none()
            }
            Message::Func(func) => {
                self.func_digit_event(func);
                Command::none()
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
                    if self.history.len() >= 1 {
                        self.scroll = RelativeOffset::END;
                        return scrollable::snap_to(
                            SCROLL.clone(),
                            self.scroll
                        )
                    }
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let custom_rule: for<'a> fn(&'a _) -> _;
        custom_rule = |_: &Theme| -> rule::Appearance {
            rule::Appearance {
                width: 1,
                radius: 0.0,
                color: Color::from([0.3, 0.3, 0.3]),
                fill_mode: rule::FillMode::Full,
            }
        };

        let custom_main: for<'a> fn(&'a _) -> _;
        custom_main = |_: &Theme| -> Appearance {
            let color = Color::from([0.2, 0.2, 0.2]);
            Appearance {
                background: Some(Background::Color(color)),
                ..Appearance::default()
            }
        };

        let list_item = |d: &CalcResult, i: usize|
            -> Element<Self::Message> {
            column![
                if i == 0 { column![
                    vertical_space(5)
                ]} else { column![
                    Rule::horizontal(1)
                        .style(theme::Rule::from(custom_rule)),
                    vertical_space(5)
                ]},
                text(format!("{}=", fill(&d.express(), 63)))
                    .size(20)
                    .width(Length::Fill)
                    .height(Length::Shrink)
                    .font(CONSOLA),
                vertical_space(3),
                text(format!("{}", fill(&d.result(), 63)))
                    .size(20)
                    .width(Length::Fill)
                    .height(Length::Shrink)
                    .font(CONSOLA)
                    .style(Color::from_rgb8(123, 104, 238)),
                vertical_space(3)
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
                vertical_space(2),
                text("No calculation history")
                    .size(19)
                    .font(CONSOLA)
                    .style(Color::from([0.35, 0.35, 0.35]))
            ].into()
        };

        let result_main = container(
            column![
                vertical_space(8),
                text(self.show.clone())
                    .size(28)
                    .height(52)
                    .font(CONSOLA)
                    .horizontal_alignment(Horizontal::Right)
                    .vertical_alignment(Vertical::Center)
            ].width(Length::Fill)
             .height(60)
             .align_items(Alignment::End)
             .padding(11)
        ).width(Length::Fill)
         .style(Container::from(custom_main));

        let display = Element::from(
            column![
                scrollable(
                    column![history_list]
                        .width(Length::Fill)
                        .align_items(Alignment::Start)
                        .padding([11, 11, 0, 11])
                ).height(260)
                 .vertical_scroll(
                     Properties::new()
                         .width(2)
                         .scroller_width(2)
                         .margin(0)
                 ).id(SCROLL.clone()),
                result_main
            ].width(Length::Fill)
        );

        let digit = |num: char| -> Element<Self::Message> {
            let num = String::from(num);
            let digit = text(num.clone())
                .size(24)
                .font(CONSOLA)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center);
            let button = button(digit)
                .width(Length::Fill)
                .on_press(Message::Digit(num));
            Element::from(button)
        };

        let oper_label = |op: char, lb: char, sz: u16|
            -> Element<Self::Message> {
            let op = String::from(op);
            let lb = String::from(lb);
            let oper = text(op.clone())
                .size(sz)
                .font(CONSOLA)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center);
            let button = button(oper)
                .width(Length::Fill)
                .on_press(Message::Operator(op, lb));
            Element::from(button)
        };

        let operator = |op: char, sz: u16|
            -> Element<Self::Message> {
            oper_label(op.clone(), op, sz)
        };

        let func_label = |fun: &str, lb: &str|
            -> Element<Self::Message> {
            let lb = String::from(lb);
            let func = text(fun)
                .size(17)
                .font(CONSOLA)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center);
            let button = button(func)
                .width(Length::Fill)
                .on_press(Message::Func(lb));
            Element::from(button)
        };

        column![
            display,
            column![
                row![
                    func_label("Cot", "cot("), func_label("Coth", "coth"),
                    func_label("Ai", "ai("), func_label("Cbrt", "cbrt("),
                    func_label("Li2", "li("), func_label("Erfc", "erfc("),
                    func_label("Sec", "sec("), func_label("Csc", "csc("),
                    func_label("Csch", "csch("),func_label("Eint", "eint("),
                    func_label("Trunc", "trunc("),
                ].height(33).spacing(3),
                row![
                    func_label("Recip", "recip("), func_label("Erf", "erf("),
                    func_label("Acosh", "acosh("), func_label("Sgn", "sgn("),
                    func_label("Asinh", "asinh("), func_label("Frac", "frac("),
                    func_label("Atanh", "atanh("), func_label("Sech", "sech("),
                    func_label("Ceil", "ceil("), func_label("Floor", "floor("),
                    func_label("Zeta", "zeta("),
                ].height(35).spacing(3),
                row![
                    digit('7'), digit('8'), digit('9'),
                    operator('÷', 26), operator('\u{25C4}', 27),
                    operator('C', 24), func_label("Cos", "cos("),
                    func_label("Sin", "sin("), func_label("Tan", "tan("),
                    func_label("Acos", "acos("), func_label("Gamma", "gamma("),
                ].height(Length::Fill).spacing(3),
                row![
                    digit('4'), digit('5'), digit('6'),
                    operator('×', 26), operator('(', 24),
                    operator(')', 24), func_label("Cosh", "cosh("),
                    func_label("Sinh", "sinh("), func_label("Tanh", "tanh("),
                    func_label("Atan", "atan("), func_label("DiGam", "digamma("),
                ].height(Length::Fill).spacing(3),
                row![
                    digit('1'), digit('2'), digit('3'),
                    oper_label('−', '-', 26), operator('π', 24),
                    oper_label('\u{039B}', '^', 21), func_label("Sqrt", "sqrt("),
                    func_label("Log2", "log("), func_label("Log10", "logx("),
                    func_label("Asin", "asin("), func_label("Exp10", "expx("),
                ].height(Length::Fill).spacing(3),
                row![
                    operator('%', 24), digit('0'), operator('.', 24),
                    operator('+', 26), operator('γ', 23),
                    operator('=', 25), func_label("Fac", "fac("),
                    func_label("Abs", "abs("), func_label("Ln", "ln("),
                    func_label("Exp", "exp("), func_label("Exp2", "expt("),
                ].height(Length::Fill).spacing(3),
            ].padding(3)
             .spacing(3)
        ].into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        subscription::events_with(|event, status| {
            if let Status::Captured = status {
                return None;
            }
            match event {
                Event::Keyboard(
                    keyboard::Event::KeyPressed {
                        modifiers,
                        key_code
                    }
                ) => handle_key(key_code, modifiers),
                _ => None
            }
        })
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

pub fn main() -> iced::Result {
    Calculator::run(Settings{
        id: Some("Calculator".to_string()),
        window: window::Settings {
            max_size: Some((715, 582)),
            resizable: false,
            icon: Some(icon::from_file_data(
                ICON, None
            ).expect("Failed to load icon")),
            ..window::Settings::default()
        },
        antialiasing: true,
        ..Settings::default()
    })
}