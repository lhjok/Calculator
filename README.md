# 高级表达式计算器

### 使用Rust+Rug+Iced编写，支持(2560Bit)超大数运算。

### 计算器使用简介:

- 保证精度范围(小数后6位零误差)
- 符号`C`= 清空输入框表达式
- 符号`◄`= 清除输入框一个字符
- 符号`%`= 求模运算符
- 符号`π`= 圆周率常数
- 符号`γ`= 欧拉-马歇罗尼常数
- 符号`^`= 乘方运算符
- 函数`exp(1)`= 自然常数e的值
- 三角函数`Default: Radian`= 弧度`Radian`转角度`Degree`例:`cos(6xπ÷180)`
- 函数`fac(9)`= 阶乘函数
- 函数`sqrt(2)`= 平方根函数(开根号)

### 支持的数学函数列表:

- `ai` , `abs` , `cos` , `sin` , `tan` , `csc` , `sec` , `cot` , `coth` , `ceil` , `floor`
- `cosh` , `sinh` , `tanh` , `sech` , `ln` , `csch` , `acos` , `asin` , `atan` , `frac` , `sgn`
- `acosh` , `asinh` , `atanh` , `log2` , `log10` , `sqrt` , `cbrt` , `fac` , `recip` , `erfc`
- `erf` , `li2` , `exp` , `exp2` ,`exp10` , `eint` , `zeta` , `trunc` , `gamma` , `digamma`

![image](https://github.com/lhjok/Calculator/blob/main/assets/calc.png)

自创的词法解析算法，一次遍历即完成计算。核心代码450行，包含错误检查机制且功能完整。
