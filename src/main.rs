use calc::Calc;
use once_cell::sync::Lazy;
use iced::executor::Default;

use iced::{
    window,
    alignment::{
        Vertical,
        Horizontal
    }
};

use iced::{
    Application, Command,
    Element, Settings,
    Alignment, Theme,
    Length, Font, Color
};

use iced::widget::{
    column, row,
    scrollable,
    button, text,
    vertical_space
};

use iced::widget::scrollable::{
    Id, Properties,
    RelativeOffset
};

static SCROLL: Lazy<Id> = Lazy::new(Id::unique);
const CONSOLA: Font = Font::External {
    name: "Consola",
    bytes: include_bytes!("../fonts/consolab.ttf")
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

    if valid.len() > 41 {
        valid[valid.len()-41..].concat()
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
            "C" => {
                self.value = String::from("0");
                self.show = String::from("0");
            },
            "\u{25C4}" => {
                if self.value.len() == 1 {
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

    fn new(_: Self::Flags) -> (Calculator, Command<Message>) {
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

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
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
                    let valid = if self.show[0..1].parse::<f64>().is_ok() {
                        self.value.clone()
                    } else { self.show.clone() };
                    let to_list = CalcResult {
                        result: Some((expr, valid))
                    };
                    self.history.push(to_list);
                    if self.history.len() >= 6 {
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

    fn view(&self) -> Element<Message> {
        // 显示单例计算结果
        let list_item = |d: &CalcResult| -> Element<Message> {
            column![
                vertical_space(2),
                text(format!("{}=", d.express()))
                    .size(19)
                    .width(Length::Fill)
                    .font(CONSOLA),
                vertical_space(2),
                text(format!("{}", d.result()))
                    .size(19)
                    .width(Length::Fill)
                    .font(CONSOLA)
                    .style(Color::from_rgb8(123, 104, 238))
            ].into()
        };

        // 显示历史记录列表
        let history_list = if self.history.len() != 0 {
            self.history.iter().fold(
                column![],
                |column, event| {
                    column.push(list_item(event))
                }
            )
        } else {
            column![
                text("No calculation history")
                    .size(19)
                    .font(CONSOLA)
                    .style(Color::from([0.35, 0.35, 0.35]))
            ].into()
        };

        // 显示输入输出结果
        let result_main = Element::from(
            column![
                text(self.show.clone())
                    .size(28)
                    .height(50)
                    .font(CONSOLA)
                    .horizontal_alignment(Horizontal::Right)
                    .vertical_alignment(Vertical::Center)
            ].width(Length::Fill)
             .height(50)
             .align_items(Alignment::End)
             .padding(11)
        );

        // 结果显示板块
        let display = Element::from(
            column![
                scrollable(
                    column![
                        history_list
                    ].width(Length::Fill)
                     .align_items(Alignment::Fill)
                     .padding([11, 11, 0, 11])
                ).height(255)
                 .vertical_scroll(
                     Properties::new()
                         .width(2)
                         .scroller_width(2)
                         .margin(0)
                 ).id(SCROLL.clone()),
                vertical_space(10),
                result_main
            ].width(Length::Fill)
        );

        // 数字按键板块
        let digit = |num: char| -> Element<Message> {
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

        // 运算符按键板块
        let oper_label = |op: char, lb: char, sz: u16| -> Element<Message> {
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

        // 运算符按键板块
        let operator = |op: char, sz: u16| -> Element<Message> {
            oper_label(op.clone(), op, sz)
        };

        // 数学函数按键板块
        let func_label = |fun: &str, lb: &str| -> Element<Message> {
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
                    func_label("Re", "re("), func_label("Erfc", "erfc("),
                    func_label("Sec", "sec("), func_label("Csc", "csc("),
                    func_label("Csch", "csch(")
                ].height(33).spacing(3),
                row![
                    func_label("Recip", "recip("), func_label("Erf", "erf("),
                    func_label("Acosh", "acosh("), func_label("Sgn", "sgn("),
                    func_label("Asinh", "asinh("), func_label("Frac", "frac("),
                    func_label("Atanh", "atanh("), func_label("Sech", "sech("),
                    func_label("Ceil", "ceil("), func_label("Floor", "floor(")
                ].height(35).spacing(3),
                row![
                    digit('7'), digit('8'), digit('9'),
                    operator('÷', 26), operator('\u{25C4}', 27),
                    operator('C', 24), func_label("Cos", "cos("),
                    func_label("Sin", "sin("), func_label("Tan", "tan("),
                    func_label("Acos", "acos(")
                ].height(Length::Fill).spacing(3),
                row![
                    digit('4'), digit('5'), digit('6'),
                    operator('×', 26), operator('(', 24),
                    operator(')', 24), func_label("Cosh", "cosh("),
                    func_label("Sinh", "sinh("), func_label("Tanh", "tanh("),
                    func_label("Atan", "atan(")
                ].height(Length::Fill).spacing(3),
                row![
                    digit('1'), digit('2'), digit('3'),
                    oper_label('−', '-', 26), operator('π', 24),
                    oper_label('\u{039B}', '^', 21), func_label("Sqrt", "sqrt("),
                    func_label("Log", "log("), func_label("Logx", "logx("),
                    func_label("Asin", "asin(")
                ].height(Length::Fill).spacing(3),
                row![
                    operator('%', 24), digit('0'), operator('.', 24),
                    operator('+', 26), operator('γ', 23),
                    operator('=', 25), func_label("Fac", "fac("),
                    func_label("Abs", "abs("), func_label("Ln", "ln("),
                    func_label("Exp", "exp(")
                ].height(Length::Fill).spacing(3),
            ].padding(3)
             .spacing(3)
        ].into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

pub fn main() -> iced::Result {
    Calculator::run(Settings{
        window: window::Settings {
            size: (635, 565),
            resizable: false,
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}
