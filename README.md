# 高性能高级表达式计算器

### 使用Rust+Rug+Iced编写，支持(2560Bit)超大数运算。

### 计算器使用简介:

- 经过极致性能优化(实现零拷贝)
- 保证精度范围(小数后6位零误差)
- 符号`C`= 清空输入框表达式【快捷键】`Delete`
- 符号`◄`= 清除输入框一个字符【快捷键】`Backspace`
- 符号`%`= 求模运算符【快捷键】`Shift+5`
- 符号`π`= 圆周率常数【快捷键】`Shift+p`
- 符号`γ`= 欧拉-马歇罗尼常数【快捷键】`Shift+y`
- 符号`^`= 乘方运算符【快捷键】`Shift+6`
- 符号`()`= 括号运算符【快捷键】`Shift+9 or 0`或`[]`
- 历史列表= 清空历史记录【快捷键】`Ctrl+Delete`
- 函数`exp(1)`= 自然常数e的值
- 三角函数`Default: Radian`= 弧度`Radian`转角度`Degree`例:`cos(6xπ÷180)`
- 函数`sqrt(2)`= 平方根函数(开根号)
- 函数`fac(9)`= 阶乘函数

### 数学函数支持列表:

- `ai` , `abs` , `cos` , `sin` , `tan` , `csc` , `sec` , `cot` , `coth` , `ceil` , `floor`
- `cosh` , `sinh` , `tanh` , `sech` , `ln` , `csch` , `acos` , `asin` , `atan` , `frac` , `sgn`
- `acosh` , `asinh` , `atanh` , `log2` , `log10` , `sqrt` , `cbrt` , `fac` , `recip` , `erfc`
- `erf` , `li2` , `exp` , `exp2` ,`exp10` , `eint` , `zeta` , `trunc` , `gamma` , `digamma`

<img src="https://github.com/lhjok/Calculator/blob/main/assets/calc.png" width="717"/>

自创词法解析算法，一次遍历即完成计算，核心代码547行并包含错误检查机制。
